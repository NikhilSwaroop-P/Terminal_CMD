//! Multi-session agent concurrency stress testing for TermCMD.
//!
//! Validates parallel PTY lifecycle, stream isolation, OSC 133 exit code extraction,
//! and 409 Conflict guard under high concurrent load.

use std::collections::HashSet;
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use reqwest::Client;
use serde_json::Value;

use termcmd_core::api::auth::AuthState;
use termcmd_core::api::models::CreateTerminalRequest;
use termcmd_core::pty::session::SessionConfig;
use termcmd_core::state::AppState;

#[tokio::test]
async fn test_10x_concurrent_agent_sessions() {
    let app_state = AppState::new();
    let auth_token = "concurrency-test-token-777";
    let auth_state = AuthState::with_token(auth_token);

    let (addr, server_handle) = termcmd_core::api::start_server(app_state.clone(), auth_state, 0)
        .await
        .expect("start server");

    let client = Client::new();
    let base_url = format!("http://{}", addr);

    let session_count = 10;
    let mut session_ids = Vec::with_capacity(session_count);

    for i in 0..session_count {
        let req_body = CreateTerminalRequest {
            title: Some(format!("Agent Session {}", i)),
            cwd: None,
            shell: Some("/bin/bash".to_string()),
            cols: Some(100),
            rows: Some(30),
            env: None,
        };

        let res = client
            .post(format!("{}/api/v1/terminals", base_url))
            .header("Authorization", format!("Bearer {}", auth_token))
            .json(&req_body)
            .send()
            .await
            .expect("create terminal");

        assert_eq!(res.status(), reqwest::StatusCode::CREATED);
        let val: Value = res.json().await.expect("parse json");
        let id = val["id"].as_str().expect("id string").to_string();
        session_ids.push(id);
    }

    assert_eq!(session_ids.len(), session_count);
    let unique_ids: HashSet<_> = session_ids.iter().cloned().collect();
    assert_eq!(unique_ids.len(), session_count);

    tokio::time::sleep(Duration::from_millis(2000)).await;

    let mut join_handles = Vec::with_capacity(session_count);

    for (idx, sid) in session_ids.iter().enumerate() {
        let client_c = client.clone();
        let sid_c = sid.clone();
        let base_url_c = base_url.clone();
        let token_c = auth_token.to_string();
        let target_exit_code = idx as i32;

        let handle = tokio::spawn(async move {
            let unique_marker = format!("ISOLATION_CHECK_SESSION_{}_{}", idx, sid_c);
            let command = format!("echo '{}' && (exit {})", unique_marker, target_exit_code);

            let res = client_c
                .post(format!("{}/api/v1/terminals/{}/exec", base_url_c, sid_c))
                .header("Authorization", format!("Bearer {}", token_c))
                .json(&serde_json::json!({
                    "command": command,
                    "stripAnsi": true,
                    "timeoutSeconds": 15
                }))
                .send()
                .await
                .expect("send exec request");

            assert_eq!(res.status(), reqwest::StatusCode::OK);

            let mut stream = res.bytes_stream();
            let mut full_body = String::new();
            let mut extracted_exit_code: Option<i32> = None;

            let start = Instant::now();
            while let Some(chunk_res) = stream.next().await {
                if let Ok(chunk) = chunk_res {
                    let text = String::from_utf8_lossy(&chunk);
                    full_body.push_str(&text);

                    if text.contains("event: done") || full_body.contains("event: done") {
                        if let Some(pos) = full_body.find("event: done") {
                            let after = &full_body[pos..];
                            if let Some(data_line) = after.lines().find(|l| l.starts_with("data:")) {
                                let json_str = data_line.trim_start_matches("data:").trim();
                                if let Ok(val) = serde_json::from_str::<Value>(json_str) {
                                    if let Some(code) = val.get("exitCode").and_then(|c| c.as_i64()) {
                                        extracted_exit_code = Some(code as i32);
                                    }
                                }
                            }
                        }
                        break;
                    }
                }
                if start.elapsed() > Duration::from_secs(12) {
                    break;
                }
            }

            (idx, sid_c, unique_marker, extracted_exit_code, full_body)
        });

        join_handles.push(handle);
    }

    let mut results = Vec::new();
    for handle in join_handles {
        let res = handle.await.expect("join task");
        results.push(res);
    }

    assert_eq!(results.len(), session_count);

    for (idx, sid, marker, exit_code_opt, body) in &results {
        assert!(
            body.contains(marker),
            "Session {} ({}) missing its marker in body: {}",
            idx,
            sid,
            body
        );

        for (other_idx, _, other_marker, _, _) in &results {
            if other_idx != idx {
                assert!(
                    !body.contains(other_marker),
                    "Cross-talk detected! Session {} contained marker of session {}: {}",
                    idx,
                    other_idx,
                    other_marker
                );
            }
        }

        assert_eq!(
            *exit_code_opt,
            Some(*idx as i32),
            "Session {} expected exit code {}, got {:?}",
            idx,
            idx,
            exit_code_opt
        );
    }

    for sid in &session_ids {
        let res = client
            .delete(format!("{}/api/v1/terminals/{}", base_url, sid))
            .header("Authorization", format!("Bearer {}", auth_token))
            .send()
            .await
            .expect("delete terminal");
        assert_eq!(res.status(), reqwest::StatusCode::OK);
    }

    server_handle.abort();
}

#[tokio::test]
async fn test_concurrency_conflict_guard_and_overlap_rejection() {
    let app_state = AppState::new();
    let auth_token = "conflict-test-token-888";
    let auth_state = AuthState::with_token(auth_token);

    let (addr, server_handle) = termcmd_core::api::start_server(app_state.clone(), auth_state, 0)
        .await
        .expect("start server");

    let client = Client::new();
    let base_url = format!("http://{}", addr);

    let session = app_state
        .pty_manager
        .spawn_session(SessionConfig {
            shell: Some("/bin/bash".to_string()),
            ..SessionConfig::default()
        })
        .expect("spawn session");

    tokio::time::sleep(Duration::from_millis(2000)).await;

    let client_1 = client.clone();
    let base_1 = base_url.clone();
    let sid_1 = session.id.clone();
    let token_1 = auth_token.to_string();

    let exec_task = tokio::spawn(async move {
        client_1
            .post(format!("{}/api/v1/terminals/{}/exec", base_1, sid_1))
            .header("Authorization", format!("Bearer {}", token_1))
            .json(&serde_json::json!({
                "command": "sleep 3",
                "timeoutSeconds": 10
            }))
            .send()
            .await
    });

    tokio::time::sleep(Duration::from_millis(300)).await;

    let conflict_res = client
        .post(format!("{}/api/v1/terminals/{}/exec", base_url, session.id))
        .header("Authorization", format!("Bearer {}", auth_token))
        .json(&serde_json::json!({
            "command": "echo 'Should be rejected'",
            "timeoutSeconds": 5
        }))
        .send()
        .await
        .expect("send overlapping exec");

    assert_eq!(conflict_res.status(), reqwest::StatusCode::CONFLICT);
    let conflict_json: Value = conflict_res.json().await.expect("parse conflict json");
    assert_eq!(conflict_json["error"].as_str().unwrap(), "TerminalBusy");

    let _ = exec_task.await;

    server_handle.abort();
}
