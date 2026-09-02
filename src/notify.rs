use notify_rust::Notification;

/// Send a desktop notification.
///
/// Cross-platform via notify-rust:
/// - Windows: native toast
/// - Linux: D-Bus (org.freedesktop.Notifications)
/// - macOS: Notification Center
///
/// Failure never panics — we warn on stderr and continue.
pub fn send(title: &str, body: &str, sound: bool) {
    let mut n = Notification::new();
    n.summary(title).body(body).appname("ssh-done");
    if sound {
        n.sound_name("default");
    }
    if let Err(e) = n.show() {
        eprintln!("warning: failed to show notification: {e}");
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn send_does_not_panic() {
        // We cannot assert a toast actually rendered headlessly, but the
        // contract is: never panic, even when no notification daemon exists.
        super::send("ssh-done test", "unit-test notification", false);
    }
}
