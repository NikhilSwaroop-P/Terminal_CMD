//! Bearer token authentication guard and runtime token persistence.
//!
//! Enforces security on all Agent API routes by generating a session-scoped
//! token and persisting it with restrictive 0600 POSIX permissions.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::{header, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Json;

use crate::api::models::ApiErrorResponse;

/// Shared authentication state containing the active bearer token.
#[derive(Clone)]
pub struct AuthState {
    token: Arc<String>,
    token_path: Option<PathBuf>,
}

impl AuthState {
    /// Initializes authentication state with a newly generated or environment-provided token.
    pub fn new() -> Self {
        let token = std::env::var("TERMCMD_TOKEN").unwrap_or_else(|_| {
            uuid::Uuid::new_v4().to_string().replace('-', "")
        });

        let token_path = Self::persist_token(&token);

        Self {
            token: Arc::new(token),
            token_path,
        }
    }

    /// Creates an authentication state with an explicit token (useful for tests).
    pub fn with_token(token: &str) -> Self {
        Self {
            token: Arc::new(token.to_string()),
            token_path: None,
        }
    }

    /// Returns the active bearer token string.
    pub fn token(&self) -> &str {
        &self.token
    }

    /// Returns the filesystem path where the token was saved.
    pub fn token_path(&self) -> Option<&Path> {
        self.token_path.as_deref()
    }

    /// Validates an incoming token string.
    pub fn is_valid(&self, candidate: &str) -> bool {
        self.token.as_str() == candidate
    }

    fn persist_token(token: &str) -> Option<PathBuf> {
        let target_path = if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
            PathBuf::from(runtime_dir).join("termcmd.token")
        } else if let Ok(home) = std::env::var("HOME") {
            let config_dir = PathBuf::from(home).join(".config").join("termcmd");
            let _ = fs::create_dir_all(&config_dir);
            config_dir.join("token")
        } else {
            std::env::temp_dir().join("termcmd.token")
        };

        if let Some(parent) = target_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let mut options = OpenOptions::new();
        options.write(true).create(true).truncate(true);

        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }

        if let Ok(mut file) = options.open(&target_path) {
            if file.write_all(token.as_bytes()).is_ok() {
                return Some(target_path);
            }
        }

        None
    }
}

impl Default for AuthState {
    fn default() -> Self {
        Self::new()
    }
}

/// Axum middleware checking Bearer token in headers or query parameters.
pub async fn auth_middleware(
    State(auth): State<AuthState>,
    request: Request,
    next: Next,
) -> Response {
    let mut provided_token: Option<String> = None;

    if let Some(auth_header) = request.headers().get(header::AUTHORIZATION) {
        if let Ok(header_str) = auth_header.to_str() {
            if let Some(bearer) = header_str.strip_prefix("Bearer ") {
                provided_token = Some(bearer.trim().to_string());
            }
        }
    }

    if provided_token.is_none() {
        if let Some(query_str) = request.uri().query() {
            for pair in query_str.split('&') {
                if let Some((k, v)) = pair.split_once('=') {
                    if k == "token" {
                        provided_token = Some(v.to_string());
                        break;
                    }
                }
            }
        }
    }

    match provided_token {
        Some(token) if auth.is_valid(&token) => next.run(request).await,
        _ => {
            let error_body = ApiErrorResponse {
                error: "Unauthorized".to_string(),
                message: "Missing or invalid Bearer token in Authorization header or query parameter".to_string(),
                active_pid: None,
            };
            (StatusCode::UNAUTHORIZED, Json(error_body)).into_response()
        }
    }
}

