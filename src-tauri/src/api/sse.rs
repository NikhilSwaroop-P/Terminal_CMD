//! Dual-streaming Server-Sent Events (SSE) command execution engine.
//!
//! Streams real-time stdout/stderr, ANSI-stripped tokens for LLM context,
//! interactive prompt detection markers, and semantic exit codes.

use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::Utc;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use crate::api::ansi::strip_ansi;
use crate::api::models::{
    ApiErrorResponse, ExecDonePayload, ExecPromptWaitingPayload, ExecRequest, ExecStartPayload,
};
use crate::pty::osc::OscEvent;
use crate::pty::session::{PtySession, SessionEvent, SessionState};
use crate::state::AppState;

/// Executes a command in the targeted terminal and streams stdout/events via SSE.
pub async fn exec_terminal(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<ExecRequest>,
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

    let session_info = session.info();
    if let SessionState::Running { command } = session.state() {
        let active_cmd = command.unwrap_or_else(|| "active process".to_string());
        let error_body = ApiErrorResponse {
            error: "TerminalBusy".to_string(),
            message: format!(
                "Terminal is currently executing PID {} ('{}'). Use /input to send stdin or spawn a new terminal.",
                session_info.pid.unwrap_or(0),
                active_cmd
            ),
            active_pid: session_info.pid,
        };
        return (StatusCode::CONFLICT, Json(error_body)).into_response();
    }

    let (tx, rx) = mpsc::channel::<Result<Event, Infallible>>(128);
    let session_clone = session.clone();

    tokio::spawn(async move {
        run_exec_stream(session_clone, payload, tx).await;
    });

    let stream = ReceiverStream::new(rx);
    Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
        .into_response()
}

async fn run_exec_stream(
    session: Arc<PtySession>,
    payload: ExecRequest,
    tx: mpsc::Sender<Result<Event, Infallible>>,
) {
    let start_instant = Instant::now();
    let should_strip = payload.strip_ansi.unwrap_or(true);
    let timeout_secs = payload.timeout_seconds.unwrap_or(300);
    let timeout_dur = Duration::from_secs(timeout_secs);

    let full_command = format_command_with_env(&payload.command, payload.env.as_ref());

    let mut event_rx = session.subscribe();
    while event_rx.try_recv().is_ok() {}

    let start_payload = ExecStartPayload {
        command: payload.command.clone(),
        timestamp: Utc::now(),
    };
    if let Ok(data) = serde_json::to_string(&start_payload) {
        if tx.send(Ok(Event::default().event("start").data(data))).await.is_err() {
            return;
        }
    }

    if let Err(_err) = session.write_command(&full_command) {
        let done_payload = ExecDonePayload {
            exit_code: 1,
            duration_ms: start_instant.elapsed().as_millis() as u64,
            command: payload.command,
            cwd: session.cwd().to_string_lossy().to_string(),
        };
        if let Ok(data) = serde_json::to_string(&done_payload) {
            let _ = tx.send(Ok(Event::default().event("done").data(data))).await;
        }
        return;
    }

    let mut command_started = true;
    let mut prompt_waiting_emitted = false;
    let mut last_activity = Instant::now();
    let mut last_output_text = String::new();
    let mut ticker = tokio::time::interval(Duration::from_millis(100));

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                if start_instant.elapsed() >= timeout_dur {
                    let done_payload = ExecDonePayload {
                        exit_code: 124,
                        duration_ms: start_instant.elapsed().as_millis() as u64,
                        command: payload.command.clone(),
                        cwd: session.cwd().to_string_lossy().to_string(),
                    };
                    if let Ok(data) = serde_json::to_string(&done_payload) {
                        let _ = tx.send(Ok(Event::default().event("done").data(data))).await;
                    }
                    break;
                }

                if command_started && !prompt_waiting_emitted && last_activity.elapsed() >= Duration::from_millis(1500) && !last_output_text.is_empty() {
                    let prompt_payload = ExecPromptWaitingPayload {
                        prompt_text: last_output_text.trim().to_string(),
                        idle_ms: last_activity.elapsed().as_millis() as u64,
                    };
                    if let Ok(data) = serde_json::to_string(&prompt_payload) {
                        if tx.send(Ok(Event::default().event("prompt_waiting").data(data))).await.is_err() {
                            break;
                        }
                    }
                    prompt_waiting_emitted = true;
                }
            }

            recv_res = event_rx.recv() => {
                match recv_res {
                    Ok(SessionEvent::Output(chunk)) => {
                        let raw_str = String::from_utf8_lossy(&chunk);
                        let clean_chunk = if should_strip {
                            strip_ansi(&raw_str)
                        } else {
                            raw_str.to_string()
                        };

                        if !clean_chunk.is_empty() {
                            last_output_text = clean_chunk.clone();
                            last_activity = Instant::now();
                            prompt_waiting_emitted = false;

                            if tx.send(Ok(Event::default().event("stdout").data(clean_chunk))).await.is_err() {
                                break;
                            }
                        }
                    }

                    Ok(SessionEvent::Osc(OscEvent::CommandStart | OscEvent::OutputStart)) => {
                        command_started = true;
                    }

                    Ok(SessionEvent::Osc(OscEvent::CommandFinished { exit_code })) => {
                        if command_started || start_instant.elapsed() > Duration::from_millis(800) || exit_code != 0 {
                            let done_payload = ExecDonePayload {
                                exit_code,
                                duration_ms: start_instant.elapsed().as_millis() as u64,
                                command: payload.command.clone(),
                                cwd: session.cwd().to_string_lossy().to_string(),
                            };
                            if let Ok(data) = serde_json::to_string(&done_payload) {
                                let _ = tx.send(Ok(Event::default().event("done").data(data))).await;
                            }
                            break;
                        }
                    }

                    Ok(SessionEvent::Terminated { exit_code }) => {
                        let done_payload = ExecDonePayload {
                            exit_code: exit_code.unwrap_or(0),
                            duration_ms: start_instant.elapsed().as_millis() as u64,
                            command: payload.command.clone(),
                            cwd: session.cwd().to_string_lossy().to_string(),
                        };
                        if let Ok(data) = serde_json::to_string(&done_payload) {
                            let _ = tx.send(Ok(Event::default().event("done").data(data))).await;
                        }
                        break;
                    }

                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        break;
                    }

                    _ => {}
                }
            }
        }
    }
}

fn format_command_with_env(command: &str, env: Option<&HashMap<String, String>>) -> String {
    match env {
        Some(vars) if !vars.is_empty() => {
            let mut prefix = String::from("env ");
            for (k, v) in vars {
                let escaped_val = v.replace('\'', "'\\''");
                prefix.push_str(&format!("{}='{}' ", k, escaped_val));
            }
            format!("{}{}", prefix, command)
        }
        _ => command.to_string(),
    }
}
