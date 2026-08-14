//! Automated Integration Tests for TermCMD Agent CLI Binary (`termcmd-cli`).
//!
//! Validates XDG/POSIX discovery, spawn, tabular & JSON listing, real-time
//! streaming execution, exit code propagation, stdin input forwarding, and session cleanup.

use std::time::Duration;
use tokio::process::Command;

use termcmd_core::api::auth::AuthState;
use termcmd_core::api::discovery::{resolve_connection, resolve_port, resolve_token};
use termcmd_core::api::start_server;
use termcmd_core::pty::session::SessionInfo;
use termcmd_core::state::AppState;

fn cli_bin() -> &'static str {
    env!("CARGO_BIN_EXE_termcmd-cli")
}

#[tokio::test]
async fn test_cli_discovery_resolution() {
    let explicit = resolve_connection(
        Some("127.0.0.1:8999"),
        Some(8999),
        Some("test_discovery_token"),
    )
    .expect("resolve explicit connection");

    assert_eq!(explicit.base_url, "http://127.0.0.1:8999");
    assert_eq!(explicit.token, "test_discovery_token");

    std::env::set_var("TERMCMD_PORT", "9123");
    assert_eq!(resolve_port(), 9123);
    std::env::remove_var("TERMCMD_PORT");

    std::env::set_var("TERMCMD_TOKEN", "env_token_override_xyz");
    assert_eq!(
        resolve_token().expect("resolve token from env"),
        "env_token_override_xyz"
    );
    std::env::remove_var("TERMCMD_TOKEN");
}

#[tokio::test]
async fn test_cli_spawn_and_list() {
    let app_state = AppState::new();
    let token = "cli-test-token-spawn";
    let auth_state = AuthState::with_token(token);

    let (addr, server_handle) = start_server(app_state, auth_state, 0)
        .await
        .expect("start test server");

    let base_url = format!("http://{}", addr);

    let spawn_output = Command::new(cli_bin())
        .args([
            "--url",
            &base_url,
            "--token",
            token,
            "spawn",
            "--title",
            "CLI-Integration-Terminal",
            "--cols",
            "100",
            "--rows",
            "30",
        ])
        .output()
        .await
        .expect("failed to run termcmd spawn");

    assert!(
        spawn_output.status.success(),
        "termcmd spawn failed: {}",
        String::from_utf8_lossy(&spawn_output.stderr)
    );

    let session_id = String::from_utf8_lossy(&spawn_output.stdout)
        .trim()
        .to_string();
    assert!(
        session_id.starts_with("term_"),
        "Expected session id starting with 'term_', got '{}'",
        session_id
    );

    let list_json_output = Command::new(cli_bin())
        .args(["--url", &base_url, "--token", token, "list", "--json"])
        .output()
        .await
        .expect("failed to run termcmd list --json");

    assert!(list_json_output.status.success());
    let terminals: Vec<SessionInfo> =
        serde_json::from_slice(&list_json_output.stdout).expect("valid json terminal list");

    assert_eq!(terminals.len(), 1);
    assert_eq!(terminals[0].id, session_id);
    assert_eq!(terminals[0].title, "CLI-Integration-Terminal");
    assert_eq!(terminals[0].cols, 100);
    assert_eq!(terminals[0].rows, 30);

    let list_table_output = Command::new(cli_bin())
        .args(["--url", &base_url, "--token", token, "list"])
        .output()
        .await
        .expect("failed to run termcmd list");

    assert!(list_table_output.status.success());
    let table_str = String::from_utf8_lossy(&list_table_output.stdout);
    assert!(table_str.contains(&session_id));
    assert!(table_str.contains("CLI-Integration-Terminal"));

    server_handle.abort();
}

#[tokio::test]
async fn test_cli_exec_streaming_output() {
    let app_state = AppState::new();
    let token = "cli-test-token-exec";
    let auth_state = AuthState::with_token(token);

    let (addr, server_handle) = start_server(app_state, auth_state, 0)
        .await
        .expect("start test server");

    let base_url = format!("http://{}", addr);

    let spawn_output = Command::new(cli_bin())
        .args([
            "--url",
            &base_url,
            "--token",
            token,
            "spawn",
            "--title",
            "Exec-Test-Terminal",
        ])
        .output()
        .await
        .expect("spawn session");

    let session_id = String::from_utf8_lossy(&spawn_output.stdout)
        .trim()
        .to_string();

    tokio::time::sleep(Duration::from_millis(500)).await;

    let exec_output = Command::new(cli_bin())
        .args([
            "--url",
            &base_url,
            "--token",
            token,
            "exec",
            &session_id,
            "echo 'Live Streaming Line CLI Test'",
        ])
        .output()
        .await
        .expect("exec command");

    assert!(
        exec_output.status.success(),
        "termcmd exec failed with stderr: {}",
        String::from_utf8_lossy(&exec_output.stderr)
    );

    let stdout_str = String::from_utf8_lossy(&exec_output.stdout);
    assert!(
        stdout_str.contains("Live Streaming Line CLI Test"),
        "stdout was: {}",
        stdout_str
    );

    server_handle.abort();
}

#[tokio::test]
async fn test_cli_exit_code_propagation() {
    let app_state = AppState::new();
    let token = "cli-test-token-exitcode";
    let auth_state = AuthState::with_token(token);

    let (addr, server_handle) = start_server(app_state, auth_state, 0)
        .await
        .expect("start test server");

    let base_url = format!("http://{}", addr);

    let spawn_output = Command::new(cli_bin())
        .args(["--url", &base_url, "--token", token, "spawn"])
        .output()
        .await
        .expect("spawn session");

    let session_id = String::from_utf8_lossy(&spawn_output.stdout)
        .trim()
        .to_string();

    tokio::time::sleep(Duration::from_millis(500)).await;

    let exec_output = Command::new(cli_bin())
        .args([
            "--url",
            &base_url,
            "--token",
            token,
            "exec",
            &session_id,
            "sh -c 'exit 42'",
        ])
        .output()
        .await
        .expect("exec command");

    assert_eq!(
        exec_output.status.code(),
        Some(42),
        "Expected exit code 42, got {:?}",
        exec_output.status.code()
    );

    server_handle.abort();
}

#[tokio::test]
async fn test_cli_snapshot_and_close() {
    let app_state = AppState::new();
    let token = "cli-test-token-close";
    let auth_state = AuthState::with_token(token);

    let (addr, server_handle) = start_server(app_state, auth_state, 0)
        .await
        .expect("start test server");

    let base_url = format!("http://{}", addr);

    let spawn_output = Command::new(cli_bin())
        .args(["--url", &base_url, "--token", token, "spawn"])
        .output()
        .await
        .expect("spawn session");

    let session_id = String::from_utf8_lossy(&spawn_output.stdout)
        .trim()
        .to_string();

    tokio::time::sleep(Duration::from_millis(300)).await;

    let _ = Command::new(cli_bin())
        .args([
            "--url",
            &base_url,
            "--token",
            token,
            "exec",
            &session_id,
            "echo 'Snapshot Buffer Marker'",
        ])
        .output()
        .await;

    let snapshot_output = Command::new(cli_bin())
        .args([
            "--url",
            &base_url,
            "--token",
            token,
            "snapshot",
            &session_id,
            "--lines",
            "10",
        ])
        .output()
        .await
        .expect("snapshot session");

    assert!(snapshot_output.status.success());
    let snapshot_str = String::from_utf8_lossy(&snapshot_output.stdout);
    assert!(snapshot_str.contains("Snapshot Buffer Marker"));

    let close_output = Command::new(cli_bin())
        .args([
            "--url",
            &base_url,
            "--token",
            token,
            "close",
            &session_id,
        ])
        .output()
        .await
        .expect("close session");

    assert!(close_output.status.success());

    let list_output = Command::new(cli_bin())
        .args(["--url", &base_url, "--token", token, "list", "--json"])
        .output()
        .await
        .expect("list session");

    let terminals: Vec<SessionInfo> =
        serde_json::from_slice(&list_output.stdout).expect("valid json terminal list");
    assert!(terminals.is_empty());

    server_handle.abort();
}

#[tokio::test]
async fn test_cli_interactive_input() {
    let app_state = AppState::new();
    let token = "cli-test-token-input";
    let auth_state = AuthState::with_token(token);

    let (addr, server_handle) = start_server(app_state, auth_state, 0)
        .await
        .expect("start test server");

    let base_url = format!("http://{}", addr);

    let spawn_output = Command::new(cli_bin())
        .args(["--url", &base_url, "--token", token, "spawn"])
        .output()
        .await
        .expect("spawn session");

    let session_id = String::from_utf8_lossy(&spawn_output.stdout)
        .trim()
        .to_string();

    tokio::time::sleep(Duration::from_millis(500)).await;

    let input_output = Command::new(cli_bin())
        .args([
            "--url",
            &base_url,
            "--token",
            token,
            "input",
            &session_id,
            "echo 'interactive_verified'",
        ])
        .output()
        .await
        .expect("send input");

    assert!(input_output.status.success());

    tokio::time::sleep(Duration::from_millis(600)).await;

    let snapshot_output = Command::new(cli_bin())
        .args([
            "--url",
            &base_url,
            "--token",
            token,
            "snapshot",
            &session_id,
        ])
        .output()
        .await
        .expect("get snapshot");

    assert!(snapshot_output.status.success());
    let snapshot_str = String::from_utf8_lossy(&snapshot_output.stdout);
    assert!(snapshot_str.contains("interactive_verified"));

    server_handle.abort();
}
