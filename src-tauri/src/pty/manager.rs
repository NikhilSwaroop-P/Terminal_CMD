//! Central registry and supervisor for managing multiple concurrent PTY sessions.
//!
//! Provides thread-safe CRUD operations, lifecycle orchestration, and crash recovery.

use std::collections::HashMap;
use std::sync::Arc;

use nix::sys::signal::Signal;
use parking_lot::RwLock;

use crate::pty::session::{PtySession, SessionConfig, SessionInfo};

/// Central manager orchestrating all active and terminated PTY sessions.
#[derive(Default)]
pub struct PtyManager {
    sessions: RwLock<HashMap<String, Arc<PtySession>>>,
}

impl PtyManager {
    /// Creates a new empty PTY manager instance.
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
        }
    }

    /// Spawns a new PTY session with the given configuration and registers it.
    pub fn spawn_session(&self, config: SessionConfig) -> std::io::Result<Arc<PtySession>> {
        let session_id = config.id.clone();
        let session = PtySession::spawn(config)?;
        self.sessions.write().insert(session_id, session.clone());
        Ok(session)
    }

    /// Retrieves an Arc reference to an existing PTY session by ID.
    pub fn get_session(&self, id: &str) -> Option<Arc<PtySession>> {
        self.sessions.read().get(id).cloned()
    }

    /// Returns a list of metadata snapshots for all registered sessions.
    pub fn list_sessions(&self) -> Vec<SessionInfo> {
        self.sessions
            .read()
            .values()
            .map(|session| session.info())
            .collect()
    }

    /// Resizes the specified terminal session dimensions.
    pub fn resize_session(&self, id: &str, cols: u16, rows: u16) -> std::io::Result<()> {
        let session = self.get_session(id).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, format!("session {} not found", id))
        })?;
        session.resize(cols, rows)
    }

    /// Writes raw input bytes to the specified terminal stdin.
    pub fn write_input(&self, id: &str, data: &[u8]) -> std::io::Result<()> {
        let session = self.get_session(id).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, format!("session {} not found", id))
        })?;
        session.write_all(data)
    }

    /// Sends an interrupt (SIGINT / Ctrl+C) to the specified session.
    pub fn send_sigint(&self, id: &str) -> std::io::Result<()> {
        let session = self.get_session(id).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, format!("session {} not found", id))
        })?;
        session.send_sigint()
    }

    /// Sends a termination signal to the session's child process.
    pub fn kill_session(&self, id: &str, sig: Option<Signal>) -> std::io::Result<()> {
        let session = self.get_session(id).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, format!("session {} not found", id))
        })?;
        let signal_to_send = sig.unwrap_or(Signal::SIGTERM);
        session.send_signal(signal_to_send)
    }

    /// Closes and unregisters a session from the manager.
    pub fn close_session(&self, id: &str) -> std::io::Result<()> {
        if let Some(session) = self.sessions.write().remove(id) {
            let _ = session.send_signal(Signal::SIGKILL);
        }
        Ok(())
    }

    /// Respawns a terminated session in its last-known working directory.
    pub fn respawn_session(&self, id: &str) -> std::io::Result<Arc<PtySession>> {
        let old_info = {
            let session = self.get_session(id).ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::NotFound, format!("session {} not found", id))
            })?;
            session.info()
        };

        let new_config = SessionConfig {
            id: id.to_string(),
            title: Some(old_info.title),
            cwd: Some(old_info.cwd),
            shell: Some(old_info.shell),
            cols: old_info.cols,
            rows: old_info.rows,
            env: HashMap::new(),
            inject_hooks: true,
        };

        let new_session = PtySession::spawn(new_config)?;
        self.sessions.write().insert(id.to_string(), new_session.clone());
        Ok(new_session)
    }

    /// Returns the total count of registered sessions.
    pub fn session_count(&self) -> usize {
        self.sessions.read().len()
    }
}
