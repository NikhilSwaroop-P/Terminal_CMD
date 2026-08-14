//! Embedded Local Agent API & Dual-Streaming Engine module.

pub mod ansi;
pub mod auth;
pub mod models;
pub mod routes;
pub mod server;
pub mod sse;
pub mod ws;

pub use auth::AuthState;
pub use server::{create_router, start_server, DEFAULT_API_PORT};
