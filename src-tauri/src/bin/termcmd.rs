//! TermCMD Universal Agent CLI Binary.
//!
//! Provides zero-configuration CLI access to local TermCMD PTY instances with
//! real-time Server-Sent Events output streaming and POSIX signal forwarding.

use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use futures_util::StreamExt;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use reqwest::Client;
use termcmd_core::api::discovery::{resolve_connection, ConnectionInfo};
use termcmd_core::api::models::{
    ApiErrorResponse, CreateTerminalRequest, CreateTerminalResponse, ExecDonePayload, ExecRequest,
    InputRequest, KillRequest, ListTerminalsResponse, ResizeRequest, TerminalDetailResponse,
};

#[derive(Parser)]
#[command(
    name = "termcmd",
    author = "TermCMD Team",
    version = "0.1.0",
    about = "TermCMD Agent CLI & Multi-Terminal Controller"
)]
struct Cli {
    #[arg(long, global = true, help = "Explicit TermCMD API port")]
    port: Option<u16>,

    #[arg(long, global = true, help = "Explicit TermCMD authentication token")]
    token: Option<String>,

    #[arg(long, global = true, help = "Explicit TermCMD API base URL (e.g. http://127.0.0.1:7890)")]
    url: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Spawn a new persistent terminal session on the desktop canvas
    Spawn {
        #[arg(long, help = "Optional title for the terminal window")]
        title: Option<String>,

        #[arg(long, help = "Initial working directory path")]
        cwd: Option<PathBuf>,

        #[arg(long, help = "Custom shell binary path (e.g. /bin/bash, /bin/zsh)")]
        shell: Option<String>,

        #[arg(long, default_value_t = 120, help = "Initial column width")]
        cols: u16,

        #[arg(long, default_value_t = 35, help = "Initial row height")]
        rows: u16,
    },

    /// List active terminal sessions
    List {
        #[arg(long, help = "Output formatted JSON array")]
        json: bool,
    },

    /// Execute a command in a terminal and stream stdout in real-time
    Exec {
        #[arg(help = "Terminal session ID")]
        id: String,

        #[arg(help = "Command string to execute")]
        command: String,

        #[arg(long, help = "Preserve raw ANSI escape sequences")]
        raw: bool,

        #[arg(long, default_value_t = 300, help = "Execution timeout in seconds")]
        timeout: u64,

        #[arg(long = "env", value_name = "KEY=VALUE", help = "Environment variables")]
        env: Vec<String>,
    },

    /// Send standard input or interactive prompt answer to a terminal
    Input {
        #[arg(help = "Terminal session ID")]
        id: String,

        #[arg(help = "Data or response string")]
        data: String,

        #[arg(long, help = "Send raw bytes without automatically appending newline")]
        raw: bool,
    },

    /// Send a POSIX signal to the terminal's foreground process group
    Kill {
        #[arg(help = "Terminal session ID")]
        id: String,

        #[arg(long, default_value = "SIGINT", help = "POSIX signal name (SIGINT, SIGTERM, SIGKILL)")]
        signal: String,
    },

    /// Close and delete a terminal session from the canvas
    Close {
        #[arg(help = "Terminal session ID")]
        id: String,
    },

    /// Retrieve the scrollback buffer snapshot
    Snapshot {
        #[arg(help = "Terminal session ID")]
        id: String,

        #[arg(long, help = "Maximum number of trailing lines to fetch")]
        lines: Option<usize>,
    },

    /// Update terminal dimensions and broadcast SIGWINCH
    Resize {
        #[arg(help = "Terminal session ID")]
        id: String,

        #[arg(long, required = true, help = "New column width")]
        cols: u16,

        #[arg(long, required = true, help = "New row height")]
        rows: u16,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let conn = match resolve_connection(
        cli.url.as_deref(),
        cli.port,
        cli.token.as_deref(),
    ) {
        Ok(c) => c,
        Err(err) => {
            eprintln!("Error: {}", err);
            std::process::exit(1);
        }
    };

    let client = create_http_client(&conn);

    match cli.command {
        Commands::Spawn {
            title,
            cwd,
            shell,
            cols,
            rows,
        } => {
            handle_spawn(&client, &conn, title, cwd, shell, cols, rows).await;
        }
        Commands::List { json } => {
            handle_list(&client, &conn, json).await;
        }
        Commands::Exec {
            id,
            command,
            raw,
            timeout,
            env,
        } => {
            handle_exec(&client, &conn, id, command, raw, timeout, env).await;
        }
        Commands::Input { id, data, raw } => {
            handle_input(&client, &conn, id, data, raw).await;
        }
        Commands::Kill { id, signal } => {
            handle_kill(&client, &conn, id, signal).await;
        }
        Commands::Close { id } => {
            handle_close(&client, &conn, id).await;
        }
        Commands::Snapshot { id, lines } => {
            handle_snapshot(&client, &conn, id, lines).await;
        }
        Commands::Resize { id, cols, rows } => {
            handle_resize(&client, &conn, id, cols, rows).await;
        }
    }
}

fn create_http_client(conn: &ConnectionInfo) -> Client {
    let mut headers = HeaderMap::new();
    if let Ok(auth_val) = HeaderValue::from_str(&format!("Bearer {}", conn.token)) {
        headers.insert(AUTHORIZATION, auth_val);
    }
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    Client::builder()
        .default_headers(headers)
        .build()
        .unwrap_or_default()
}

async fn handle_spawn(
    client: &Client,
    conn: &ConnectionInfo,
    title: Option<String>,
    cwd: Option<PathBuf>,
    shell: Option<String>,
    cols: u16,
    rows: u16,
) {
    let req = CreateTerminalRequest {
        title,
        cwd,
        shell,
        cols: Some(cols),
        rows: Some(rows),
        env: None,
    };

    let url = format!("{}/api/v1/terminals", conn.base_url);
    let resp = match client.post(&url).json(&req).send().await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to connect to TermCMD server at {}: {}", conn.base_url, e);
            std::process::exit(1);
        }
    };

    if !resp.status().is_success() {
        print_error_and_exit(resp).await;
    }

    match resp.json::<CreateTerminalResponse>().await {
        Ok(body) => {
            println!("{}", body.id);
        }
        Err(err) => {
            eprintln!("Failed to parse response: {}", err);
            std::process::exit(1);
        }
    }
}

async fn handle_list(client: &Client, conn: &ConnectionInfo, json_mode: bool) {
    let url = format!("{}/api/v1/terminals", conn.base_url);
    let resp = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to connect to TermCMD server at {}: {}", conn.base_url, e);
            std::process::exit(1);
        }
    };

    if !resp.status().is_success() {
        print_error_and_exit(resp).await;
    }

    match resp.json::<ListTerminalsResponse>().await {
        Ok(body) => {
            if json_mode {
                if let Ok(json_str) = serde_json::to_string_pretty(&body.terminals) {
                    println!("{}", json_str);
                    return;
                }
            }

            if body.terminals.is_empty() {
                println!("No active terminal sessions found.");
                return;
            }

            println!("{:<20} {:<24} {:<8} {:<12} {}", "ID", "TITLE", "PID", "STATE", "CWD");
            println!("{}", "-".repeat(84));
            for t in &body.terminals {
                let pid_str = t.pid.map(|p| p.to_string()).unwrap_or_else(|| "-".to_string());
                let state_str = format!("{:?}", t.state);
                let cwd_str = t.cwd.display().to_string();
                println!(
                    "{:<20} {:<24} {:<8} {:<12} {}",
                    t.id, t.title, pid_str, state_str, cwd_str
                );
            }
        }
        Err(err) => {
            eprintln!("Failed to parse response: {}", err);
            std::process::exit(1);
        }
    }
}

async fn handle_exec(
    client: &Client,
    conn: &ConnectionInfo,
    id: String,
    command: String,
    raw: bool,
    timeout: u64,
    env_args: Vec<String>,
) {
    let mut env_map = HashMap::new();
    for item in env_args {
        if let Some((k, v)) = item.split_once('=') {
            env_map.insert(k.to_string(), v.to_string());
        }
    }

    let req = ExecRequest {
        command,
        strip_ansi: Some(!raw),
        timeout_seconds: Some(timeout),
        env: if env_map.is_empty() { None } else { Some(env_map) },
    };

    let url = format!("{}/api/v1/terminals/{}/exec", conn.base_url, id);
    let resp = match client
        .post(&url)
        .header(ACCEPT, "text/event-stream")
        .json(&req)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to connect to TermCMD server at {}: {}", conn.base_url, e);
            std::process::exit(1);
        }
    };

    if !resp.status().is_success() {
        print_error_and_exit(resp).await;
    }

    let mut stream = resp.bytes_stream();
    let mut parser = SseParser::new();
    let mut final_exit_code: i32 = 0;

    while let Some(chunk_res) = stream.next().await {
        match chunk_res {
            Ok(chunk) => {
                let chunk_str = String::from_utf8_lossy(&chunk);
                let frames = parser.feed(&chunk_str);
                for frame in frames {
                    match frame.event.as_str() {
                        "stdout" => {
                            let mut stdout = std::io::stdout().lock();
                            let _ = stdout.write_all(frame.data.as_bytes());
                            let _ = stdout.flush();
                        }
                        "done" => {
                            if let Ok(done) = serde_json::from_str::<ExecDonePayload>(&frame.data) {
                                std::process::exit(done.exit_code);
                            } else {
                                std::process::exit(0);
                            }
                        }
                        _ => {}
                    }
                }
            }
            Err(err) => {
                eprintln!("Streaming error: {}", err);
                std::process::exit(1);
            }
        }
    }

    for frame in parser.flush() {
        if frame.event == "stdout" {
            let mut stdout = std::io::stdout().lock();
            let _ = stdout.write_all(frame.data.as_bytes());
            let _ = stdout.flush();
        } else if frame.event == "done" {
            if let Ok(done) = serde_json::from_str::<ExecDonePayload>(&frame.data) {
                final_exit_code = done.exit_code;
            }
        }
    }

    std::process::exit(final_exit_code);
}

async fn handle_input(
    client: &Client,
    conn: &ConnectionInfo,
    id: String,
    data: String,
    raw: bool,
) {
    let payload_data = if !raw && !data.ends_with('\n') && !data.ends_with('\r') {
        format!("{}\n", data)
    } else {
        data
    };

    let req = InputRequest { data: payload_data };
    let url = format!("{}/api/v1/terminals/{}/input", conn.base_url, id);
    let resp = match client.post(&url).json(&req).send().await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to connect to TermCMD server at {}: {}", conn.base_url, e);
            std::process::exit(1);
        }
    };

    if !resp.status().is_success() {
        print_error_and_exit(resp).await;
    }
}

async fn handle_kill(client: &Client, conn: &ConnectionInfo, id: String, signal: String) {
    let req = KillRequest {
        signal: Some(signal.clone()),
    };
    let url = format!("{}/api/v1/terminals/{}/kill", conn.base_url, id);
    let resp = match client.post(&url).json(&req).send().await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to connect to TermCMD server at {}: {}", conn.base_url, e);
            std::process::exit(1);
        }
    };

    if !resp.status().is_success() {
        print_error_and_exit(resp).await;
    }

    println!("Signal {} sent to terminal session {}", signal, id);
}

async fn handle_close(client: &Client, conn: &ConnectionInfo, id: String) {
    let url = format!("{}/api/v1/terminals/{}", conn.base_url, id);
    let resp = match client.delete(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to connect to TermCMD server at {}: {}", conn.base_url, e);
            std::process::exit(1);
        }
    };

    if !resp.status().is_success() {
        print_error_and_exit(resp).await;
    }

    println!("Terminal session {} closed", id);
}

async fn handle_snapshot(
    client: &Client,
    conn: &ConnectionInfo,
    id: String,
    lines: Option<usize>,
) {
    let url = format!("{}/api/v1/terminals/{}", conn.base_url, id);
    let resp = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to connect to TermCMD server at {}: {}", conn.base_url, e);
            std::process::exit(1);
        }
    };

    if !resp.status().is_success() {
        print_error_and_exit(resp).await;
    }

    match resp.json::<TerminalDetailResponse>().await {
        Ok(body) => {
            let buffer = body.buffer;
            let slice = if let Some(n) = lines {
                if n < buffer.len() {
                    &buffer[buffer.len() - n..]
                } else {
                    &buffer[..]
                }
            } else {
                &buffer[..]
            };

            for line in slice {
                println!("{}", line);
            }
        }
        Err(err) => {
            eprintln!("Failed to parse response: {}", err);
            std::process::exit(1);
        }
    }
}

async fn handle_resize(client: &Client, conn: &ConnectionInfo, id: String, cols: u16, rows: u16) {
    let req = ResizeRequest { cols, rows };
    let url = format!("{}/api/v1/terminals/{}/resize", conn.base_url, id);
    let resp = match client.post(&url).json(&req).send().await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to connect to TermCMD server at {}: {}", conn.base_url, e);
            std::process::exit(1);
        }
    };

    if !resp.status().is_success() {
        print_error_and_exit(resp).await;
    }

    println!("Resized terminal session {} to {}x{}", id, cols, rows);
}

async fn print_error_and_exit(resp: reqwest::Response) -> ! {
    let status = resp.status();
    let body_text = resp.text().await.unwrap_or_default();
    if let Ok(api_err) = serde_json::from_str::<ApiErrorResponse>(&body_text) {
        eprintln!("Error ({}): {}", api_err.error, api_err.message);
    } else {
        eprintln!("Error {}: {}", status, body_text);
    }
    std::process::exit(1);
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SseFrame {
    event: String,
    data: String,
}

struct SseParser {
    buffer: String,
    current_event: String,
    current_data: String,
}

impl SseParser {
    fn new() -> Self {
        Self {
            buffer: String::new(),
            current_event: String::from("message"),
            current_data: String::new(),
        }
    }

    fn feed(&mut self, chunk: &str) -> Vec<SseFrame> {
        self.buffer.push_str(chunk);
        let mut frames = Vec::new();

        while let Some(pos) = self.buffer.find('\n') {
            let mut line = self.buffer[..pos].to_string();
            self.buffer.drain(..pos + 1);

            if line.ends_with('\r') {
                line.pop();
            }

            if line.is_empty() {
                if !self.current_data.is_empty() || self.current_event != "message" {
                    frames.push(SseFrame {
                        event: std::mem::replace(&mut self.current_event, String::from("message")),
                        data: std::mem::take(&mut self.current_data),
                    });
                }
            } else if let Some(stripped) = line.strip_prefix("event:") {
                self.current_event = stripped.trim().to_string();
            } else if let Some(stripped) = line.strip_prefix("data:") {
                let d = stripped.strip_prefix(' ').unwrap_or(stripped);
                if !self.current_data.is_empty() {
                    self.current_data.push('\n');
                }
                self.current_data.push_str(d);
            }
        }

        frames
    }

    fn flush(&mut self) -> Vec<SseFrame> {
        let mut frames = Vec::new();
        if !self.current_data.is_empty() || self.current_event != "message" {
            frames.push(SseFrame {
                event: std::mem::replace(&mut self.current_event, String::from("message")),
                data: std::mem::take(&mut self.current_data),
            });
        }
        frames
    }
}
