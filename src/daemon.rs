use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use sysinfo::{ProcessesToUpdate, System};

use crate::notify;
use crate::util::format_duration;

/// A session that ended after living at least `min_duration`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SshEvent {
    pub pid: u32,
    pub cmdline: String,
    pub duration_secs: u64,
}

struct Session {
    first_seen: Instant,
    cmdline: String,
}

/// Pure state machine tracking SSH sessions across poll snapshots.
///
/// `observe()` receives the set of currently running SSH processes and
/// returns events for sessions that disappeared after living at least
/// `min_duration_secs`. Sessions shorter than the threshold are dropped
/// silently. A PID that reappears after its session ended is treated as a
/// fresh session (PID reuse).
pub struct SshTracker {
    sessions: HashMap<u32, Session>,
    min_duration_secs: u64,
}

impl SshTracker {
    pub fn new(min_duration_secs: u64) -> Self {
        Self {
            sessions: HashMap::new(),
            min_duration_secs,
        }
    }

    #[cfg(test)]
    pub fn tracked_count(&self) -> usize {
        self.sessions.len()
    }

    /// Update state with a new snapshot of `(pid, cmdline)` pairs.
    /// Returns end-events for sessions that vanished this cycle.
    pub fn observe(&mut self, now: Instant, running: &[(u32, String)]) -> Vec<SshEvent> {
        let mut events = Vec::new();
        let seen: std::collections::HashSet<u32> = running.iter().map(|(pid, _)| *pid).collect();

        // Register new sessions.
        for (pid, cmdline) in running {
            self.sessions.entry(*pid).or_insert_with(|| Session {
                first_seen: now,
                cmdline: cmdline.clone(),
            });
        }

        // Detect ended sessions.
        self.sessions.retain(|pid, session| {
            if seen.contains(pid) {
                true
            } else {
                let lived = now.saturating_duration_since(session.first_seen).as_secs();
                if lived >= self.min_duration_secs {
                    events.push(SshEvent {
                        pid: *pid,
                        cmdline: session.cmdline.clone(),
                        duration_secs: lived,
                    });
                }
                false
            }
        });

        events
    }
}

/// True if the executable looks like an SSH client.
pub fn is_ssh_process(exe: Option<&Path>) -> bool {
    let Some(name) = exe.and_then(|p| p.file_name()).and_then(|n| n.to_str()) else {
        return false;
    };
    matches!(name.to_ascii_lowercase().as_str(), "ssh" | "ssh.exe")
}

/// Collect the current `(pid, cmdline)` snapshot of SSH processes.
fn ssh_snapshot(sys: &System) -> Vec<(u32, String)> {
    sys.processes()
        .iter()
        .filter(|(_, proc)| is_ssh_process(proc.exe()))
        .map(|(pid, proc)| {
            let cmdline = proc
                .cmd()
                .iter()
                .map(|s| s.to_string_lossy())
                .collect::<Vec<_>>()
                .join(" ");
            (pid.as_u32(), cmdline)
        })
        .collect()
}

/// Run the daemon loop: poll the process table, notify on ended sessions.
pub fn run(min_duration_secs: u64, poll_interval_secs: u64, sound: bool) -> Result<()> {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .ok(); // Second handler registration failing is not fatal.

    let mut sys = System::new();
    let mut tracker = SshTracker::new(min_duration_secs);
    let interval = Duration::from_secs(poll_interval_secs.max(1));

    eprintln!(
        "ssh-done daemon: watching for SSH sessions >= {}s (poll every {}s). Ctrl+C to stop.",
        min_duration_secs,
        interval.as_secs()
    );

    while running.load(Ordering::SeqCst) {
        sys.refresh_processes(ProcessesToUpdate::All, true);
        let snapshot = ssh_snapshot(&sys);
        for event in tracker.observe(Instant::now(), &snapshot) {
            let tail = cmdline_tail(&event.cmdline);
            notify::send(
                "SSH session ended",
                &format!("{} after {}", tail, format_duration(event.duration_secs)),
                sound,
            );
        }
        std::thread::sleep(interval);
    }

    eprintln!("ssh-done daemon: stopped.");
    Ok(())
}

/// Trim the leading program path from a command line for notification text.
fn cmdline_tail(cmdline: &str) -> String {
    let trimmed = cmdline.trim();
    if trimmed.is_empty() {
        return "ssh".to_string();
    }
    // Keep the last ~3 args at most, joined.
    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    let start = parts.len().saturating_sub(3);
    parts[start..].join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(pids: &[(u32, &str)]) -> Vec<(u32, String)> {
        pids.iter()
            .map(|(pid, cmd)| (*pid, cmd.to_string()))
            .collect()
    }

    #[test]
    fn new_sessions_registered() {
        let mut t = SshTracker::new(10);
        let now = Instant::now();
        let events = t.observe(now, &snap(&[(101, "ssh host deploy")]));
        assert!(events.is_empty());
        assert_eq!(t.tracked_count(), 1);
    }

    #[test]
    fn below_threshold_dropped_silently() {
        let mut t = SshTracker::new(10);
        let now = Instant::now();
        t.observe(now, &snap(&[(101, "ssh host")]));
        // Session vanishes immediately -> lived 0s < 10s -> no event.
        let events = t.observe(now, &snap(&[]));
        assert!(events.is_empty());
        assert_eq!(t.tracked_count(), 0);
    }

    #[test]
    fn at_threshold_notifies() {
        let min = 10u64;
        let mut t = SshTracker::new(min);
        let now = Instant::now();
        // Pre-seed a session that started `min` seconds ago.
        t.sessions.insert(
            202,
            Session {
                first_seen: now.checked_sub(Duration::from_secs(min)).unwrap(),
                cmdline: "ssh backup@srv1 restic backup /".to_string(),
            },
        );
        let events = t.observe(now, &snap(&[]));
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].pid, 202);
        assert!(events[0].duration_secs >= min);
    }

    #[test]
    fn pid_reuse_treated_as_new_session() {
        let mut t = SshTracker::new(0);
        let now = Instant::now();
        t.observe(now, &snap(&[(303, "ssh old-host")]));
        let events = t.observe(now, &snap(&[]));
        assert_eq!(events.len(), 1); // min 0 -> immediate event
                                     // Same PID reappears with a different command -> fresh session.
        let events = t.observe(now, &snap(&[(303, "ssh new-host")]));
        assert!(events.is_empty());
        assert_eq!(t.tracked_count(), 1);
        let events = t.observe(now, &snap(&[]));
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].cmdline, "ssh new-host");
    }

    #[test]
    fn long_running_session_reports_real_duration() {
        let mut t = SshTracker::new(1);
        let start = Instant::now() - Duration::from_secs(65);
        t.sessions.insert(
            404,
            Session {
                first_seen: start,
                cmdline: "ssh prod migrate".to_string(),
            },
        );
        let events = t.observe(Instant::now(), &snap(&[]));
        assert_eq!(events.len(), 1);
        assert!(events[0].duration_secs >= 65);
    }

    #[test]
    fn ssh_exe_detection() {
        assert!(is_ssh_process(Some(Path::new("/usr/bin/ssh"))));
        assert!(is_ssh_process(Some(Path::new(
            "C:\\Windows\\System32\\OpenSSH\\ssh.exe"
        ))));
        assert!(is_ssh_process(Some(Path::new("/usr/bin/SSH"))));
        assert!(!is_ssh_process(Some(Path::new("/usr/bin/bash"))));
        assert!(!is_ssh_process(Some(Path::new("/usr/bin/sshd"))));
        assert!(!is_ssh_process(None));
    }

    #[test]
    fn cmdline_tail_keeps_last_args() {
        assert_eq!(
            cmdline_tail("/usr/bin/ssh user@host deploy.sh --slow"),
            "user@host deploy.sh --slow"
        );
        assert_eq!(cmdline_tail(""), "ssh");
        assert_eq!(cmdline_tail("ssh"), "ssh");
    }
}
