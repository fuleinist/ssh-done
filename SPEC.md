# ssh-done — SPEC

**Desktop notification when remote SSH commands finish. No more staring at hung terminal tabs.**

## Problem

Every developer with multiple servers has forgotten a long-running SSH command — a deploy, a backup, a migration — and stared at a terminal waiting. ssh-done is a tiny local tool that tells you when it's done. No cloud, no account, no setup on the remote host.

## Non-goals

- No remote agent/daemon to install on servers.
- No GUI configuration app.
- No parsing of SSH protocol; we watch local processes.

## Features

### F1 — `notify` subcommand (foundation)
`ssh-done notify "Title" "Body"` sends a desktop notification immediately. Used to verify notification plumbing and by users who want manual alerts.

### F2 — `wrap` subcommand
`ssh-done wrap -- ssh user@host 'long deploy script'` runs the given command as a child process, forwards stdio transparently, and on completion sends a notification: command name, exit status, elapsed duration.
- Exit code of the child is propagated to ssh-done's exit code.
- A `--min-duration <secs>` flag suppresses the notification if the command finished faster than the threshold (default: always notify).

### F3 — `daemon` subcommand
`ssh-done daemon` runs in the foreground and polls the local process table every 3 seconds.
- Detects running SSH client processes (`ssh`, `ssh.exe`) by executable name.
- Tracks each by PID with first-seen timestamp.
- When a tracked SSH process disappears and lived at least `min_duration` (config default 10s), fire a notification: "SSH session ended" with the command line tail and duration.
- Short-lived sessions (below threshold) are silently dropped.
- Runs until Ctrl+C; clean shutdown.

### F4 — Configuration
Config file at the platform config dir (`ssh-done/config.toml`):
```toml
min_duration_secs = 10   # daemon: ignore sessions shorter than this
poll_interval_secs = 3   # daemon poll cadence
sound = true             # notification sound where supported
```
CLI flags override config. Missing file = defaults. Invalid file = warn + defaults.

### F5 — Cross-platform notifications
- Windows: native toast via notify-rust.
- Linux: D-Bus notifications via notify-rust.
- macOS: Notification Center via notify-rust.
Notification failure never crashes the program (warn + continue).

## Tech stack

- Rust 2021 edition, MSRV current stable.
- `clap` (derive) for CLI.
- `sysinfo` for process enumeration (daemon mode).
- `notify-rust` for notifications.
- `serde` + `toml` for config.
- `dirs` for config path.

## Architecture

```
src/
  main.rs        — clap CLI entry, subcommand dispatch
  notify.rs      — notification abstraction over notify-rust
  wrap.rs        — wrap mode: spawn child, wait, notify
  daemon.rs      — poll loop + SshTracker state machine
  config.rs      — config load/defaults
```

`SshTracker` is a pure state machine (no I/O) so it is unit-testable:
- `observe(snapshot)` → returns list of events (`SessionEnded { pid, cmdline, duration }`) for processes that disappeared after living ≥ min_duration.
- New PIDs are registered; gone PIDs are removed.

## Acceptance criteria

1. `cargo build` and `cargo test` pass on Windows.
2. `ssh-done notify "Hi" "Hello"` delivers a desktop notification on the current platform.
3. `ssh-done wrap -- ping -n 2 127.0.0.1` (Windows) / `ping -c 2 127.0.0.1` notifies with exit code + duration; exit code propagates.
4. Unit tests cover SshTracker: new-session registration, below-threshold silent drop, at-threshold notification event, PID reuse handled (same PID reappearing as new session resets tracking).
5. `ssh-done daemon` starts, polls, handles Ctrl+C gracefully (unit-test the tracker; manual smoke for the loop).
6. Config loading tested: defaults, valid file, invalid file fallback.
7. README with install (cargo install / binaries), usage, examples, platform notes.

## Milestones (GSD)

- **M1**: Scaffold + CLI + notify module. Verify: `cargo run -- notify T B` works; `cargo test` green.
- **M2**: wrap mode with stdio forwarding + exit code propagation + `--min-duration`. Verify: wrap ping smoke test; tests green.
- **M3**: daemon mode: sysinfo scan + SshTracker + notification on session end. Verify: tracker unit tests green; daemon starts and exits cleanly.
- **M4**: config file support wired into daemon; clippy + fmt clean; README + LICENSE; GitHub Actions CI (cargo fmt --check, clippy, test). Verify: full `cargo build --release && cargo test && cargo clippy`.

## Verify protocol (each cycle)

```powershell
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cargo build --release
```

Any failure = fix before next milestone.
