//! Cross-distro Linux discovery for TermCMD authentication tokens and API ports.
//!
//! Implements multi-tier resolution conforming to the XDG Base Directory
//! Specification with POSIX UID and temporary directory fallbacks.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

/// Resolved connection parameters for communicating with a TermCMD instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionInfo {
    pub base_url: String,
    pub token: String,
}

/// Resolves active TermCMD connection info using explicit overrides or Linux fallback chains.
pub fn resolve_connection(
    explicit_url: Option<&str>,
    explicit_port: Option<u16>,
    explicit_token: Option<&str>,
) -> Result<ConnectionInfo, String> {
    let token = match explicit_token {
        Some(t) if !t.trim().is_empty() => t.trim().to_string(),
        _ => resolve_token()?,
    };

    let base_url = if let Some(url) = explicit_url {
        let trimmed = url.trim().trim_end_matches('/');
        if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
            trimmed.to_string()
        } else {
            format!("http://{}", trimmed)
        }
    } else {
        let port = match explicit_port {
            Some(p) => p,
            None => resolve_port(),
        };
        format!("http://127.0.0.1:{}", port)
    };

    Ok(ConnectionInfo { base_url, token })
}

/// Discovers the active authentication token from environment, XDG runtime, config, or temp files.
pub fn resolve_token() -> Result<String, String> {
    if let Ok(tok) = std::env::var("TERMCMD_TOKEN") {
        let trimmed = tok.trim().to_string();
        if !trimmed.is_empty() {
            return Ok(trimmed);
        }
    }

    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        let p = PathBuf::from(runtime_dir).join("termcmd.token");
        if let Ok(tok) = fs::read_to_string(&p) {
            let trimmed = tok.trim().to_string();
            if !trimmed.is_empty() {
                return Ok(trimmed);
            }
        }
    }

    if let Ok(config_home) = std::env::var("XDG_CONFIG_HOME") {
        let p = PathBuf::from(config_home).join("termcmd").join("token");
        if let Ok(tok) = fs::read_to_string(&p) {
            let trimmed = tok.trim().to_string();
            if !trimmed.is_empty() {
                return Ok(trimmed);
            }
        }
    }

    if let Ok(home) = std::env::var("HOME") {
        let p = PathBuf::from(home).join(".config").join("termcmd").join("token");
        if let Ok(tok) = fs::read_to_string(&p) {
            let trimmed = tok.trim().to_string();
            if !trimmed.is_empty() {
                return Ok(trimmed);
            }
        }
    }

    #[cfg(unix)]
    {
        let uid = nix::unistd::getuid();
        let p = PathBuf::from(format!("/tmp/termcmd-{}/token", uid));
        if let Ok(tok) = fs::read_to_string(&p) {
            let trimmed = tok.trim().to_string();
            if !trimmed.is_empty() {
                return Ok(trimmed);
            }
        }
    }

    let p = std::env::temp_dir().join("termcmd.token");
    if let Ok(tok) = fs::read_to_string(&p) {
        let trimmed = tok.trim().to_string();
        if !trimmed.is_empty() {
            return Ok(trimmed);
        }
    }

    Err(
        "TermCMD token not found. Ensure TermCMD is running or provide TERMCMD_TOKEN.".to_string(),
    )
}

fn is_port_reachable(port: u16) -> bool {
    std::net::TcpStream::connect_timeout(
        &std::net::SocketAddr::from(([127, 0, 0, 1], port)),
        std::time::Duration::from_millis(60),
    )
    .is_ok()
}

/// Discovers the active API port from environment, XDG runtime, config, or temp files.
pub fn resolve_port() -> u16 {
    if let Ok(port_str) = std::env::var("TERMCMD_PORT") {
        if let Ok(port) = port_str.trim().parse::<u16>() {
            return port;
        }
    }

    let mut candidate_port: Option<u16> = None;

    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        let p = PathBuf::from(runtime_dir).join("termcmd.port");
        if let Ok(content) = fs::read_to_string(&p) {
            if let Ok(port) = content.trim().parse::<u16>() {
                candidate_port = Some(port);
            }
        }
    }

    if candidate_port.is_none() {
        if let Ok(config_home) = std::env::var("XDG_CONFIG_HOME") {
            let p = PathBuf::from(config_home).join("termcmd").join("port");
            if let Ok(content) = fs::read_to_string(&p) {
                if let Ok(port) = content.trim().parse::<u16>() {
                    candidate_port = Some(port);
                }
            }
        }
    }

    if candidate_port.is_none() {
        if let Ok(home) = std::env::var("HOME") {
            let p = PathBuf::from(home).join(".config").join("termcmd").join("port");
            if let Ok(content) = fs::read_to_string(&p) {
                if let Ok(port) = content.trim().parse::<u16>() {
                    candidate_port = Some(port);
                }
            }
        }
    }

    if candidate_port.is_none() {
        #[cfg(unix)]
        {
            let uid = nix::unistd::getuid();
            let p = PathBuf::from(format!("/tmp/termcmd-{}/port", uid));
            if let Ok(content) = fs::read_to_string(&p) {
                if let Ok(port) = content.trim().parse::<u16>() {
                    candidate_port = Some(port);
                }
            }
        }
    }

    if candidate_port.is_none() {
        let p = std::env::temp_dir().join("termcmd.port");
        if let Ok(content) = fs::read_to_string(&p) {
            if let Ok(port) = content.trim().parse::<u16>() {
                candidate_port = Some(port);
            }
        }
    }

    if let Some(port) = candidate_port {
        if is_port_reachable(port) {
            return port;
        }
    }

    if is_port_reachable(crate::api::DEFAULT_API_PORT) {
        return crate::api::DEFAULT_API_PORT;
    }

    candidate_port.unwrap_or(crate::api::DEFAULT_API_PORT)
}

/// Persists the active API port to standard discovery locations.
pub fn persist_port(port: u16) -> Option<PathBuf> {
    let port_str = port.to_string();
    let target_path = if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        PathBuf::from(runtime_dir).join("termcmd.port")
    } else if let Ok(home) = std::env::var("HOME") {
        let config_dir = PathBuf::from(home).join(".config").join("termcmd");
        let _ = fs::create_dir_all(&config_dir);
        config_dir.join("port")
    } else {
        std::env::temp_dir().join("termcmd.port")
    };

    if let Some(parent) = target_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o644);
    }

    if let Ok(mut file) = options.open(&target_path) {
        if file.write_all(port_str.as_bytes()).is_ok() {
            return Some(target_path);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_explicit_overrides_precedence() {
        let conn = resolve_connection(
            Some("127.0.0.1:9000"),
            Some(9000),
            Some("explicit_test_token"),
        )
        .expect("Failed to resolve connection");

        assert_eq!(conn.base_url, "http://127.0.0.1:9000");
        assert_eq!(conn.token, "explicit_test_token");
    }

    #[test]
    fn test_url_formatting_without_protocol() {
        let conn = resolve_connection(
            Some("localhost:8080/"),
            None,
            Some("token123"),
        )
        .expect("Failed to resolve connection");

        assert_eq!(conn.base_url, "http://localhost:8080");
    }
}
