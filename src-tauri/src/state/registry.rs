//! Global application state holding shared manager registries.

use std::sync::Arc;
use crate::pty::PtyManager;

/// Global shared state accessible across Tauri commands and API handlers.
#[derive(Clone)]
pub struct AppState {
    pub pty_manager: Arc<PtyManager>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    /// Creates a new initialized application state.
    pub fn new() -> Self {
        Self {
            pty_manager: Arc::new(PtyManager::new()),
        }
    }
}
