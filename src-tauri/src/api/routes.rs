//! REST endpoint handlers for terminal lifecycle, input dispatch, and dimension control.

use std::str::FromStr;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use nix::sys::signal::Signal;

use crate::api::models::{
    ApiErrorResponse, CreateTerminalRequest, CreateTerminalResponse, DeleteResponse,
    InputRequest, InputResponse, KillRequest, KillResponse, ListTerminalsResponse,
    ResizeRequest, ResizeResponse, TerminalDetailResponse,
};
use crate::pty::session::SessionConfig;
use crate::state::AppState;

/// Spawns a new persistent PTY terminal session.
pub async fn create_terminal(
    State(state): State<AppState>,
    Json(payload): Json<CreateTerminalRequest>,
) -> Response {
    let session_id = format!("term_{}", &uuid::Uuid::new_v4().to_string().replace('-', "")[..10]);
    let cols = payload.cols.unwrap_or(120);
    let rows = payload.rows.unwrap_or(35);

    let config = SessionConfig {
        id: session_id.clone(),
        title: payload.title,
        cwd: payload.cwd,
        shell: payload.shell,
        cols,
        rows,
        env: payload.env.unwrap_or_default(),
        inject_hooks: true,
    };

    match state.pty_manager.spawn_session(config) {
        Ok(session) => {
            let info = session.info();
            let response = CreateTerminalResponse {
                id: info.id,
                title: info.title,
                cwd: info.cwd,
                pid: info.pid,
                state: format!("{:?}", info.state),
                created_at: info.created_at,
            };
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(err) => {
            let error_body = ApiErrorResponse {
                error: "SpawnFailed".to_string(),
                message: format!("Failed to spawn PTY session: {}", err),
                active_pid: None,
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error_body)).into_response()
        }
    }
}

/// Lists all registered terminal sessions with status metadata.
pub async fn list_terminals(State(state): State<AppState>) -> Response {
    let terminals = state.pty_manager.list_sessions();
    (StatusCode::OK, Json(ListTerminalsResponse { terminals })).into_response()
}

/// Retrieves detailed session metadata and a recent scrollback buffer snapshot.
pub async fn get_terminal(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    match state.pty_manager.get_session(&id) {
        Some(session) => {
            let detail = TerminalDetailResponse {
                terminal: session.info(),
                buffer: session.get_buffer_snapshot(),
            };
            (StatusCode::OK, Json(detail)).into_response()
        }
        None => not_found_response(&id),
    }
}

/// Resizes the PTY dimensions and broadcasts SIGWINCH.
pub async fn resize_terminal(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<ResizeRequest>,
) -> Response {
    match state.pty_manager.get_session(&id) {
        Some(session) => match session.resize(payload.cols, payload.rows) {
            Ok(_) => (
                StatusCode::OK,
                Json(ResizeResponse {
                    resized: true,
                    cols: payload.cols,
                    rows: payload.rows,
                }),
            )
                .into_response(),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiErrorResponse {
                    error: "ResizeFailed".to_string(),
                    message: err.to_string(),
                    active_pid: None,
                }),
            )
                .into_response(),
        },
        None => not_found_response(&id),
    }
}

/// Sends raw standard input bytes to the slave PTY.
pub async fn send_input(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<InputRequest>,
) -> Response {
    match state.pty_manager.get_session(&id) {
        Some(session) => match session.write_all(payload.data.as_bytes()) {
            Ok(_) => (StatusCode::OK, Json(InputResponse { success: true })).into_response(),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiErrorResponse {
                    error: "InputFailed".to_string(),
                    message: err.to_string(),
                    active_pid: None,
                }),
            )
                .into_response(),
        },
        None => not_found_response(&id),
    }
}

/// Dispatches a POSIX signal to the foreground process group.
pub async fn kill_terminal(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<KillRequest>,
) -> Response {
    let session = match state.pty_manager.get_session(&id) {
        Some(s) => s,
        None => return not_found_response(&id),
    };

    let sig_str = payload.signal.unwrap_or_else(|| "SIGINT".to_string());
    let normalized = if sig_str.starts_with("SIG") {
        sig_str.clone()
    } else {
        format!("SIG{}", sig_str)
    };

    let parsed_sig = match Signal::from_str(&normalized) {
        Ok(s) => s,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiErrorResponse {
                    error: "InvalidSignal".to_string(),
                    message: format!("Unsupported or invalid signal '{}'", sig_str),
                    active_pid: None,
                }),
            )
                .into_response();
        }
    };

    let result = if parsed_sig == Signal::SIGINT {
        session.send_sigint()
    } else {
        session.send_signal(parsed_sig)
    };

    match result {
        Ok(_) => (
            StatusCode::OK,
            Json(KillResponse {
                signaled: true,
                signal: normalized,
            }),
        )
            .into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiErrorResponse {
                error: "SignalFailed".to_string(),
                message: err.to_string(),
                active_pid: None,
            }),
        )
            .into_response(),
    }
}

/// Closes the session and terminates child processes.
pub async fn delete_terminal(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    match state.pty_manager.get_session(&id) {
        Some(_) => match state.pty_manager.close_session(&id) {
            Ok(_) => (StatusCode::OK, Json(DeleteResponse { closed: true })).into_response(),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiErrorResponse {
                    error: "DeleteFailed".to_string(),
                    message: err.to_string(),
                    active_pid: None,
                }),
            )
                .into_response(),
        },
        None => not_found_response(&id),
    }
}

fn not_found_response(id: &str) -> Response {
    let error_body = ApiErrorResponse {
        error: "TerminalNotFound".to_string(),
        message: format!("Terminal session with id '{}' was not found", id),
        active_pid: None,
    };
    (StatusCode::NOT_FOUND, Json(error_body)).into_response()
}
