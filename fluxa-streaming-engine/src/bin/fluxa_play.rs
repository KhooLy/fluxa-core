/// fluxa-play — Cinemeta + Torrentio + rqbit → mpv
///
/// Kullanım: fluxa_play

use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use librqbit::{
    AddTorrent, AddTorrentOptions, Api, PeerConnectionOptions, Session, SessionOptions,
};
use serde_json::Value;
use std::{
    collections::HashSet,
    io::{self, BufRead, SeekFrom, Write},
    sync::Arc,
    time::Duration,
};
use tokio::io::AsyncSeekExt;
use tokio::net::TcpListener;
use tokio_util::io::ReaderStream;

const CINEMETA: &str = "https://v3-cinemeta.strem.io";
const TORRENTIO: &str = "https://torrentio.strem.fun";
const TRACKERS: &str =
    "tr=udp%3A%2F%2Ftracker.opentrackr.org%3A1337%2Fannounce\
     &tr=udp%3A%2F%2Fopen.stealth.si%3A80%2Fannounce";

// ── helpers ───────────────────────────────────────────────────────────────────

fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn http_get(url: &str) -> Option<Value> {
    let text = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("Mozilla/5.0")
        .build()
        .ok()?
        .get(url)
        .send()
        .ok()?
        .text()
        .ok()?;
    serde_json::from_str(&text).ok()
}

fn readline(prompt: &str) -> String {
    print!("{prompt}");
    io::stdout().flush().ok();
    let mut line = String::new();
    io::stdin().lock().read_line(&mut line).ok();
    line.trim().to_string()
}

fn pick_idx(max: usize) -> Option<usize> {
    let s = readline("> Seçim (q = çık): ");
    if s == "q" {
        return None;
    }
    let n: usize = s.parse().ok()?;
    if n == 0 || n > max {
        eprintln!("  Geçersiz seçim.");
        None
    } else {
        Some(n - 1)
    }
}

// ── Cinemeta ──────────────────────────────────────────────────────────────────

struct Meta {
    id: String,
    kind: &'static str,
    name: String,
    year: String,
}

fn cinemeta_search(query: &str) -> Vec<Meta> {
    let enc = percent_encode(query);
    let mut out = Vec::new();
    for kind in &[("movie", "🎬"), ("series", "📺")] {
        let url = format!("{CINEMETA}/catalog/{}/top/search={enc}.json", kind.0);
        if let Some(j) = http_get(&url) {
            for m in j["metas"].as_array().into_iter().flatten().take(5) {
                let year = m["year"].as_str().map(|y| format!(" ({y})")).unwrap_or_default();
                out.push(Meta {
                    id: m["id"].as_str().unwrap_or("").to_string(),
                    kind: kind.0,
                    name: format!("{} {}", kind.1, m["name"].as_str().unwrap_or("?")),
                    year,
                });
            }
        }
    }
    out
}

// ── Torrentio ─────────────────────────────────────────────────────────────────

struct Stream {
    label: String,
    info_hash: String,
    file_idx: Option<usize>,
}

fn torrentio_streams(kind: &str, id: &str) -> Vec<Stream> {
    let url = format!("{TORRENTIO}/stream/{kind}/{id}.json");
    let j = match http_get(&url) {
        Some(v) => v,
        None => {
            eprintln!("  Torrentio'ya ulaşılamadı.");
            return vec![];
        }
    };
    j["streams"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|s| {
            let info_hash = s["infoHash"].as_str()?.to_string();
            let lines: Vec<&str> = s["title"].as_str().unwrap_or("").lines().collect();
            let label = lines.into_iter().take(2).collect::<Vec<_>>().join("  ");
            let file_idx = s["fileIdx"].as_u64().map(|n| n as usize);
            Some(Stream { label, info_hash, file_idx })
        })
        .take(12)
        .collect()
}

// ── rqbit HTTP sunucusu ───────────────────────────────────────────────────────

#[derive(Clone)]
struct Ctx {
    api: Api,
    torrent_id: usize,
    file_id: usize,
    file_name: String,
    file_len: u64,
}

async fn handle_stream(State(ctx): State<Arc<Ctx>>, headers: HeaderMap) -> Response {
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
        "avi" => "video/x-msvideo",
        _ => "video/mp4",
    };

    let mut h = HeaderMap::new();
    h.insert("Accept-Ranges", HeaderValue::from_static("bytes"));
    h.insert("Content-Type", HeaderValue::from_str(mime).unwrap());
    h.insert("Content-Length", HeaderValue::from_str(&length.to_string()).unwrap());
    if status == StatusCode::PARTIAL_CONTENT {
        h.insert(
            "Content-Range",
            HeaderValue::from_str(&format!("bytes {start}-{}-{total}", start + length - 1))
                .unwrap(),
        );
    }

    (status, h, Body::from_stream(ReaderStream::with_capacity(stream, 65536))).into_response()
}

fn parse_range(value: &str, length: u64) -> Option<(u64, Option<u64>)> {
    let raw = value.strip_prefix("bytes=")?;
    let (s, e) = raw.split_once('-')?;
    let start = s.parse::<u64>().ok()?;
    if start >= length {
        return None;
    }
    let end = e.parse::<u64>().ok().map(|v| v.min(length - 1));
    Some((start, end))
}

async fn start_server(magnet: String, file_idx: Option<usize>) -> Result<String, String> {
    let cache = std::env::temp_dir().join("fluxa_play");
    let _ = std::fs::remove_dir_all(&cache);
    std::fs::create_dir_all(&cache).map_err(|e| e.to_string())?;

    let mut opts = SessionOptions::default();
    opts.disable_dht_persistence = true;
    opts.defer_writes_up_to = Some(64);
    opts.listen_port_range = Some(49152..65535);
    opts.disable_upload = true;

    let session = Session::new_with_opts(cache.clone(), opts)
        .await
        .map_err(|e| e.to_string())?;
    let api = Api::new(session, None);

    let mut add_opts = AddTorrentOptions::default();
    add_opts.overwrite = true;
    add_opts.output_folder = Some(cache.to_string_lossy().into_owned());
    add_opts.peer_opts = Some(PeerConnectionOptions {
        connect_timeout: Some(Duration::from_secs(10)),
        read_write_timeout: Some(Duration::from_secs(20)),
        ..Default::default()
    });

    let resp = api
        .api_add_torrent(AddTorrent::Url(magnet.into()), Some(add_opts))
        .await
        .map_err(|e| e.to_string())?;

    let torrent_id = resp.id.ok_or("torrent id alınamadı")?;
    let files = resp.details.files.as_ref().ok_or("dosya listesi alınamadı")?;

    let (file_id, file_name, file_len) = if let Some(idx) = file_idx {
        let f = files.get(idx).ok_or("fileIdx geçersiz")?;
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
            .ok_or("oynatılacak dosya bulunamadı")?
    };

    eprintln!("  [{file_id}] {file_name} ({} MB)", file_len / 1024 / 1024);

    let only = HashSet::from([file_id]);
    let _ = api
        .api_torrent_action_update_only_files(
            librqbit::api::TorrentIdOrHash::Id(torrent_id),
            &only,
        )
        .await;

    let ctx = Arc::new(Ctx { api, torrent_id, file_id, file_name, file_len });

    let listener = TcpListener::bind("127.0.0.1:0").await.map_err(|e| e.to_string())?;
    let port = listener.local_addr().unwrap().port();
    let url = format!("http://127.0.0.1:{port}/stream");

    let app = Router::new().route("/stream", get(handle_stream)).with_state(ctx);
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    Ok(url)
}

// ── ana akış ─────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    println!("╔══════════════════════════════════════════╗");
    println!("║  fluxa-play  │  Cinemeta + Torrentio     ║");
    println!("╚══════════════════════════════════════════╝");
    println!();

    // 1. Arama
    let query = readline("🔍 Ara: ");
    if query.is_empty() {
        return;
    }

    println!("  Aranıyor…");
    let results = tokio::task::spawn_blocking({
        let q = query.clone();
        move || cinemeta_search(&q)
    })
    .await
    .unwrap_or_default();

    if results.is_empty() {
        eprintln!("  Sonuç bulunamadı.");
        return;
    }

    println!();
    for (i, m) in results.iter().enumerate() {
        println!("  {}. {}{}", i + 1, m.name, m.year);
    }
    println!();

    let Some(idx) = pick_idx(results.len()) else {
        return;
    };
    let meta = &results[idx];

    // 2. Dizi → sezon / bölüm
    let video_id = if meta.kind == "series" {
        println!();
        let s = readline("  Sezon  : ");
        let e = readline("  Bölüm  : ");
        format!("{}:{}:{}", meta.id, s.trim(), e.trim())
    } else {
        meta.id.clone()
    };

    // 3. Torrentio
    println!("\n  Streamler yükleniyor…");
    let kind = meta.kind;
    let vid = video_id.clone();
    let streams = tokio::task::spawn_blocking(move || torrentio_streams(kind, &vid))
        .await
        .unwrap_or_default();

    if streams.is_empty() {
        eprintln!("  Torrentio'dan stream gelmedi.");
        return;
    }

    println!();
    for (i, s) in streams.iter().enumerate() {
        println!("  {}. {}", i + 1, s.label);
    }
    println!();

    let Some(sidx) = pick_idx(streams.len()) else {
        return;
    };
    let chosen = &streams[sidx];

    // 4. rqbit başlat
    let magnet = format!("magnet:?xt=urn:btih:{}&{TRACKERS}", chosen.info_hash);
    let file_idx = chosen.file_idx;

    println!("\n  rqbit başlatılıyor, metadata bekleniyor…");
    let url = match start_server(magnet, file_idx).await {
        Ok(u) => u,
        Err(e) => {
            eprintln!("  Hata: {e}");
            return;
        }
    };

    println!("  ✅ Hazır: {url}");
    println!("  mpv başlatılıyor…\n");

    // 5. mpv
    let _ = std::process::Command::new("mpv")
        .arg("--force-seekable=yes")
        .arg("--cache=yes")
        .arg("--demuxer-max-bytes=50MiB")
        .arg(&url)
        .status();
}
