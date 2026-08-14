//! Persistent PTY Session lifecycle and process supervisor.
//!
//! Handles master/slave terminal forking, asynchronous stdout reading,
//! OSC semantic parsing, process group signal dispatching, and dynamic resizing.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use parking_lot::{Mutex, RwLock};
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtyPair, PtySize};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::pty::buffer::RingBuffer;
use crate::pty::hooks::{create_init_environment, ShellInit, ShellType};
use crate::pty::osc::{OscEvent, OscParser};

/// Current execution state of a PTY session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum SessionState {
    /// Terminal is at prompt, waiting for user or agent input.
    Idle,
    /// Terminal is actively executing a command.
    Running { command: Option<String> },
    /// Child process has exited.
    Terminated { exit_code: Option<i32> },
}

/// Metadata snapshot describing a PTY session.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub id: String,
    pub title: String,
    pub cwd: PathBuf,
    pub shell: String,
    pub pid: Option<u32>,
    pub cols: u16,
    pub rows: u16,
    pub state: SessionState,
    pub active_command: Option<String>,
    pub command_started_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Broadcast event emitted across the session event bus.
#[derive(Debug, Clone)]
pub enum SessionEvent {
    /// Raw terminal byte chunk.
    Output(Vec<u8>),
    /// Semantic OSC event parsed from stream.
    Osc(OscEvent),
    /// State transition (Idle, Running, Terminated).
    StateChanged(SessionState),
    /// Working directory updated.
    CwdChanged(PathBuf),
    /// Dimension resized.
    Resized { cols: u16, rows: u16 },
    /// Process exited.
    Terminated { exit_code: Option<i32> },
}

/// Configuration options for spawning a new PTY session.
#[derive(Debug, Clone)]
pub struct SessionConfig {
    pub id: String,
    pub title: Option<String>,
    pub cwd: Option<PathBuf>,
    pub shell: Option<String>,
    pub cols: u16,
    pub rows: u16,
    pub env: HashMap<String, String>,
    pub inject_hooks: bool,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            title: None,
            cwd: None,
            shell: None,
            cols: 120,
            rows: 35,
            env: HashMap::new(),
            inject_hooks: true,
        }
    }
}

/// Represents an active, managed PTY session.
pub struct PtySession {
    pub id: String,
    title: RwLock<String>,
    cwd: RwLock<PathBuf>,
    shell: String,
    cols: RwLock<u16>,
    rows: RwLock<u16>,
    state: RwLock<SessionState>,
    active_command: RwLock<Option<String>>,
    command_started_at: RwLock<Option<DateTime<Utc>>>,
    pid: Option<u32>,
    created_at: DateTime<Utc>,
    master: Arc<Mutex<Box<dyn MasterPty + Send>>>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    buffer: Arc<RwLock<RingBuffer>>,
    event_tx: broadcast::Sender<SessionEvent>,
    is_alive: Arc<AtomicBool>,
    _init_env: Option<ShellInit>,
}

impl PtySession {
    /// Spawns a new PTY session and launches background reader tasks.
    pub fn spawn(config: SessionConfig) -> std::io::Result<Arc<Self>> {
        let pty_system = native_pty_system();
        let size = PtySize {
            rows: config.rows,
            cols: config.cols,
            pixel_width: 0,
            pixel_height: 0,
        };

        let PtyPair { master, slave } = pty_system
            .openpty(size)
            .map_err(|e| std::io::Error::other(e.to_string()))?;

        let shell_path = config.shell.clone().unwrap_or_else(|| {
            if Path::new("/usr/bin/fish").exists() {
                "/usr/bin/fish".to_string()
            } else if Path::new("/bin/fish").exists() {
                "/bin/fish".to_string()
            } else {
                std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string())
            }
        });
        let shell_type = ShellType::detect(&shell_path);

        let initial_cwd = config
            .cwd
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")));

        let mut cmd = CommandBuilder::new(&shell_path);
        cmd.cwd(&initial_cwd);
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        cmd.env("LANG", "en_US.UTF-8");

        for (k, v) in &config.env {
            cmd.env(k, v);
        }

        let init_env = if config.inject_hooks {
            match create_init_environment(shell_type)? {
                Some(ShellInit::BashFile(script_file)) => {
                    cmd.arg("--rcfile");
                    cmd.arg(script_file.path());
                    cmd.arg("-i");
                    Some(ShellInit::BashFile(script_file))
                }
                Some(ShellInit::ZshDir(temp_dir)) => {
                    if let Ok(orig_zdotdir) = std::env::var("ZDOTDIR") {
                        cmd.env("TERMCMD_ORIG_ZDOTDIR", orig_zdotdir);
                    }
                    cmd.env("ZDOTDIR", temp_dir.path());
                    cmd.arg("-i");
                    Some(ShellInit::ZshDir(temp_dir))
                }
                Some(ShellInit::FishFile(script_file)) => {
                    cmd.arg("--init-command");
                    cmd.arg(format!("source {}", script_file.path().display()));
                    cmd.arg("-i");
                    Some(ShellInit::FishFile(script_file))
                }
                None => None,
            }
        } else {
            None
        };

        let child = slave
            .spawn_command(cmd)
            .map_err(|e| std::io::Error::other(e.to_string()))?;

        let pid = child.process_id();

        let reader = master
            .try_clone_reader()
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        let writer = master
            .take_writer()
            .map_err(|e| std::io::Error::other(e.to_string()))?;

        let (event_tx, _) = broadcast::channel(4096);
        let buffer = Arc::new(RwLock::new(RingBuffer::default()));
        let is_alive = Arc::new(AtomicBool::new(true));

        let session = Arc::new(Self {
            id: config.id.clone(),
            title: RwLock::new(config.title.unwrap_or_else(|| format!("Terminal {}", &config.id[..config.id.len().min(8)]))),
            cwd: RwLock::new(initial_cwd),
            shell: shell_path,
            cols: RwLock::new(config.cols),
            rows: RwLock::new(config.rows),
            state: RwLock::new(SessionState::Idle),
            active_command: RwLock::new(None),
            command_started_at: RwLock::new(None),
            pid,
            created_at: Utc::now(),
            master: Arc::new(Mutex::new(master)),
            writer: Arc::new(Mutex::new(writer)),
            buffer,
            event_tx: event_tx.clone(),
            is_alive: is_alive.clone(),
            _init_env: init_env,
        });

        Self::start_reader_loop(
            session.clone(),
            reader,
            event_tx.clone(),
            is_alive.clone(),
        );

        Self::start_exit_monitor(session.clone(), child, is_alive);

        Ok(session)
    }

    fn start_reader_loop(
        session: Arc<Self>,
        mut reader: Box<dyn Read + Send>,
        event_tx: broadcast::Sender<SessionEvent>,
        is_alive: Arc<AtomicBool>,
    ) {
        std::thread::Builder::new()
            .name(format!("pty-reader-{}", session.id))
            .spawn(move || {
                let mut buf = [0u8; 4096];
                let mut osc_parser = OscParser::new();

                while is_alive.load(Ordering::Relaxed) {
                    match reader.read(&mut buf) {
                        Ok(0) => {
                            break;
                        }
                        Ok(n) => {
                            let chunk = &buf[..n];

                            session.buffer.write().push_chunk(chunk);
                            let _ = event_tx.send(SessionEvent::Output(chunk.to_vec()));

                            let osc_events = osc_parser.parse_chunk(chunk);
                            for osc_ev in osc_events {
                                {
                                    let mut state_guard = session.state.write();
                                    if !matches!(*state_guard, SessionState::Terminated { .. }) {
                                        match &osc_ev {
                                            OscEvent::PromptStart | OscEvent::CommandFinished { .. } => {
                                                *state_guard = SessionState::Idle;
                                                *session.active_command.write() = None;
                                                *session.command_started_at.write() = None;
                                                let _ = event_tx.send(SessionEvent::StateChanged(SessionState::Idle));
                                            }
                                            OscEvent::OutputStart => {
                                                let current_cmd = session.active_command.read().clone();
                                                let new_state = SessionState::Running { command: current_cmd };
                                                *state_guard = new_state.clone();
                                                let _ = event_tx.send(SessionEvent::StateChanged(new_state));
                                            }
                                            OscEvent::CommandStart => {}
                                            OscEvent::CwdChanged(new_cwd) => {
                                                *session.cwd.write() = new_cwd.clone();
                                                let _ = event_tx.send(SessionEvent::CwdChanged(new_cwd.clone()));
                                            }
                                        }
                                    }
                                }
                                let _ = event_tx.send(SessionEvent::Osc(osc_ev));
                            }
                        }
                        Err(_) => {
                            break;
                        }
                    }
                }
            })
            .expect("spawn pty reader thread");
    }

    fn start_exit_monitor(
        session: Arc<Self>,
        mut child: Box<dyn portable_pty::Child + Send>,
        is_alive: Arc<AtomicBool>,
    ) {
        std::thread::Builder::new()
            .name(format!("pty-monitor-{}", session.id))
            .spawn(move || {
                let status = child.wait();
                is_alive.store(false, Ordering::Relaxed);

                let exit_code = status.ok().map(|s| s.exit_code() as i32);
                let terminated_state = SessionState::Terminated { exit_code };

                *session.state.write() = terminated_state.clone();
                *session.active_command.write() = None;
                *session.command_started_at.write() = None;

                let _ = session.event_tx.send(SessionEvent::Terminated { exit_code });
                let _ = session.event_tx.send(SessionEvent::StateChanged(terminated_state));
            })
            .expect("spawn pty monitor thread");
    }

    /// Writes raw input bytes to the PTY stdin.
    pub fn write_all(&self, data: &[u8]) -> std::io::Result<()> {
        let mut writer = self.writer.lock();
        writer.write_all(data)?;
        writer.flush()?;
        Ok(())
    }

    /// Writes a command string with trailing newline and tracks command activity.
    pub fn write_command(&self, command: &str) -> std::io::Result<()> {
        let trimmed = command.trim();
        if !trimmed.is_empty() {
            *self.active_command.write() = Some(trimmed.to_string());
            *self.command_started_at.write() = Some(Utc::now());
            *self.state.write() = SessionState::Running { command: Some(trimmed.to_string()) };
        }

        let mut full_cmd = command.to_string();
        if !full_cmd.ends_with('\n') {
            full_cmd.push('\n');
        }
        self.write_all(full_cmd.as_bytes())
    }

    /// Resizes the PTY dimensions and notifies kernel/slaves with `SIGWINCH`.
    pub fn resize(&self, cols: u16, rows: u16) -> std::io::Result<()> {
        let size = PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        };

        self.master
            .lock()
            .resize(size)
            .map_err(|e| std::io::Error::other(e.to_string()))?;

        *self.cols.write() = cols;
        *self.rows.write() = rows;

        let _ = self.event_tx.send(SessionEvent::Resized { cols, rows });
        Ok(())
    }

    /// Sends an interrupt signal (SIGINT / Ctrl+C) to the terminal foreground process.
    pub fn send_sigint(&self) -> std::io::Result<()> {
        self.write_all(&[0x03])
    }

    /// Sends an arbitrary POSIX signal to the child process group.
    pub fn send_signal(&self, sig: Signal) -> std::io::Result<()> {
        if let Some(pid_val) = self.pid {
            signal::killpg(Pid::from_raw(pid_val as i32), sig)
                .map_err(|e| std::io::Error::other(e.to_string()))?;
        }
        Ok(())
    }

    /// Returns session information snapshot.
    pub fn info(&self) -> SessionInfo {
        SessionInfo {
            id: self.id.clone(),
            title: self.title.read().clone(),
            cwd: self.cwd.read().clone(),
            shell: self.shell.clone(),
            pid: self.pid,
            cols: *self.cols.read(),
            rows: *self.rows.read(),
            state: self.state.read().clone(),
            active_command: self.active_command.read().clone(),
            command_started_at: *self.command_started_at.read(),
            created_at: self.created_at,
        }
    }

    /// Returns current working directory.
    pub fn cwd(&self) -> PathBuf {
        self.cwd.read().clone()
    }

    /// Returns current session state.
    pub fn state(&self) -> SessionState {
        self.state.read().clone()
    }

    /// Returns snapshot clone of the output buffer.
    pub fn get_buffer_snapshot(&self) -> Vec<String> {
        self.buffer.read().get_snapshot()
    }

    /// Subscribes to the live broadcast event stream for this session.
    pub fn subscribe(&self) -> broadcast::Receiver<SessionEvent> {
        self.event_tx.subscribe()
    }

    /// Returns true if the child process is still running.
    pub fn is_alive(&self) -> bool {
        self.is_alive.load(Ordering::Relaxed)
    }

    /// Clears the internal scrollback buffer.
    pub fn clear_buffer(&self) {
        self.buffer.write().clear();
    }
}
