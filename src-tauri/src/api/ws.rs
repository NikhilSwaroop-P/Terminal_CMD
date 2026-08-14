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

fn is_ignored_terminal_query_response(bytes: &[u8]) -> bool {
    bytes.starts_with(b"\x1b]10;")
        || bytes.starts_with(b"\x1b]11;")
        || bytes.starts_with(b"\x1b[4;")
        || bytes.starts_with(b"\x1b[8;")
        || (bytes.starts_with(b"\x1b[") && bytes.ends_with(b"t"))
        || (bytes.starts_with(b"\x1b[?") && bytes.ends_with(b"c"))
}

async fn handle_socket(socket: WebSocket, session: Arc<PtySession>) {
    let (mut ws_sender, mut ws_receiver) = socket.split();
    let mut event_rx = session.subscribe();

    let initial_snapshot = session.get_buffer_snapshot();
    if !initial_snapshot.is_empty() {
        let initial_bytes = initial_snapshot.join("\r\n");
        if !initial_bytes.is_empty() {
            let _ = ws_sender.send(Message::Binary(initial_bytes.into_bytes())).await;
        }
    }

    let session_writer = session.clone();
    let mut receive_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_receiver.next().await {
            match msg {
                Message::Text(text) => {
                    if !is_ignored_terminal_query_response(text.as_bytes()) {
                        let _ = session_writer.write_all(text.as_bytes());
                    }
                }
                Message::Binary(bytes) => {
                    if !is_ignored_terminal_query_response(&bytes) {
                        let _ = session_writer.write_all(&bytes);
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    let mut send_task = tokio::spawn(async move {
        loop {
            match event_rx.recv().await {
                Ok(SessionEvent::Output(chunk)) => {
                    if ws_sender.send(Message::Binary(chunk)).await.is_err() {
                        break;
                    }
                }
                Ok(_) => {}
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    tokio::select! {
        _ = (&mut receive_task) => send_task.abort(),
        _ = (&mut send_task) => receive_task.abort(),
    }
}
