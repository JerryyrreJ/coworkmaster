use axum::{Json, Router, routing::get};
use serde::Serialize;
use std::{env, net::SocketAddr};

#[derive(Serialize)]
struct HealthResponse {
    ok: bool,
    service: &'static str,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        ok: true,
        service: "opportunity-autopilot-rust",
    })
}

#[tokio::main]
async fn main() {
    let port = env::var("RUST_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(8790);

    let app = Router::new().route("/health", get(health));
    let address = SocketAddr::from(([127, 0, 0, 1], port));

    let listener = tokio::net::TcpListener::bind(address)
        .await
        .expect("failed to bind Rust backend listener");

    println!("Rust backend listening on http://{address}");

    axum::serve(listener, app)
        .await
        .expect("Rust backend server failed");
}
