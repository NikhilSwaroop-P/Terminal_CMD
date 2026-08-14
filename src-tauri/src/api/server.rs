//! Embedded Axum server bootstrap, CORS configuration, and loopback listener.

use std::net::SocketAddr;

use axum::extract::State;
use axum::http::{header, HeaderValue, Method};
use axum::middleware;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::json;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tracing::info;

use crate::api::auth::{auth_middleware, AuthState};
use crate::api::routes::{
    create_terminal, delete_terminal, get_terminal, kill_terminal, list_terminals,
    resize_terminal, send_input,
};
use crate::api::sse::exec_terminal;
use crate::api::ws::ws_terminal;
use crate::state::AppState;

async fn token_endpoint(State(auth): State<AuthState>) -> Json<serde_json::Value> {
    Json(json!({ "token": auth.token() }))
}

/// Default port for the TermCMD Agent API server.
pub const DEFAULT_API_PORT: u16 = 7890;

/// Constructs the complete Axum Router with authentication middleware and CORS enforcement.
pub fn create_router(app_state: AppState, auth_state: AuthState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(|origin: &HeaderValue, _| {
            if let Ok(origin_str) = origin.to_str() {
                origin_str.starts_with("tauri://localhost")
                    || origin_str.starts_with("https://tauri.localhost")
                    || origin_str.starts_with("http://localhost")
                    || origin_str.starts_with("http://127.0.0.1")
            } else {
                false
            }
        }))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            header::ACCEPT,
        ]);

    let protected_routes = Router::new()
        .route("/api/v1/terminals", post(create_terminal).get(list_terminals))
        .route("/api/v1/terminals/:id", get(get_terminal).delete(delete_terminal))
        .route("/api/v1/terminals/:id/resize", post(resize_terminal))
        .route("/api/v1/terminals/:id/input", post(send_input))
        .route("/api/v1/terminals/:id/kill", post(kill_terminal))
        .route("/api/v1/terminals/:id/exec", post(exec_terminal))
        .route("/api/v1/terminals/:id/ws", get(ws_terminal))
        .layer(middleware::from_fn_with_state(
            auth_state.clone(),
            auth_middleware,
        ))
        .with_state(app_state);

    Router::new()
        .route("/__token", get(token_endpoint).with_state(auth_state))
        .merge(protected_routes)
        .layer(cors)
}

/// Starts the embedded API server on loopback with fallback port handling.
pub async fn start_server(
    app_state: AppState,
    auth_state: AuthState,
    initial_port: u16,
) -> std::io::Result<(SocketAddr, tokio::task::JoinHandle<()>)> {
    let router = create_router(app_state, auth_state);

    let mut port = initial_port;
    let mut listener = None;

    for _ in 0..10 {
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        match tokio::net::TcpListener::bind(addr).await {
            Ok(l) => {
                listener = Some((addr, l));
                break;
            }
            Err(_) => {
                port += 1;
            }
        }
    }

    let (_requested_addr, tcp_listener) = listener.ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::AddrNotAvailable,
            "Could not bind to any loopback port in range",
        )
    })?;

    let bound_addr = tcp_listener.local_addr()?;
    let _ = crate::api::discovery::persist_port(bound_addr.port());

    info!(
        addr = %bound_addr,
        "Embedded TermCMD Agent API server listening"
    );

    let handle = tokio::spawn(async move {
        let _ = axum::serve(tcp_listener, router).await;
    });

    Ok((bound_addr, handle))
}
