//! TermCMD Main binary entry point.

use termcmd_core::api::{start_server, AuthState, DEFAULT_API_PORT};
use termcmd_core::pty::SessionConfig;
use termcmd_core::state::AppState;
use tracing::info;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let args: Vec<String> = std::env::args().collect();
    let is_headless = args.iter().any(|arg| arg == "--headless" || arg == "--daemon");

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

    let token = auth_state.token().to_string();
    println!("TERMCMD_TOKEN: {}", token);

    let runtime = tokio::runtime::Runtime::new()?;
    let app_state_clone = app_state.clone();
    let auth_state_clone = auth_state.clone();

    let (bound_addr, _server_handle) = runtime.block_on(async {
        start_server(app_state_clone, auth_state_clone, DEFAULT_API_PORT).await
    })?;

    info!(
        addr = %bound_addr,
        token = %token,
        "Agent API server ready"
    );

    if is_headless {
        info!("Running in headless server daemon mode");
        runtime.block_on(async {
            let _ = tokio::signal::ctrl_c().await;
        });
        info!("Shutting down TermCMD");
        return Ok(());
    }

    tauri::Builder::default()
        .setup(move |_app| {
            info!("Tauri 2.0 Webview desktop window initialized");
            Ok(())
        })
        .run(tauri::generate_context!())
        .map_err(|e| std::io::Error::other(e.to_string()))?;

    Ok(())
}
