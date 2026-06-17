#[tokio::main]
async fn main() {
    let port: u16 = std::env::var("FLUXA_COMPANION_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(48211);

    fluxa_streaming_engine::companion_server::serve(port)
        .await
        .expect("companion server crashed");
}
