use super::*;

#[test]
fn log_build_filter_selection_source_and_copy_are_typed() {
    let mut app = App::new(10, 1_000);
    app.build.target = Some("core-image-minimal".into());
    let mut first = tagged_log("busybox", "do_compile", Severity::Info, "compiler output");
    first.path = Some(PathBuf::from("/tmp/log.do_compile"));
    let _ = update(&mut app, Action::Log(first));
    app.build.target = Some("core-image-full-cmdline".into());
    let _ = update(&mut app, Action::Log(log("second build")));

    let _ = update(&mut app, Action::CycleLogBuildFilter);
    assert_eq!(
        app.logs.build_filter.as_deref(),
        Some("core-image-full-cmdline")
    );
    assert_eq!(app.logs.filtered().count(), 1);
    let _ = update(&mut app, Action::CycleLogBuildFilter);
    assert_eq!(app.logs.build_filter.as_deref(), Some("core-image-minimal"));
    assert_eq!(
        update(&mut app, Action::OpenSelectedLogSource),
        Some(Effect::OpenInEditor(PathBuf::from("/tmp/log.do_compile")))
    );
    let Some(Effect::CopyToClipboard(details)) = update(&mut app, Action::CopySelectedLog) else {
        panic!("selected log details were not copied through a typed effect");
    };
    assert!(details.contains("Build: core-image-minimal"));
    assert!(details.contains("compiler output"));
}
