//! TermCMD Main binary entry point.

use termcmd_core::api::{start_server, AuthState, DEFAULT_API_PORT};
use termcmd_core::pty::SessionConfig;
use termcmd_core::state::AppState;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    info!("Starting TermCMD Backend Core & Embedded Agent API Server");

    let app_state = AppState::new();
    let auth_state = AuthState::new();

    let initial_session = app_state
        .pty_manager
        .spawn_session(SessionConfig::default())?;

    info!(
        session_id = %initial_session.id,
        pid = ?initial_session.info().pid,
        "Spawned initial default PTY session"
    );

    let (bound_addr, _server_handle) =
        start_server(app_state, auth_state.clone(), DEFAULT_API_PORT).await?;

    let token = auth_state.token().to_string();
    let _ = std::fs::write("/tmp/termcmd_token", &token);
    println!("TERMCMD_TOKEN: {}", token);

    info!(
        addr = %bound_addr,
        token = %token,
        "Agent API server ready"
    );

    tokio::signal::ctrl_c().await?;
    info!("Shutting down TermCMD");

    Ok(())
}

