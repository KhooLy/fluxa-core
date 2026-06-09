/// rqbit bench — magnet → metadata → first 256 KB
/// Usage: cargo run --bin torrent_bench -- "<magnet>" [--bytes N]
/// Prints one JSON line.

use librqbit::{
    AddTorrent, AddTorrentOptions, Api, PeerConnectionOptions, Session, SessionOptions,
};
use std::io::SeekFrom;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncSeekExt};

const DEFAULT_BYTES: usize = 256 * 1024;
const DEFAULT_MAGNET: &str = concat!(
    "magnet:?xt=urn:btih:dd8255ecdc7ca55fb0bbf81323d87062db1f6d1c",
    "&dn=Big+Buck+Bunny",
    "&tr=udp://tracker.opentrackr.org:1337/announce",
    "&tr=udp://open.stealth.si:80/announce",
    "&tr=udp://tracker.openbittorrent.com:80/announce"
);

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let magnet = args.get(1).map(String::as_str).unwrap_or(DEFAULT_MAGNET);
    let bytes_target: usize = args
        .windows(2)
        .find(|w| w[0] == "--bytes")
        .and_then(|w| w[1].parse().ok())
        .unwrap_or(DEFAULT_BYTES);

    let cache = std::env::temp_dir().join("fluxa_bench_rqbit");
    let _ = std::fs::remove_dir_all(&cache);
    std::fs::create_dir_all(&cache).unwrap();

    eprintln!("[rqbit] starting session…");
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
    let t_engine = t0.elapsed();
    eprintln!("[rqbit] engine ready in {}ms", t_engine.as_millis());

    let mut add_opts = AddTorrentOptions::default();
    add_opts.overwrite = true;
    add_opts.output_folder = Some(cache.to_string_lossy().into_owned());
    add_opts.peer_opts = Some(PeerConnectionOptions {
        connect_timeout: Some(Duration::from_secs(10)),
        read_write_timeout: Some(Duration::from_secs(20)),
        ..Default::default()
    });

    eprintln!("[rqbit] adding torrent…");
    let t_add = Instant::now();
    let response = api
        .api_add_torrent(AddTorrent::Url(magnet.to_string().into()), Some(add_opts))
        .await
        .expect("add_torrent");

    let torrent_id = response.id.expect("metadata must be ready after add");
    let t_metadata = t_add.elapsed();
    eprintln!("[rqbit] metadata in {}ms", t_metadata.as_millis());

    let files = response.details.files.as_ref().expect("file list");
    let (file_id, file_name, file_len) = files
        .iter()
        .enumerate()
        .filter(|(_, f)| {
            let n = f.name.to_ascii_lowercase();
            matches!(
                n.rsplit('.').next().unwrap_or(""),
                "mkv" | "mp4" | "avi" | "webm" | "m4v" | "mov"
            )
        })
        .max_by_key(|(_, f)| f.length)
        .or_else(|| files.iter().enumerate().max_by_key(|(_, f)| f.length))
        .map(|(i, f)| (i, f.name.clone(), f.length))
        .expect("at least one file");

    eprintln!("[rqbit] target: [{file_id}] {file_name} ({} MB)", file_len / 1024 / 1024);

    // prioritize only the target file
    {
        use std::collections::HashSet;
        let only = HashSet::from([file_id]);
        let _ = api
            .api_torrent_action_update_only_files(
                librqbit::api::TorrentIdOrHash::Id(torrent_id),
                &only,
            )
            .await;
    }

    eprintln!("[rqbit] waiting for first {}KB…", bytes_target / 1024);
    let t_read_start = Instant::now();
    let mut first_byte_ms: Option<u128> = None;
    let mut total_read = 0usize;

    // retry until api_stream succeeds (torrent must be Live)
    let mut stream = loop {
        match api.api_stream(
            librqbit::api::TorrentIdOrHash::Id(torrent_id),
            file_id,
        ) {
            Ok(s) => break s,
            Err(_) => tokio::time::sleep(Duration::from_millis(100)).await,
        }
    };

    // seek to start (just in case)
    stream.seek(SeekFrom::Start(0)).await.ok();

    let mut buf = vec![0u8; 32 * 1024];
    while total_read < bytes_target {
        let n = stream.read(&mut buf).await.expect("read");
        if n == 0 {
            break;
        }
        if first_byte_ms.is_none() {
            first_byte_ms = Some(t_read_start.elapsed().as_millis());
            eprintln!("[rqbit] first byte in {}ms", first_byte_ms.unwrap());
        }
        total_read += n;
    }
    let t_data_done = t_read_start.elapsed();
    eprintln!(
        "[rqbit] {}KB read in {}ms",
        total_read / 1024,
        t_data_done.as_millis()
    );

    let out = serde_json::json!({
        "engine": "rqbit",
        "engineStartupMs": t_engine.as_millis(),
        "metadataMs": t_metadata.as_millis(),
        "firstByteMs": first_byte_ms,
        "readDoneMs": t_data_done.as_millis(),
        "bytesRead": total_read,
        "fileName": file_name,
        "fileSizeBytes": file_len
    });
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}
