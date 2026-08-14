//! Automated integration test suite for TermCMD Backend Core & PTY Engine.

use std::path::PathBuf;
use std::time::Duration;
use termcmd_core::pty::{PtyManager, SessionConfig, SessionEvent, SessionState};
use tokio::time::timeout;

#[tokio::test]
async fn test_pty_spawn_and_prompt() {
    let manager = PtyManager::new();
    let config = SessionConfig {
        shell: Some("/bin/bash".to_string()),
        inject_hooks: true,
        ..Default::default()
    };

    let session = manager.spawn_session(config).expect("spawn session");
    assert!(session.is_alive());
    assert!(session.info().pid.is_some());

    let mut rx = session.subscribe();

    session.write_command("echo hello_termcmd").expect("write command");

    let mut found_output = false;
    let timeout_duration = Duration::from_secs(5);

    let start = std::time::Instant::now();
    while start.elapsed() < timeout_duration {
        if let Ok(Ok(event)) = timeout(Duration::from_millis(500), rx.recv()).await {
            if let SessionEvent::Output(bytes) = event {
                let text = String::from_utf8_lossy(&bytes);
                if text.contains("hello_termcmd") {
                    found_output = true;
                    break;
                }
            }
        }
    }

    assert!(found_output, "Expected echo output in session stream");
}

#[tokio::test]
async fn test_osc133_exit_code_capture() {
    let manager = PtyManager::new();
    let config = SessionConfig {
        shell: Some("/bin/bash".to_string()),
        inject_hooks: true,
        ..Default::default()
    };

    let session = manager.spawn_session(config).expect("spawn session");
    let mut rx = session.subscribe();

    tokio::time::sleep(Duration::from_millis(200)).await;

    session.write_command("(exit 42)").expect("write exit command");

    let mut captured_exit_code = None;
    let timeout_duration = Duration::from_secs(5);
    let start = std::time::Instant::now();

    while start.elapsed() < timeout_duration {
        if let Ok(Ok(event)) = timeout(Duration::from_millis(500), rx.recv()).await {
            if let SessionEvent::Osc(termcmd_core::pty::OscEvent::CommandFinished { exit_code }) = event {
                if exit_code == 42 {
                    captured_exit_code = Some(exit_code);
                    break;
                }
            }
        }
    }

    assert_eq!(captured_exit_code, Some(42));
}

#[tokio::test]
async fn test_osc7_cwd_tracking() {
    let manager = PtyManager::new();
    let config = SessionConfig {
        shell: Some("/bin/bash".to_string()),
        inject_hooks: true,
        ..Default::default()
    };

    let session = manager.spawn_session(config).expect("spawn session");
    let mut rx = session.subscribe();

    tokio::time::sleep(Duration::from_millis(200)).await;

    session.write_command("cd /tmp").expect("write cd command");

    let mut updated_cwd = None;
    let timeout_duration = Duration::from_secs(5);
    let start = std::time::Instant::now();

    while start.elapsed() < timeout_duration {
        if let Ok(Ok(event)) = timeout(Duration::from_millis(500), rx.recv()).await {
            if let SessionEvent::CwdChanged(cwd) = event {
                if cwd == PathBuf::from("/tmp") {
                    updated_cwd = Some(cwd);
                    break;
                }
            }
        }
    }

    assert_eq!(updated_cwd, Some(PathBuf::from("/tmp")));
    assert_eq!(session.cwd(), PathBuf::from("/tmp"));
}

#[tokio::test]
async fn test_sigint_process_isolation() {
    let manager = PtyManager::new();
    let config = SessionConfig {
        shell: Some("/bin/bash".to_string()),
        inject_hooks: true,
        ..Default::default()
    };

    let session = manager.spawn_session(config).expect("spawn session");
    let mut rx = session.subscribe();

    tokio::time::sleep(Duration::from_millis(200)).await;

    session.write_command("sleep 30").expect("write sleep command");

    tokio::time::sleep(Duration::from_millis(300)).await;

    session.send_sigint().expect("send sigint");

    tokio::time::sleep(Duration::from_millis(200)).await;

    session.write_command("echo STATUS:$?").expect("write status check");

    let mut captured_status = false;
    let timeout_duration = Duration::from_secs(5);
    let start = std::time::Instant::now();

    while start.elapsed() < timeout_duration {
        if let Ok(Ok(event)) = timeout(Duration::from_millis(500), rx.recv()).await {
            if let SessionEvent::Output(bytes) = event {
                let text = String::from_utf8_lossy(&bytes);
                if text.contains("STATUS:130") {
                    captured_status = true;
                    break;
                }
            }
        }
    }

    assert!(captured_status, "Process should be interrupted with exit code 130");
    assert!(session.is_alive(), "Parent shell must remain alive after interrupt");
}

#[tokio::test]
async fn test_pty_resize() {
    let manager = PtyManager::new();
    let session = manager.spawn_session(SessionConfig::default()).expect("spawn session");

    assert_eq!(session.info().cols, 120);
    assert_eq!(session.info().rows, 35);

    session.resize(140, 45).expect("resize pty");

    assert_eq!(session.info().cols, 140);
    assert_eq!(session.info().rows, 45);
}

#[tokio::test]
async fn test_pty_manager_lifecycle_and_respawn() {
    let manager = PtyManager::new();
    let session = manager
        .spawn_session(SessionConfig {
            id: "term_test_lifecycle".to_string(),
            cwd: Some(PathBuf::from("/tmp")),
            ..Default::default()
        })
        .expect("spawn session");

    assert_eq!(manager.session_count(), 1);
    assert_eq!(session.cwd(), PathBuf::from("/tmp"));

    let sessions = manager.list_sessions();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].id, "term_test_lifecycle");

    tokio::time::sleep(Duration::from_millis(200)).await;

    session.write_command("exit").expect("write exit");

    let timeout_duration = Duration::from_secs(5);
    let start = std::time::Instant::now();
    while start.elapsed() < timeout_duration {
        if let SessionState::Terminated { .. } = session.state() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    assert!(matches!(session.state(), SessionState::Terminated { .. }));

    let respawned = manager.respawn_session("term_test_lifecycle").expect("respawn session");
    assert!(respawned.is_alive());
    assert_eq!(respawned.cwd(), PathBuf::from("/tmp"));

    manager.close_session("term_test_lifecycle").expect("close session");
    assert_eq!(manager.session_count(), 0);
}
