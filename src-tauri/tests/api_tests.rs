//! Integration tests for the TermCMD Local Agent API, REST, SSE, and WebSocket endpoints.

use std::collections::HashMap;
use std::time::Duration;

use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;

use termcmd_core::api::ansi::optimize_tokens;
use termcmd_core::api::auth::AuthState;
use termcmd_core::api::models::{CreateTerminalRequest, ExecRequest, InputRequest, KillRequest, ResizeRequest};
use termcmd_core::api::server::create_router;
use termcmd_core::pty::session::SessionConfig;
use termcmd_core::state::AppState;

#[tokio::test]
async fn test_api_auth_token_guard() {
    let app_state = AppState::new();
    let test_token = "test-secret-token-12345";
    let auth_state = AuthState::with_token(test_token);
    let app = create_router(app_state, auth_state);

    let req_unauth = Request::builder()
        .uri("/api/v1/terminals")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res_unauth = app.clone().oneshot(req_unauth).await.unwrap();
    assert_eq!(res_unauth.status(), StatusCode::UNAUTHORIZED);

    let req_bad_token = Request::builder()
        .uri("/api/v1/terminals")
        .method("GET")
        .header(header::AUTHORIZATION, "Bearer wrong-token")
        .body(Body::empty())
        .unwrap();
    let res_bad_token = app.clone().oneshot(req_bad_token).await.unwrap();
    assert_eq!(res_bad_token.status(), StatusCode::UNAUTHORIZED);

    let req_valid_header = Request::builder()
        .uri("/api/v1/terminals")
        .method("GET")
        .header(header::AUTHORIZATION, format!("Bearer {}", test_token))
        .body(Body::empty())
        .unwrap();
    let res_valid_header = app.clone().oneshot(req_valid_header).await.unwrap();
    assert_eq!(res_valid_header.status(), StatusCode::OK);

    let req_valid_query = Request::builder()
        .uri(format!("/api/v1/terminals?token={}", test_token))
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res_valid_query = app.clone().oneshot(req_valid_query).await.unwrap();
    assert_eq!(res_valid_query.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_terminal_crud_lifecycle() {
    let app_state = AppState::new();
    let test_token = "crud-token-999";
    let auth_state = AuthState::with_token(test_token);
    let app = create_router(app_state.clone(), auth_state);

    let create_payload = CreateTerminalRequest {
        title: Some("Integration Test Terminal".to_string()),
        cwd: None,
        shell: None,
        cols: Some(100),
        rows: Some(30),
        env: Some(HashMap::new()),
    };

    let req_create = Request::builder()
        .uri("/api/v1/terminals")
        .method("POST")
        .header(header::AUTHORIZATION, format!("Bearer {}", test_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&create_payload).unwrap()))
        .unwrap();

    let res_create = app.clone().oneshot(req_create).await.unwrap();
    assert_eq!(res_create.status(), StatusCode::CREATED);

    let body_bytes = to_bytes(res_create.into_body(), usize::MAX).await.unwrap();
    let json_create: Value = serde_json::from_slice(&body_bytes).unwrap();
    let term_id = json_create["id"].as_str().unwrap().to_string();
    assert!(term_id.starts_with("term_"));
    assert_eq!(json_create["title"].as_str().unwrap(), "Integration Test Terminal");

    let req_list = Request::builder()
        .uri("/api/v1/terminals")
        .method("GET")
        .header(header::AUTHORIZATION, format!("Bearer {}", test_token))
        .body(Body::empty())
        .unwrap();

    let res_list = app.clone().oneshot(req_list).await.unwrap();
    assert_eq!(res_list.status(), StatusCode::OK);
    let list_bytes = to_bytes(res_list.into_body(), usize::MAX).await.unwrap();
    let json_list: Value = serde_json::from_slice(&list_bytes).unwrap();
    let terminals_arr = json_list["terminals"].as_array().unwrap();
    assert!(terminals_arr.iter().any(|t| t["id"] == term_id));

    let req_get = Request::builder()
        .uri(format!("/api/v1/terminals/{}", term_id))
        .method("GET")
        .header(header::AUTHORIZATION, format!("Bearer {}", test_token))
        .body(Body::empty())
        .unwrap();

    let res_get = app.clone().oneshot(req_get).await.unwrap();
    assert_eq!(res_get.status(), StatusCode::OK);
    let get_bytes = to_bytes(res_get.into_body(), usize::MAX).await.unwrap();
    let json_get: Value = serde_json::from_slice(&get_bytes).unwrap();
    assert_eq!(json_get["terminal"]["id"].as_str().unwrap(), term_id);

    let req_delete = Request::builder()
        .uri(format!("/api/v1/terminals/{}", term_id))
        .method("DELETE")
        .header(header::AUTHORIZATION, format!("Bearer {}", test_token))
        .body(Body::empty())
        .unwrap();

    let res_delete = app.clone().oneshot(req_delete).await.unwrap();
    assert_eq!(res_delete.status(), StatusCode::OK);

    let req_get_after = Request::builder()
        .uri(format!("/api/v1/terminals/{}", term_id))
        .method("GET")
        .header(header::AUTHORIZATION, format!("Bearer {}", test_token))
        .body(Body::empty())
        .unwrap();

    let res_get_after = app.clone().oneshot(req_get_after).await.unwrap();
    assert_eq!(res_get_after.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_pty_resize_endpoint() {
    let app_state = AppState::new();
    let test_token = "resize-token-123";
    let auth_state = AuthState::with_token(test_token);
    let app = create_router(app_state.clone(), auth_state);

    let session = app_state
        .pty_manager
        .spawn_session(SessionConfig::default())
        .unwrap();

    let resize_payload = ResizeRequest {
        cols: 140,
        rows: 40,
    };

    let req_resize = Request::builder()
        .uri(format!("/api/v1/terminals/{}/resize", session.id))
        .method("POST")
        .header(header::AUTHORIZATION, format!("Bearer {}", test_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&resize_payload).unwrap()))
        .unwrap();

    let res_resize = app.clone().oneshot(req_resize).await.unwrap();
    assert_eq!(res_resize.status(), StatusCode::OK);

    let info = session.info();
    assert_eq!(info.cols, 140);
    assert_eq!(info.rows, 40);
}

#[tokio::test]
async fn test_pty_input_and_signal_endpoint() {
    let app_state = AppState::new();
    let test_token = "input-sig-token-123";
    let auth_state = AuthState::with_token(test_token);
    let app = create_router(app_state.clone(), auth_state);

    let session = app_state
        .pty_manager
        .spawn_session(SessionConfig::default())
        .unwrap();

    let input_payload = InputRequest {
        data: "echo test\n".to_string(),
    };

    let req_input = Request::builder()
        .uri(format!("/api/v1/terminals/{}/input", session.id))
        .method("POST")
        .header(header::AUTHORIZATION, format!("Bearer {}", test_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&input_payload).unwrap()))
        .unwrap();

    let res_input = app.clone().oneshot(req_input).await.unwrap();
    assert_eq!(res_input.status(), StatusCode::OK);

    let kill_payload = KillRequest {
        signal: Some("SIGINT".to_string()),
    };

    let req_kill = Request::builder()
        .uri(format!("/api/v1/terminals/{}/kill", session.id))
        .method("POST")
        .header(header::AUTHORIZATION, format!("Bearer {}", test_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&kill_payload).unwrap()))
        .unwrap();

    let res_kill = app.clone().oneshot(req_kill).await.unwrap();
    assert_eq!(res_kill.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_concurrency_conflict_409() {
    let app_state = AppState::new();
    let test_token = "concurrency-token-123";
    let auth_state = AuthState::with_token(test_token);
    let app = create_router(app_state.clone(), auth_state);

    let session = app_state
        .pty_manager
        .spawn_session(SessionConfig::default())
        .unwrap();

    session.write_command("sleep 30").unwrap();

    let exec_payload = ExecRequest {
        command: "ls -la".to_string(),
        strip_ansi: Some(true),
        timeout_seconds: Some(10),
        env: None,
    };

    let req_exec = Request::builder()
        .uri(format!("/api/v1/terminals/{}/exec", session.id))
        .method("POST")
        .header(header::AUTHORIZATION, format!("Bearer {}", test_token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&exec_payload).unwrap()))
        .unwrap();

    let res_exec = app.clone().oneshot(req_exec).await.unwrap();
    assert_eq!(res_exec.status(), StatusCode::CONFLICT);

    let body_bytes = to_bytes(res_exec.into_body(), usize::MAX).await.unwrap();
    let json_err: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(json_err["error"].as_str().unwrap(), "TerminalBusy");
}

#[tokio::test]
async fn test_sse_exec_streaming_success() {
    let app_state = AppState::new();
    let test_token = "sse-token-test";
    let auth_state = AuthState::with_token(test_token);

    let (addr, server_handle) =
        termcmd_core::api::start_server(app_state.clone(), auth_state, 0).await.unwrap();

    let session = app_state
        .pty_manager
        .spawn_session(SessionConfig {
            shell: Some("/bin/bash".to_string()),
            ..SessionConfig::default()
        })
        .unwrap();

    let wait_start = std::time::Instant::now();
    while session.state() != termcmd_core::pty::SessionState::Idle && wait_start.elapsed() < Duration::from_secs(3) {
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    let client = reqwest::Client::new();
    let exec_url = format!("http://{}/api/v1/terminals/{}/exec", addr, session.id);

    let res = client
        .post(&exec_url)
        .header("Authorization", format!("Bearer {}", test_token))
        .json(&serde_json::json!({
            "command": "echo 'Hello TermCMD'",
            "stripAnsi": true,
            "timeoutSeconds": 10
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), reqwest::StatusCode::OK);

    let mut stream = res.bytes_stream();
    use futures_util::StreamExt;

    let mut accumulated_body = String::new();
    while let Some(chunk) = stream.next().await {
        if let Ok(bytes) = chunk {
            let s = String::from_utf8_lossy(&bytes);
            accumulated_body.push_str(&s);
            if accumulated_body.contains("event: done") {
                break;
            }
        }
    }

    assert!(accumulated_body.contains("event: start"));
    assert!(accumulated_body.contains("event: done"));
    assert!(accumulated_body.contains("Hello TermCMD"));

    server_handle.abort();
}

#[tokio::test]
async fn test_sse_exec_exit_code_propagation() {
    let app_state = AppState::new();
    let test_token = "sse-exit-token";
    let auth_state = AuthState::with_token(test_token);

    let (addr, server_handle) =
        termcmd_core::api::start_server(app_state.clone(), auth_state, 0).await.unwrap();

    let session = app_state
        .pty_manager
        .spawn_session(SessionConfig {
            shell: Some("/bin/bash".to_string()),
            ..SessionConfig::default()
        })
        .unwrap();

    tokio::time::sleep(Duration::from_millis(2200)).await;

    let client = reqwest::Client::new();
    let exec_url = format!("http://{}/api/v1/terminals/{}/exec", addr, session.id);

    let res = client
        .post(&exec_url)
        .header("Authorization", format!("Bearer {}", test_token))
        .json(&serde_json::json!({
            "command": "(exit 7)",
            "stripAnsi": true,
            "timeoutSeconds": 10
        }))
        .send()
        .await
        .unwrap();

    let status = res.status();
    if status != reqwest::StatusCode::OK {
        let err_body = res.text().await.unwrap();
        panic!("Non-200 Status: {}, Body: {}", status, err_body);
    }

    let mut stream = res.bytes_stream();
    use futures_util::StreamExt;

    let mut accumulated_body = String::new();
    while let Some(chunk) = stream.next().await {
        if let Ok(bytes) = chunk {
            let s = String::from_utf8_lossy(&bytes);
            accumulated_body.push_str(&s);
            if accumulated_body.contains("event: done") {
                break;
            }
        }
    }

    assert!(accumulated_body.contains("event: done"), "Body: {}", accumulated_body);
    assert!(accumulated_body.contains("\"exitCode\":7") || accumulated_body.contains("\"exitCode\": 7"), "Body: {}", accumulated_body);

    server_handle.abort();
}

#[tokio::test]
async fn test_ansi_stripper_token_efficiency() {
    let noisy_build_output = "\x1b[1m\x1b[32m    Compiling\x1b[0m termcmd-core v0.1.0 (/\x1b[35mhome\x1b[0m/user)\r\n\x1b[1m\x1b[32m     Finished\x1b[0m `dev` profile [unoptimized + debuginfo] in 0.42s\r\n";
    let (cleaned, raw_len, stripped_len) = optimize_tokens(noisy_build_output);

    assert!(!cleaned.contains("\x1b"));
    assert!(cleaned.contains("Compiling termcmd-core v0.1.0 (/home/user)"));
    assert!(cleaned.contains("Finished `dev` profile"));
    assert!(stripped_len < raw_len);
}

#[tokio::test]
async fn test_ws_terminal_streaming() {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::connect_async;
    use tokio_tungstenite::tungstenite::Message;

    let app_state = AppState::new();
    let test_token = "ws-stream-token";
    let auth_state = AuthState::with_token(test_token);

    let (addr, server_handle) =
        termcmd_core::api::start_server(app_state.clone(), auth_state, 0).await.unwrap();

    let session = app_state
        .pty_manager
        .spawn_session(SessionConfig {
            shell: Some("/bin/bash".to_string()),
            ..SessionConfig::default()
        })
        .unwrap();

    tokio::time::sleep(Duration::from_millis(1500)).await;

    let ws_url = format!("ws://{}/api/v1/terminals/{}/ws?token={}", addr, session.id, test_token);
    let (ws_stream, _) = connect_async(ws_url).await.unwrap();
    let (mut ws_write, mut ws_read) = ws_stream.split();

    ws_write
        .send(Message::Text("echo 'WS_STREAM_OK'\n".to_string()))
        .await
        .unwrap();

    let mut found = false;
    let timeout = tokio::time::sleep(Duration::from_secs(5));
    tokio::pin!(timeout);

    loop {
        tokio::select! {
            _ = &mut timeout => break,
            msg = ws_read.next() => {
                if let Some(Ok(m)) = msg {
                    let text = match m {
                        Message::Text(t) => t,
                        Message::Binary(b) => String::from_utf8_lossy(&b).to_string(),
                        _ => String::new(),
                    };
                    if text.contains("WS_STREAM_OK") {
                        found = true;
                        break;
                    }
                }
            }
        }
    }

    assert!(found);
    server_handle.abort();
}

#[tokio::test]
async fn test_prompt_waiting_event_detection() {
    use futures_util::StreamExt;

    let app_state = AppState::new();
    let test_token = "prompt-wait-token";
    let auth_state = AuthState::with_token(test_token);

    let (addr, server_handle) =
        termcmd_core::api::start_server(app_state.clone(), auth_state, 0).await.unwrap();

    let session = app_state
        .pty_manager
        .spawn_session(SessionConfig {
            shell: Some("/bin/bash".to_string()),
            ..SessionConfig::default()
        })
        .unwrap();

    tokio::time::sleep(Duration::from_millis(2200)).await;

    let client = reqwest::Client::new();
    let exec_url = format!("http://{}/api/v1/terminals/{}/exec", addr, session.id);

    let res = client
        .post(&exec_url)
        .header("Authorization", format!("Bearer {}", test_token))
        .json(&serde_json::json!({
            "command": "printf 'Do you want to continue? [y/N] '; sleep 3",
            "stripAnsi": true,
            "timeoutSeconds": 6
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), reqwest::StatusCode::OK);

    let mut stream = res.bytes_stream();
    let mut received_prompt_waiting = false;

    let timeout = tokio::time::sleep(Duration::from_secs(5));
    tokio::pin!(timeout);

    loop {
        tokio::select! {
            _ = &mut timeout => break,
            chunk = stream.next() => {
                match chunk {
                    Some(Ok(bytes)) => {
                        let s = String::from_utf8_lossy(&bytes);
                        if s.contains("event: prompt_waiting") {
                            received_prompt_waiting = true;
                            break;
                        }
                    }
                    _ => break,
                }
            }
        }
    }

    assert!(received_prompt_waiting);
    server_handle.abort();
}

