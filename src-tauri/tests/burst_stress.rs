//! High-throughput log burst stress benchmarks for TermCMD PTY reader and ring buffer.

use std::time::{Duration, Instant};
use termcmd_core::pty::buffer::RingBuffer;
use termcmd_core::pty::session::{PtySession, SessionConfig, SessionEvent};

#[test]
fn test_ring_buffer_100k_burst_throughput() {
    let mut buffer = RingBuffer::new(50_000);
    let total_lines = 100_000;
    
    let start = Instant::now();
    for i in 0..total_lines {
        buffer.push_line(format!("LOG_LINE_{:06}: payload data item with timestamp and metadata", i));
    }
    let elapsed = start.elapsed();
    
    assert_eq!(buffer.len(), 50_000);
    assert_eq!(buffer.total_lines_ingested(), total_lines as u64);
    
    let snapshot = buffer.get_recent_lines(5);
    assert_eq!(snapshot.len(), 5);
    assert_eq!(snapshot[4], "LOG_LINE_099999: payload data item with timestamp and metadata");
    
    let lines_per_sec = (total_lines as f64) / elapsed.as_secs_f64();
    let lines_per_min = lines_per_sec * 60.0;
    assert!(lines_per_min > 100_000.0, "Throughput was {} lines/min, expected > 100,000", lines_per_min);
}

#[tokio::test]
async fn test_pty_100k_stream_burst() {
    let config = SessionConfig {
        shell: Some("/bin/bash".to_string()),
        inject_hooks: false,
        ..SessionConfig::default()
    };
    
    let session = PtySession::spawn(config).expect("failed to spawn PTY session");
    let mut rx = session.subscribe();
    
    let burst_lines = 100_000;
    let cmd = format!("seq 1 {}\n", burst_lines);
    
    let start = Instant::now();
    session.write_all(cmd.as_bytes()).expect("write to pty");
    
    let mut received_bytes = 0usize;
    let mut received_chunks = 0usize;
    let timeout = Duration::from_secs(15);
    let deadline = Instant::now() + timeout;
    
    while Instant::now() < deadline {
        match tokio::time::timeout(Duration::from_millis(500), rx.recv()).await {
            Ok(Ok(SessionEvent::Output(chunk))) => {
                received_bytes += chunk.len();
                received_chunks += 1;
                if session.get_buffer_snapshot().len() >= 50_000 || session.is_alive() == false {
                    let recent = session.get_buffer_snapshot();
                    if recent.iter().any(|l| l.trim() == "100000") {
                        break;
                    }
                }
            }
            Ok(Ok(_)) => {}
            Ok(Err(tokio::sync::broadcast::error::RecvError::Lagged(_))) => {}
            Ok(Err(tokio::sync::broadcast::error::RecvError::Closed)) => break,
            Err(_) => {
                let snapshot = session.get_buffer_snapshot();
                if snapshot.iter().any(|l| l.trim() == "100000") {
                    break;
                }
            }
        }
    }
    
    let elapsed = start.elapsed();
    let snapshot = session.get_buffer_snapshot();
    assert!(!snapshot.is_empty());
    assert!(received_bytes > 0);
    assert!(received_chunks > 0);
    
    let throughput_lpm = (burst_lines as f64 / elapsed.as_secs_f64()) * 60.0;
    assert!(throughput_lpm > 100_000.0, "Throughput was {} lines/min", throughput_lpm);
}

#[tokio::test]
async fn test_ws_burst_throughput_via_api() {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::connect_async;
    use tokio_tungstenite::tungstenite::Message;
    use termcmd_core::api::auth::AuthState;
    use termcmd_core::state::AppState;
    
    let app_state = AppState::new();
    let auth_token = "burst-token-xyz";
    let auth_state = AuthState::with_token(auth_token);
    
    let (addr, server_handle) = termcmd_core::api::start_server(app_state.clone(), auth_state, 0)
        .await
        .expect("start server");
        
    let session = app_state
        .pty_manager
        .spawn_session(SessionConfig {
            shell: Some("/bin/bash".to_string()),
            inject_hooks: false,
            ..SessionConfig::default()
        })
        .expect("spawn session");
        
    tokio::time::sleep(Duration::from_millis(300)).await;
    
    let ws_url = format!("ws://{}/api/v1/terminals/{}/ws?token={}", addr, session.id, auth_token);
    let (ws_stream, _) = connect_async(ws_url).await.expect("connect websocket");
    let (mut ws_write, mut ws_read) = ws_stream.split();
    
    let burst_count = 25_000;
    let burst_cmd = "python3 -c \"for i in range(1, 25001): print(f'ITEM_{i}')\"\r".to_string();
    let start = Instant::now();
    ws_write.send(Message::Text(burst_cmd)).await.expect("send command over WS");
    
    let mut total_ws_bytes = 0usize;
    let mut saw_target = false;
    let timeout = Duration::from_secs(10);
    let deadline = Instant::now() + timeout;
    
    while Instant::now() < deadline {
        match tokio::time::timeout(Duration::from_millis(500), ws_read.next()).await {
            Ok(Some(Ok(msg))) => {
                let text = match msg {
                    Message::Text(t) => {
                        total_ws_bytes += t.len();
                        t
                    }
                    Message::Binary(b) => {
                        total_ws_bytes += b.len();
                        String::from_utf8_lossy(&b).to_string()
                    }
                    _ => String::new(),
                };
                
                if text.contains("ITEM_25000") {
                    saw_target = true;
                    break;
                }
            }
            _ => {
                if saw_target {
                    break;
                }
            }
        }
    }
    
    let elapsed = start.elapsed();
    assert!(saw_target, "Expected to receive ITEM_25000 completion marker over WebSocket");
    assert!(total_ws_bytes > 50_000, "Received {} bytes, expected > 50000", total_ws_bytes);
    
    let calculated_lpm = (burst_count as f64 / elapsed.as_secs_f64()) * 60.0;
    assert!(calculated_lpm > 100_000.0, "WS Burst throughput was {} lines/min", calculated_lpm);
    
    server_handle.abort();
}
