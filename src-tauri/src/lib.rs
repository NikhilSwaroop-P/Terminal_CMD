//! TermCMD Backend Core & PTY Engine.
//!
//! Provides asynchronous PTY management, zero-mangling shell integrations,
//! process group supervision, and circular output buffering.

pub mod api;
pub mod pty;
pub mod state;

pub use api::*;
pub use pty::*;
pub use state::AppState;

