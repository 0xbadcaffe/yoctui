use super::*;

#[test]
fn error_entries_gain_typed_category_summary_metadata_and_suggestions() {
    let mut app = App::new(10, 1_000);
    app.build.target = Some("core-image-minimal".into());
    let mut entry = tagged_log(
        "busybox",
        "do_compile",
        Severity::Error,
        "compile failed\nfull compiler context",
    );
    entry.path = Some(PathBuf::from("/tmp/log.do_compile"));
    let _ = update(&mut app, Action::Log(entry));
    let retained = app.logs.diagnostics().next().unwrap();
    let diagnostic = retained.diagnostic.as_ref().unwrap();
    assert_eq!(diagnostic.category, "BitBake error");
    assert_eq!(diagnostic.summary, "compile failed");
    assert!(
        diagnostic
            .event_metadata
            .iter()
            .any(|(name, value)| name == "build" && value == "core-image-minimal")
    );
    assert!(diagnostic.suggestions.len() >= 2);
    assert_eq!(retained.build.as_deref(), Some("core-image-minimal"));
}
