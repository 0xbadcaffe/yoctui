use super::*;

#[test]
fn selected_error_opens_its_source_path() {
    let mut app = App::new(10, 1_000);
    let mut entry = tagged_log("busybox", "do_compile", Severity::Error, "compile failed");
    entry.path = Some(PathBuf::from("/tmp/log.do_compile"));
    let _ = update(&mut app, Action::Log(entry));

    assert_eq!(
        update(&mut app, Action::OpenSelectedErrorSource),
        Some(Effect::OpenInEditor(PathBuf::from("/tmp/log.do_compile")))
    );
}
