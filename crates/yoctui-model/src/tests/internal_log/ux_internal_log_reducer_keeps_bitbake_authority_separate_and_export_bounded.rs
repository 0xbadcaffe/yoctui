use super::*;

#[test]
fn ux_internal_log_reducer_keeps_bitbake_authority_separate_and_export_bounded() {
    let mut app = App::new(64, 512 * 1024);
    let bitbake = LogEntry {
        id: 0,
        severity: Severity::Error,
        message: "BitBake domain failure".into(),
        recipe: Some("busybox".into()),
        task: Some("do_compile".into()),
        path: None,
        timestamp: SystemTime::UNIX_EPOCH,
        build: Some("core-image-minimal".into()),
        protected: true,
        diagnostic: None,
    };
    let _ = update(&mut app, Action::Log(bitbake));
    for index in 0..64 {
        let mut entry = record(index, InternalLogLevel::Info, "yoctui::client");
        entry.message = format!("{index:03}-{}", "構".repeat(30_000));
        let _ = update(&mut app, Action::InternalLog(entry));
    }
    assert_eq!(app.logs.entries.len(), 1);
    assert!(
        app.internal_logs
            .entries
            .iter()
            .all(|entry| !entry.message.contains("BitBake domain failure"))
    );

    let _ = update(&mut app, Action::CycleLogWorkspaceView);
    assert_eq!(app.log_workspace_view, LogWorkspaceView::Yoctui);
    let Some(Effect::CopyToClipboard(export)) = update(&mut app, Action::ExportInternalLogs) else {
        panic!("internal export must use the existing typed clipboard authority");
    };
    assert!(export.len() <= MAX_INTERNAL_LOG_EXPORT_BYTES);
    assert!(export.ends_with("[internal diagnostic export truncated at 256 KiB]"));

    let evicted = app.internal_logs.evicted;
    let _ = update(&mut app, Action::ClearInternalLogs);
    assert!(app.internal_logs.entries.is_empty());
    assert_eq!(app.internal_logs.evicted, evicted);
    assert_eq!(app.logs.entries.len(), 1);
}
