use super::*;

#[test]
fn hardening_stress_model_retention_preserves_high_volume_invariants() {
    const EVENTS: usize = 20_000;
    let mut app = App::new(128, 4_096);
    let _ = update(&mut app, Action::ToggleLogFollow);
    for index in 0..EVENTS {
        let severity = match index % 4 {
            0 => Severity::Trace,
            1 => Severity::Info,
            2 => Severity::Warning,
            _ => Severity::Error,
        };
        let _ = update(
            &mut app,
            Action::Log(LogEntry {
                id: 0,
                severity,
                message: format!("stress-event-{index:05}-{}", "x".repeat(index % 31)),
                recipe: Some(format!("recipe-{}", index % 17)),
                task: Some(format!("do_task_{}", index % 11)),
                path: None,
                timestamp: SystemTime::UNIX_EPOCH + Duration::from_millis(index as u64),
                build: Some(format!("build-{}", index % 3)),
                protected: false,
                diagnostic: None,
            }),
        );
        if index % 257 == 0 {
            let _ = update(&mut app, Action::ScrollLogs { delta: 19 });
        }
    }

    assert!(app.logs.entries.len() <= app.logs.max_entries);
    assert!(app.logs.retained_bytes <= app.logs.max_bytes);
    assert_eq!(
        app.logs.retained_bytes,
        app.logs
            .entries
            .iter()
            .map(|entry| entry.message.len())
            .sum::<usize>()
    );
    assert_eq!(app.logs.dropped + app.logs.entries.len(), EVENTS);
    assert_eq!(app.logs.coalesced, 0);
    assert_eq!(app.build.warnings, EVENTS / 4);
    assert_eq!(app.build.errors, EVENTS / 4);
    let visible = app.logs.filtered().count();
    assert!(visible == 0 || app.logs.selection < visible);
    assert!(app.logs.scroll_offset <= visible.saturating_sub(1));
}
