use super::*;

fn row(buffer: &ratatui::buffer::Buffer, width: u16, y: u16) -> String {
    let start = usize::from(y) * usize::from(width);
    buffer.content[start..start + usize::from(width)]
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}

#[test]
fn header_status_places_local_clock_message_and_health_above_shortcut_only_footer() {
    let now = UNIX_EPOCH + Duration::from_secs(3_661);
    let mut app = App::new(32, 8_192);
    app.notification = Some("Daemon accepted workspace scan".into());
    let mut terminal = Terminal::new(TestBackend::new(140, 32)).unwrap();
    terminal.draw(|frame| render_at(frame, &app, now)).unwrap();
    let buffer = terminal.backend().buffer();
    let identity = row(buffer, 140, 1);
    let message = row(buffer, 140, 2);
    let footer = row(buffer, 140, 30);
    assert!(identity.contains(&clock_label(now)), "{identity}");
    assert!(!identity.contains("UTC"), "{identity}");
    assert!(
        message.contains("Daemon accepted workspace scan"),
        "{message}"
    );
    let status_start = usize::from(2_u16 * 140);
    assert!(
        buffer.content[status_start..status_start + 140]
            .iter()
            .any(|cell| cell.modifier.contains(Modifier::BOLD))
    );
    assert!(footer.contains("F1 Help"), "{footer}");
    assert!(!footer.contains("Daemon accepted"), "{footer}");
    assert!(!footer.contains("Local "), "{footer}");

    app.notification = None;
    terminal.draw(|frame| render_at(frame, &app, now)).unwrap();
    let health = row(terminal.backend().buffer(), 140, 2);
    assert!(health.contains("Daemon health:"), "{health}");
    assert!(health.contains("Disconnected"), "{health}");
    assert!(health.contains("BitBake:"), "{health}");
}

#[test]
fn header_status_uses_braille_for_waiting_daemon_activity() {
    let mut app = App::new(32, 8_192);
    app.daemon.status = yoctui_model::ClientReplicaStatus::Current;
    app.daemon.bitbake = yoctui_model::ClientDaemonLifecycle::Connecting;
    let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
    terminal
        .draw(|frame| render_at(frame, &app, UNIX_EPOCH))
        .unwrap();
    let status = row(terminal.backend().buffer(), 120, 2);
    let status = status.trim_matches([' ', '│']);
    assert!(
        throbber_widgets_tui::BRAILLE_EIGHT_DOUBLE
            .symbols
            .iter()
            .any(|symbol| status.starts_with(symbol)),
        "{status}"
    );
    assert!(status.contains("BitBake connecting"), "{status}");
}

#[test]
fn header_status_local_clock_has_no_seconds_or_utc_label() {
    let label = clock_label(SystemTime::now());
    assert!(label.starts_with("Local "), "{label}");
    assert_eq!(label.matches(':').count(), 1, "{label}");
    assert!(!label.contains("UTC"), "{label}");
}
