//! Reclaim the HTTP listen port from a stale translator instance.

use log::{info, warn};
use std::{thread, time::Duration};

const RECLAIM_WAIT: Duration = Duration::from_millis(300);

/// Stop any process already listening on `port` (except this process).
pub fn reclaim(port: u16) {
    #[cfg(unix)]
    reclaim_unix(port);
    #[cfg(windows)]
    reclaim_windows(port);
}

#[cfg(unix)]
fn reclaim_unix(port: u16) {
    use std::process::Command;

    let me = std::process::id();
    let Ok(out) = Command::new("lsof")
        .args([
            "-nP",
            &format!("-iTCP:{port}"),
            "-sTCP:LISTEN",
            "-t",
        ])
        .output()
    else {
        return;
    };

    if !out.status.success() {
        return;
    }

    let pids: Vec<u32> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| line.trim().parse().ok())
        .filter(|pid| *pid != me)
        .collect();

    if pids.is_empty() {
        return;
    }

    for pid in &pids {
        info!("Port {port} in use by PID {pid} — stopping previous instance");
        terminate_pid(*pid);
    }
    thread::sleep(RECLAIM_WAIT);
}

#[cfg(unix)]
fn terminate_pid(pid: u32) {
    use std::process::Command;

    let _ = Command::new("kill").arg(pid.to_string()).status();
    thread::sleep(Duration::from_millis(150));
    if process_alive(pid) {
        warn!("PID {pid} did not exit, sending SIGKILL");
        let _ = Command::new("kill").arg("-9").arg(pid.to_string()).status();
        thread::sleep(Duration::from_millis(100));
    }
}

#[cfg(unix)]
fn process_alive(pid: u32) -> bool {
    use std::process::Command;

    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(windows)]
fn reclaim_windows(port: u16) {
    use std::process::Command;

    let me = std::process::id();
    let Ok(out) = Command::new("netstat")
        .args(["-ano"])
        .output()
    else {
        return;
    };

    let needle = format!(":{port}");
    let mut pids = Vec::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        if !line.contains("LISTENING") || !line.contains(&needle) {
            continue;
        }
        if let Some(pid) = line.split_whitespace().last().and_then(|s| s.parse::<u32>().ok()) {
            if pid != me {
                pids.push(pid);
            }
        }
    }

    let mut any = false;
    for pid in pids {
        info!("Port {port} in use by PID {pid} — stopping previous instance");
        let _ = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F"])
            .status();
        any = true;
    }
    if any {
        thread::sleep(RECLAIM_WAIT);
    }
}
