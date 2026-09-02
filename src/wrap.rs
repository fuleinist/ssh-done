use std::process::Command;
use std::time::Instant;

use anyhow::{Context, Result};

use crate::notify;
use crate::util::format_duration;

/// Run `command` as a child process with inherited stdio, then notify on exit.
///
/// The child's exit code is propagated to our exit code.
/// If `min_duration_secs` is set and the command finished faster, no
/// notification is sent (useful to skip trivially fast runs).
pub fn run(command: &[String], min_duration_secs: Option<u64>, sound: bool) -> Result<i32> {
    let (program, args) = command
        .split_first()
        .context("wrap requires a command to run")?;

    let started = Instant::now();
    let mut child = Command::new(program)
        .args(args)
        .spawn()
        .with_context(|| format!("failed to start: {program}"))?;

    let status = child.wait().context("failed to wait for child process")?;
    let elapsed = started.elapsed();
    let elapsed_secs = elapsed.as_secs();

    let code = status.code();
    let status_text = match code {
        Some(0) => "exit 0".to_string(),
        Some(c) => format!("exit {c}"),
        None => "terminated by signal".to_string(),
    };

    if let Some(min) = min_duration_secs {
        if elapsed_secs < min {
            return Ok(code.unwrap_or(1));
        }
    }

    let title = if code == Some(0) {
        format!("✓ {program} finished")
    } else {
        format!("✗ {program} failed ({status_text})")
    };
    let body = format!(
        "{} after {}",
        summarize_command(command),
        format_duration(elapsed_secs)
    );
    notify::send(&title, &body, sound);

    Ok(code.unwrap_or(1))
}

/// Short human summary of the command line for notification bodies.
fn summarize_command(command: &[String]) -> String {
    let joined = command.join(" ");
    if joined.chars().count() <= 80 {
        joined
    } else {
        let cut: String = joined.chars().take(77).collect();
        format!("{cut}...")
    }
}

#[cfg(test)]
mod tests {
    use super::summarize_command;

    #[test]
    fn short_command_unchanged() {
        let cmd: Vec<String> = vec!["ssh".into(), "host".into(), "uptime".into()];
        assert_eq!(summarize_command(&cmd), "ssh host uptime");
    }

    #[test]
    fn long_command_truncated() {
        let cmd = vec!["ssh".into(), "x".repeat(200)];
        let s = summarize_command(&cmd);
        assert!(s.chars().count() <= 80);
        assert!(s.ends_with("..."));
    }
}
