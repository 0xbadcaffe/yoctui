use super::*;

#[test]
fn log_batches_preserve_critical_order_counts_and_cached_search() {
    let mut app = App::new(8, 4_096);
    app.build.target = Some("core-image-minimal".into());
    let _ = update(
        &mut app,
        Action::Logs(vec![
            log("ordinary Alpha"),
            tagged_log("busybox", "do_compile", Severity::Warning, "warning Beta"),
            tagged_log("busybox", "do_install", Severity::Error, "failure Gamma"),
        ]),
    );

    assert_eq!(app.build.warnings, 1);
    assert_eq!(app.build.errors, 1);
    assert_eq!(app.logs.entries.len(), 3);
    assert_eq!(app.logs.normalized_messages.len(), 3);
    assert!(app.logs.entries[1].protected);
    assert!(app.logs.entries[2].protected);
    assert_eq!(app.logs.entries[1].message, "warning Beta");
    assert_eq!(app.logs.entries[2].message, "failure Gamma");
    assert!(
        app.logs
            .entries
            .iter()
            .all(|entry| entry.build.as_deref() == Some("core-image-minimal"))
    );

    app.logs.query = "GAMMA".into();
    assert_eq!(app.logs.filtered().next().unwrap().message, "failure Gamma");
}
