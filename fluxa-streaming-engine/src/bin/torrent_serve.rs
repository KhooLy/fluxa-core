/// rqbit HTTP streaming server — magnet → HTTP stream URL
/// Starts a local HTTP server, adds the torrent, prints:
///   READY <url>  (once the stream is serveable)
/// then keeps running until killed.
///
/// Usage: torrent_serve "<magnet>" [--file-idx N]

use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use std::io::Write;
use librqbit::{
    AddTorrent, AddTorrentOptions, Api, PeerConnectionOptions, Session, SessionOptions,
};
use std::collections::HashSet;
use std::io::SeekFrom;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::AsyncSeekExt;
use tokio::net::TcpListener;
use tokio_util::io::ReaderStream;

#[derive(Clone)]
struct Ctx {
    api: Api,
    torrent_id: usize,
    file_id: usize,
    file_name: String,
    file_len: u64,
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let magnet = args
        .get(1)
        .expect("Usage: torrent_serve <magnet> [--file-idx N]");
    let file_idx_override: Option<usize> = args
        .windows(2)
        .find(|w| w[0] == "--file-idx")
        .and_then(|w| w[1].parse().ok());

    let cache = std::env::temp_dir().join("fluxa_bench_serve");
    let _ = std::fs::remove_dir_all(&cache);
    std::fs::create_dir_all(&cache).unwrap();

    eprintln!("[serve] starting rqbit session…");
    let t0 = Instant::now();

    let mut session_opts = SessionOptions::default();
    session_opts.disable_dht_persistence = true;
    session_opts.defer_writes_up_to = Some(64);
    session_opts.listen_port_range = Some(49152..65535);
    session_opts.disable_upload = true;

    let session = Session::new_with_opts(cache.clone(), session_opts)
        .await
        .expect("session");
    let api = Api::new(session, None);
    eprintln!("[serve] engine up in {}ms", t0.elapsed().as_millis());

    eprintln!("[serve] adding torrent (waiting for metadata)…");
    let t_meta = Instant::now();

    let mut add_opts = AddTorrentOptions::default();
    add_opts.overwrite = true;
    add_opts.output_folder = Some(cache.to_string_lossy().into_owned());
    add_opts.peer_opts = Some(PeerConnectionOptions {
        connect_timeout: Some(Duration::from_secs(10)),
        read_write_timeout: Some(Duration::from_secs(20)),
        ..Default::default()
    });

    let response = api
        .api_add_torrent(AddTorrent::Url(magnet.to_string().into()), Some(add_opts))
        .await
        .expect("add_torrent failed");

    let torrent_id = response.id.expect("torrent id");
    eprintln!("[serve] metadata in {}ms", t_meta.elapsed().as_millis());

    let files = response.details.files.as_ref().expect("files");
    let (file_id, file_name, file_len) = if let Some(idx) = file_idx_override {
        let f = &files[idx];
        (idx, f.name.clone(), f.length)
    } else {
        files
            .iter()
            .enumerate()
            .filter(|(_, f)| {
                matches!(
                    f.name.to_ascii_lowercase().rsplit('.').next().unwrap_or(""),
                    "mkv" | "mp4" | "avi" | "webm" | "m4v" | "mov"
                )
            })
            .max_by_key(|(_, f)| f.length)
            .or_else(|| files.iter().enumerate().max_by_key(|(_, f)| f.length))
            .map(|(i, f)| (i, f.name.clone(), f.length))
            .expect("at least one file")
    };

    eprintln!(
        "[serve] target file: [{file_id}] {file_name} ({} MB)",
        file_len / 1024 / 1024
    );

    // prioritize only this file
    let only = HashSet::from([file_id]);
    let _ = api
        .api_torrent_action_update_only_files(
            librqbit::api::TorrentIdOrHash::Id(torrent_id),
            &only,
        )
        .await;

    let ctx = Arc::new(Ctx {
        api,
        torrent_id,
        file_id,
        file_name,
        file_len,
    });

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let stream_url = format!("http://127.0.0.1:{port}/stream");

    let app = Router::new()
        .route("/stream", get(handle_stream))
        .with_state(ctx);

    // Signal ready BEFORE accepting connections — explicit flush required when stdout is a file
    println!("READY {stream_url}");
    let _ = std::io::stdout().flush();
    eprintln!("[serve] listening on {stream_url}");

    axum::serve(listener, app).await.unwrap();
}

async fn handle_stream(
    State(ctx): State<Arc<Ctx>>,
    headers: HeaderMap,
) -> Response {
    // retry until rqbit transitions to Live
    let mut stream = loop {
        match ctx
            .api
            .api_stream(librqbit::api::TorrentIdOrHash::Id(ctx.torrent_id), ctx.file_id)
        {
            Ok(s) => break s,
            Err(_) => tokio::time::sleep(Duration::from_millis(100)).await,
        }
    };

    let total = ctx.file_len;
    let range_str = headers
        .get("Range")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let (status, start, length) = if let Some((s, e)) = parse_range(range_str, total) {
        let end = e.unwrap_or(total - 1);
        let _ = stream.seek(SeekFrom::Start(s)).await;
        (StatusCode::PARTIAL_CONTENT, s, end - s + 1)
    } else {
        (StatusCode::OK, 0u64, total)
    };

    let ext = ctx.file_name.rsplit('.').next().unwrap_or("mp4");
    let mime = match ext {
        "mkv" => "video/x-matroska",
        "webm" => "video/webm",
        "mp4" | "m4v" => "video/mp4",
        "avi" => "video/x-msvideo",
        _ => "application/octet-stream",
    };

    let mut resp_headers = HeaderMap::new();
    resp_headers.insert("Accept-Ranges", HeaderValue::from_static("bytes"));
    resp_headers.insert("Content-Type", HeaderValue::from_str(mime).unwrap());
    resp_headers.insert(
        "Content-Length",
        HeaderValue::from_str(&length.to_string()).unwrap(),
    );
    if status == StatusCode::PARTIAL_CONTENT {
        let end = start + length - 1;
        resp_headers.insert(
            "Content-Range",
            HeaderValue::from_str(&format!("bytes {start}-{end}/{total}")).unwrap(),
        );
    }

    let body = Body::from_stream(ReaderStream::with_capacity(stream, 65536));
    (status, resp_headers, body).into_response()
}

fn parse_range(value: &str, length: u64) -> Option<(u64, Option<u64>)> {
    let raw = value.strip_prefix("bytes=")?;
    let (start, end) = raw.split_once('-')?;
    let start = start.parse::<u64>().ok()?;
    if start >= length {
        return None;
    }
    let end = end.parse::<u64>().ok().map(|e| e.min(length - 1));
    Some((start, end))
}
