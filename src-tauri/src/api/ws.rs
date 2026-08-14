//! Full-duplex WebSocket streaming handler for interactive canvas terminal rendering.
//!
//! Bridges xterm.js frontend clients and persistent backend PTY instances.

use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use futures_util::{SinkExt, StreamExt};

use crate::api::models::ApiErrorResponse;
use crate::pty::session::{PtySession, SessionEvent};
use crate::state::AppState;

/// Upgrades HTTP connection to full-duplex WebSocket streaming for terminal I/O.
pub async fn ws_terminal(
    State(state): State<AppState>,
    Path(id): Path<String>,
    ws: WebSocketUpgrade,
) -> Response {
    let session = match state.pty_manager.get_session(&id) {
        Some(s) => s,
        None => {
            let error_body = ApiErrorResponse {
                error: "TerminalNotFound".to_string(),
                message: format!("Terminal session '{}' was not found", id),
                active_pid: None,
            };
            return (StatusCode::NOT_FOUND, Json(error_body)).into_response();
        }
    };

    ws.on_upgrade(move |socket| handle_socket(socket, session))
}

async fn handle_socket(socket: WebSocket, session: Arc<PtySession>) {
    let (mut ws_sender, mut ws_receiver) = socket.split();
    let mut event_rx = session.subscribe();

    let session_writer = session.clone();
    let mut receive_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_receiver.next().await {
            match msg {
                Message::Text(text) => {
                    let _ = session_writer.write_all(text.as_bytes());
                }
                Message::Binary(bytes) => {
                    let _ = session_writer.write_all(&bytes);
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    let mut send_task = tokio::spawn(async move {
        while let Ok(event) = event_rx.recv().await {
            if let SessionEvent::Output(chunk) = event {
                if ws_sender.send(Message::Binary(chunk)).await.is_err() {
                    break;
                }
            }
        }
    });

    tokio::select! {
        _ = (&mut receive_task) => send_task.abort(),
        _ = (&mut send_task) => receive_task.abort(),
    }
}
