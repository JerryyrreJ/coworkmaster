mod mailbox;
mod models;

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use models::{ConfigRequest, MessageRequest, SendRequest};
use serde::Serialize;
use std::{env, net::SocketAddr};

#[derive(Clone)]
struct AppState;

#[derive(Serialize)]
struct HealthResponse {
    ok: bool,
    service: &'static str,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        ok: true,
        service: "opportunity-autopilot-rust",
    })
}

async fn mail_test(
    State(_): State<AppState>,
    Json(request): Json<ConfigRequest>,
) -> impl IntoResponse {
    Json(mailbox::test_connection(&request.config).await)
}

async fn mail_unread(State(_): State<AppState>, Json(request): Json<ConfigRequest>) -> Response {
    result(mailbox::list_unread(&request.config).await)
}

async fn mail_messages(State(_): State<AppState>, Json(request): Json<ConfigRequest>) -> Response {
    result(mailbox::list_messages(&request.config).await)
}

async fn mail_message(State(_): State<AppState>, Json(request): Json<MessageRequest>) -> Response {
    result(mailbox::get_message(&request.config, &request.id).await)
}

async fn mail_send(State(_): State<AppState>, Json(request): Json<SendRequest>) -> Response {
    result(mailbox::send(&request.config, &request.message).await)
}

fn result<T: Serialize>(value: Result<T, mailbox::MailError>) -> Response {
    match value {
        Ok(value) => Json(value).into_response(),
        Err(error) => (
            StatusCode::BAD_GATEWAY,
            Json(ErrorResponse {
                error: error.to_string(),
            }),
        )
            .into_response(),
    }
}

#[tokio::main]
async fn main() {
    let port = env::var("RUST_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(8790);

    let state = AppState;
    let app = Router::new()
        .route("/health", get(health))
        .route("/mail/test", post(mail_test))
        .route("/mail/unread", post(mail_unread))
        .route("/mail/messages", post(mail_messages))
        .route("/mail/message", post(mail_message))
        .route("/mail/send", post(mail_send))
        .with_state(state);

    let address = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .expect("failed to bind Rust backend listener");

    println!("Rust backend listening on http://{address}");

    axum::serve(listener, app)
        .await
        .expect("Rust backend server failed");
}
