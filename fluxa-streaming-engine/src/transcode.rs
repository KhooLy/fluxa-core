use axum::body::Body;
use axum::extract::Query;
use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use serde::Deserialize;
use std::process::Stdio;
use tokio::process::Command;
use tokio_util::io::ReaderStream;

use crate::ffmpeg_locator;

#[derive(Deserialize)]
pub struct TranscodeQuery {
    url: String,
    /// Input-side seek in seconds. The transcode itself isn't byte-range
    /// seekable (it's a live ffmpeg pipe), so the player seeks by re-opening
    /// this endpoint with a new `start` instead.
    start: Option<f64>,
}

/// Blocks ffmpeg url schemes like `file:` and SSRF to non-loopback hosts.
fn is_allowed_stream_url(raw: &str) -> bool {
    let Ok(parsed) = url::Url::parse(raw) else { return false };
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return false;
    }
    matches!(parsed.host_str(), Some("127.0.0.1") | Some("localhost") | Some("[::1]") | Some("::1"))
}

#[derive(Default)]
struct ProbedCodecs {
    video: Option<String>,
    audio: Option<String>,
    duration: Option<f64>,
}

async fn probe(url: &str) -> ProbedCodecs {
    let ffprobe = ffmpeg_locator::resolve("ffprobe");
    let output = Command::new(ffprobe)
        .args([
            "-v", "error",
            "-print_format", "json",
            "-show_entries", "stream=codec_type,codec_name:format=duration",
        ])
        .arg(url)
        .stdin(Stdio::null())
        .output()
        .await;

    let Ok(output) = output else { return ProbedCodecs::default() };
    let Ok(json) = serde_json::from_slice::<serde_json::Value>(&output.stdout) else {
        return ProbedCodecs::default();
    };
    let mut codecs = ProbedCodecs::default();
    if let Some(streams) = json.get("streams").and_then(|s| s.as_array()) {
        for stream in streams {
            let kind = stream.get("codec_type").and_then(|v| v.as_str());
            let name = stream.get("codec_name").and_then(|v| v.as_str()).map(str::to_string);
            match kind {
                Some("video") if codecs.video.is_none() => codecs.video = name,
                Some("audio") if codecs.audio.is_none() => codecs.audio = name,
                _ => {}
            }
        }
    }
    codecs.duration = json
        .get("format")
        .and_then(|f| f.get("duration"))
        .and_then(|d| d.as_str())
        .and_then(|d| d.parse::<f64>().ok());
    codecs
}

#[derive(Deserialize)]
pub struct ProbeQuery {
    url: String,
}

pub async fn handle_probe(Query(q): Query<ProbeQuery>) -> Response {
    if !is_allowed_stream_url(&q.url) {
        return (StatusCode::BAD_REQUEST, "url must be http(s)://127.0.0.1 or localhost").into_response();
    }
    let codecs = probe(&q.url).await;
    axum::Json(serde_json::json!({
        "videoCodec": codecs.video,
        "audioCodec": codecs.audio,
        "duration": codecs.duration,
    }))
    .into_response()
}

/// Remuxes (stream-copy) when the source codecs are already browser-playable,
/// and falls back to a real transcode only for the tracks that aren't —
/// most addon releases are h264+aac in an mkv container, so this is a cheap
/// container rewrite rather than a full re-encode in the common case.
pub async fn handle_transcode(Query(q): Query<TranscodeQuery>) -> Response {
    if !is_allowed_stream_url(&q.url) {
        return (StatusCode::BAD_REQUEST, "url must be http(s)://127.0.0.1 or localhost").into_response();
    }
    let codecs = probe(&q.url).await;

    let video_args: &[&str] = match codecs.video.as_deref() {
        Some("h264") => &["-c:v", "copy"],
        _ => &["-c:v", "libx264", "-preset", "veryfast", "-crf", "20"],
    };
    let audio_args: &[&str] = match codecs.audio.as_deref() {
        Some("aac") => &["-c:a", "copy"],
        _ => &["-c:a", "aac", "-b:a", "192k"],
    };

    let ffmpeg = ffmpeg_locator::resolve("ffmpeg");
    let mut cmd = Command::new(ffmpeg);
    if let Some(start) = q.start.filter(|s| *s > 0.0) {
        cmd.args(["-ss", &start.to_string()]);
    }
    cmd.arg("-i").arg(&q.url)
        .args(video_args)
        .args(audio_args)
        .args(["-sn", "-movflags", "frag_keyframe+empty_moov+default_base_moof", "-f", "mp4", "-"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("failed to start ffmpeg: {e}")).into_response(),
    };
    let Some(stdout) = child.stdout.take() else {
        return (StatusCode::INTERNAL_SERVER_ERROR, "ffmpeg produced no stdout pipe").into_response();
    };

    // Detached: the child is reaped once stdout closes (process exits) or the
    // response stream is dropped (client disconnects), whichever comes first.
    tokio::spawn(async move {
        let _ = child.wait().await;
    });

    let body = Body::from_stream(ReaderStream::with_capacity(stdout, 65536));
    let mut response = (StatusCode::OK, body).into_response();
    response.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        HeaderValue::from_static("video/mp4"),
    );
    response
}

pub fn router() -> Router {
    Router::new()
        .route("/transcode", get(handle_transcode))
        .route("/probe", get(handle_probe))
}
