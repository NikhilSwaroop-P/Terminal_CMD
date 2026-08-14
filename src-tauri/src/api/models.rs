//! Data transfer objects and schema definitions for the TermCMD Agent API.

use std::collections::HashMap;
use std::path::PathBuf;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::pty::session::SessionInfo;

/// Payload for creating a new terminal session.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTerminalRequest {
    pub title: Option<String>,
    pub cwd: Option<PathBuf>,
    pub shell: Option<String>,
    pub cols: Option<u16>,
    pub rows: Option<u16>,
    pub env: Option<HashMap<String, String>>,
}

/// Response payload returned after creating a terminal session.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTerminalResponse {
    pub id: String,
    pub title: String,
    pub cwd: PathBuf,
    pub pid: Option<u32>,
    pub state: String,
    pub created_at: DateTime<Utc>,
}

/// Response payload for listing all terminal sessions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListTerminalsResponse {
    pub terminals: Vec<SessionInfo>,
}

/// Response payload for inspecting a single terminal session.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalDetailResponse {
    pub terminal: SessionInfo,
    pub buffer: Vec<String>,
}

/// Payload for resizing a terminal's dimensions.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResizeRequest {
    pub cols: u16,
    pub rows: u16,
}

/// Response payload after resizing a terminal.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResizeResponse {
    pub resized: bool,
    pub cols: u16,
    pub rows: u16,
}

/// Payload for writing raw input bytes to a terminal.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InputRequest {
    pub data: String,
}

/// Response payload after sending input.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputResponse {
    pub success: bool,
}

/// Payload for sending a signal to a terminal's process group.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KillRequest {
    pub signal: Option<String>,
}

/// Response payload after sending a signal.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KillResponse {
    pub signaled: bool,
    pub signal: String,
}

/// Response payload after deleting a terminal session.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteResponse {
    pub closed: bool,
}

/// Payload for executing a command in a terminal via SSE.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecRequest {
    pub command: String,
    pub strip_ansi: Option<bool>,
    pub timeout_seconds: Option<u64>,
    pub env: Option<HashMap<String, String>>,
}

/// Event payload emitted when command execution begins.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecStartPayload {
    pub command: String,
    pub timestamp: DateTime<Utc>,
}

/// Event payload emitted when terminal output pauses awaiting user input.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecPromptWaitingPayload {
    pub prompt_text: String,
    pub idle_ms: u64,
}

/// Event payload emitted when command execution finishes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecDonePayload {
    pub exit_code: i32,
    pub duration_ms: u64,
    pub command: String,
    pub cwd: String,
}

/// Structured error response payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiErrorResponse {
    pub error: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_pid: Option<u32>,
}
