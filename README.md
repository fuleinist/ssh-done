# ssh-done

**Desktop notification when remote SSH commands finish. No more staring at hung terminal tabs.**

ssh-done is a tiny local tool that tells you when an SSH session or a wrapped command completes. No cloud, no account, nothing installed on the remote host.

```
you:  ssh prod 'run 40-minute migration'
      ... go make coffee ...
ssh-done: 🔔 "SSH session ended — prod 'run 40-minute migration' after 41m 12s"
```

## Install

From source (Rust):

```bash
cargo install --git https://github.com/fuleinist/ssh-done
```

Or build locally:

```bash
git clone https://github.com/fuleinist/ssh-done
cd ssh-done
cargo build --release
# binary at target/release/ssh-done(.exe)
```

## Usage

### Watch all SSH sessions (daemon mode)

```bash
ssh-done daemon
```

Polls your local process table every 3 seconds. When an `ssh` process that ran for at least 10 seconds exits, you get a desktop notification with the command tail and duration. Short sessions are silently ignored. Ctrl+C stops the daemon.

```bash
ssh-done daemon --min-duration 30 --poll-interval 5
```

### Wrap a single command

```bash
ssh-done wrap -- ssh user@server 'deploy.sh'
ssh-done wrap -- ./long-build.sh
ssh-done wrap --min-duration 60 -- make release
```

Runs the command with your terminal attached (stdio forwarded), then notifies with exit status and elapsed time. The wrapped command's exit code is propagated. `--min-duration` skips the notification for fast runs.

### Test notifications

```bash
ssh-done notify "Hello" "Notifications work"
```

## Configuration

Optional config file:

| Platform | Path |
|----------|------|
| Linux    | `~/.config/ssh-done/config.toml` |
| macOS    | `~/Library/Application Support/ssh-done/config.toml` |
| Windows  | `%APPDATA%\ssh-done\config.toml` |

```toml
min_duration_secs = 10   # daemon: ignore sessions shorter than this
poll_interval_secs = 3   # daemon: process-table poll cadence
sound = true             # notification sound where supported
```

Missing file → defaults. Invalid file → warning + defaults. CLI flags override config.

## How it works

- **No remote agent.** ssh-done only watches your *local* process table for the SSH client process (`ssh`/`ssh.exe`) via [`sysinfo`](https://crates.io/crates/sysinfo).
- When a tracked SSH process disappears after living at least `min_duration_secs`, a notification fires via [`notify-rust`](https://crates.io/crates/notify-rust):
  - **Windows:** native toast
  - **Linux:** D-Bus desktop notifications
  - **macOS:** Notification Center
- `wrap` mode doesn't watch processes at all — it runs your command as a child and reports when it exits.

### Limitations

- Session duration is measured from when the daemon first observes the process (sessions already running when the daemon starts are timed from daemon start).
- If an SSH client is killed by a different name/symlink it won't match — the filter looks for executables named `ssh` or `ssh.exe`.
- Mux'd sessions: OpenSSH multiplexing keeps a master connection open; the notification fires when the master exits.

## Development

```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```

See [SPEC.md](SPEC.md) for the full specification and milestone plan.

## License

MIT — see [LICENSE](LICENSE).
