//! Process group signal and zombie process cleanup audit for TermCMD.
//!
//! Verifies that process group signal dispatching and terminal termination
//! completely reap all descendant subprocesses without leaving defunct or zombie processes.

use std::fs;
use std::path::Path;
use std::time::Duration;

use nix::sys::signal::Signal;
use termcmd_core::pty::session::{PtySession, SessionConfig};

fn get_descendant_pids(parent_pid: u32) -> Vec<u32> {
    let mut descendants = Vec::new();
    if let Ok(entries) = fs::read_dir("/proc") {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(file_name) = path.file_name().and_then(|f| f.to_str()) {
                if let Ok(pid) = file_name.parse::<u32>() {
                    let stat_path = path.join("stat");
                    if let Ok(stat) = fs::read_to_string(&stat_path) {
                        if let Some(close_paren) = stat.rfind(')') {
                            let rest = &stat[close_paren + 1..].trim();
                            let parts: Vec<&str> = rest.split_whitespace().collect();
                            if parts.len() >= 3 {
                                if let (Ok(ppid), Ok(pgrp)) = (parts[1].parse::<u32>(), parts[2].parse::<u32>()) {
                                    if (ppid == parent_pid || pgrp == parent_pid) && pid != parent_pid {
                                        descendants.push(pid);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    descendants
}

fn check_pid_is_reaped_or_nonexistent(pid_val: u32) -> bool {
    let stat_path = format!("/proc/{}/stat", pid_val);
    if !Path::new(&stat_path).exists() {
        return true;
    }
    if let Ok(stat_content) = fs::read_to_string(&stat_path) {
        if let Some(close_paren) = stat_content.rfind(')') {
            let rest = stat_content[close_paren + 1..].trim();
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if let Some(state) = parts.first() {
                return *state != "Z";
            }
        }
    }
    true
}

#[tokio::test]
async fn test_sigint_process_group_cleanup() {
    let config = SessionConfig {
        shell: Some("/bin/bash".to_string()),
        inject_hooks: false,
        ..SessionConfig::default()
    };
    
    let session = PtySession::spawn(config).expect("spawn session");
    let main_pid = session.info().pid.expect("valid child pid");
    tokio::time::sleep(Duration::from_millis(300)).await;
    
    session.write_command("sleep 50").expect("write command");
    let mut descendants_before = Vec::new();
    for _ in 0..20 {
        descendants_before = get_descendant_pids(main_pid);
        if !descendants_before.is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    
    assert!(!descendants_before.is_empty(), "Expected at least 1 child process under parent PID {}", main_pid);
    
    session.send_sigint().expect("send sigint");
    tokio::time::sleep(Duration::from_millis(600)).await;
    
    let descendants_after = get_descendant_pids(main_pid);
    assert_eq!(descendants_after.len(), 0, "Descendant processes should be terminated by SIGINT, found {:?}", descendants_after);
}

#[tokio::test]
async fn test_terminal_deletion_reaps_child_process_tree() {
    let config = SessionConfig {
        shell: Some("/bin/bash".to_string()),
        inject_hooks: false,
        ..SessionConfig::default()
    };
    
    let session = PtySession::spawn(config).expect("spawn session");
    let main_pid = session.info().pid.expect("valid child pid");
    
    tokio::time::sleep(Duration::from_millis(300)).await;
    
    session.write_command("sh -c 'sleep 70 & sleep 80 & wait'").expect("write command");
    tokio::time::sleep(Duration::from_millis(600)).await;
    
    let descendants_before = get_descendant_pids(main_pid);
    assert!(!descendants_before.is_empty(), "Expected nested sleep tree processes");
    
    session.send_signal(Signal::SIGKILL).expect("send sigkill");
    tokio::time::sleep(Duration::from_millis(600)).await;
    
    let is_reaped = check_pid_is_reaped_or_nonexistent(main_pid);
    assert!(is_reaped, "Main child PID {} should be cleanly reaped", main_pid);
    
    for d_pid in descendants_before {
        assert!(check_pid_is_reaped_or_nonexistent(d_pid), "Descendant PID {} must be reaped", d_pid);
    }
}

#[tokio::test]
async fn test_zombie_defunct_status_audit() {
    let config = SessionConfig {
        shell: Some("/bin/bash".to_string()),
        inject_hooks: false,
        ..SessionConfig::default()
    };
    
    let session = PtySession::spawn(config).expect("spawn session");
    let child_pid = session.info().pid.expect("valid child pid");
    
    tokio::time::sleep(Duration::from_millis(150)).await;
    
    session.write_command("exit").expect("write exit");
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    let is_reaped = check_pid_is_reaped_or_nonexistent(child_pid);
    assert!(is_reaped, "Exited PID {} must not remain in zombie state", child_pid);
}
