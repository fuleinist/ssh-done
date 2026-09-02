use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const DEFAULT_MIN_DURATION_SECS: u64 = 10;
pub const DEFAULT_POLL_INTERVAL_SECS: u64 = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Daemon: ignore SSH sessions shorter than this (seconds).
    pub min_duration_secs: u64,
    /// Daemon: process-table poll cadence (seconds).
    pub poll_interval_secs: u64,
    /// Play a notification sound where the platform supports it.
    pub sound: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            min_duration_secs: DEFAULT_MIN_DURATION_SECS,
            poll_interval_secs: DEFAULT_POLL_INTERVAL_SECS,
            sound: true,
        }
    }
}

impl Config {
    /// Platform config path: `<config_dir>/ssh-done/config.toml`.
    pub fn path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("ssh-done").join("config.toml"))
    }

    /// Load config from `path`.
    ///
    /// Missing file -> defaults. Invalid file -> warn on stderr and fall back
    /// to defaults (never crashes).
    pub fn load(path: Option<&Path>) -> Self {
        let path = match path.map(Path::to_path_buf).or_else(Self::path) {
            Some(p) => p,
            None => return Self::default(),
        };
        match std::fs::read_to_string(&path) {
            Ok(raw) => match toml::from_str::<Config>(&raw) {
                Ok(cfg) => cfg,
                Err(e) => {
                    eprintln!(
                        "warning: invalid config at {} ({}); using defaults",
                        path.display(),
                        e
                    );
                    Self::default()
                }
            },
            Err(_) => Self::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_when_missing() {
        let cfg = Config::load(Some(Path::new(
            "./nonexistent/config-that-does-not-exist.toml",
        )));
        assert_eq!(cfg.min_duration_secs, DEFAULT_MIN_DURATION_SECS);
        assert_eq!(cfg.poll_interval_secs, DEFAULT_POLL_INTERVAL_SECS);
        assert!(cfg.sound);
    }

    #[test]
    fn invalid_file_falls_back_to_defaults() {
        let dir = tempdir();
        let path = dir.join("bad.toml");
        std::fs::write(&path, "this is [not valid toml = =").unwrap();
        let cfg = Config::load(Some(&path));
        assert_eq!(cfg.min_duration_secs, DEFAULT_MIN_DURATION_SECS);
    }

    #[test]
    fn valid_file_overrides() {
        let dir = tempdir();
        let path = dir.join("good.toml");
        std::fs::write(
            &path,
            "min_duration_secs = 30\npoll_interval_secs = 5\nsound = false\n",
        )
        .unwrap();
        let cfg = Config::load(Some(&path));
        assert_eq!(cfg.min_duration_secs, 30);
        assert_eq!(cfg.poll_interval_secs, 5);
        assert!(!cfg.sound);
    }

    #[test]
    fn partial_file_uses_defaults_for_missing_keys() {
        let dir = tempdir();
        let path = dir.join("partial.toml");
        std::fs::write(&path, "min_duration_secs = 42\n").unwrap();
        let cfg = Config::load(Some(&path));
        assert_eq!(cfg.min_duration_secs, 42);
        assert_eq!(cfg.poll_interval_secs, DEFAULT_POLL_INTERVAL_SECS);
        assert!(cfg.sound);
    }

    fn tempdir() -> PathBuf {
        let base = std::env::temp_dir().join(format!(
            "ssh-done-test-{}-{}",
            std::process::id(),
            rand_suffix()
        ));
        std::fs::create_dir_all(&base).unwrap();
        base
    }

    fn rand_suffix() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0)
    }
}
