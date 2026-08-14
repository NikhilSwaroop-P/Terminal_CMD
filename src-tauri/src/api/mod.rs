//! Embedded Local Agent API & Dual-Streaming Engine module.

pub mod ansi;
pub mod auth;
pub mod discovery;
pub mod models;
pub mod routes;
pub mod server;
pub mod sse;
pub mod ws;

pub use auth::AuthState;
pub use discovery::{persist_port, resolve_connection, resolve_port, resolve_token, ConnectionInfo};
pub use server::{create_router, start_server, DEFAULT_API_PORT};
