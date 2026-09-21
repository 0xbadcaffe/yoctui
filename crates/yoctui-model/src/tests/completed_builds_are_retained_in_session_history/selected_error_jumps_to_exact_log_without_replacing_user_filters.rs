use super::*;

#[test]
fn selected_error_jumps_to_exact_log_without_replacing_user_filters() {
    let mut app = App::new(10, 1_000);
    let _ = update(
        &mut app,
        Action::Log(tagged_log(
            "busybox",
            "do_compile",
            Severity::Error,
            "compile failed",
        )),
    );
    app.logs.query = "user query".into();
    app.logs.filter = Some(Severity::Warning);
    let _ = update(&mut app, Action::Open(Screen::Errors));
    let _ = update(&mut app, Action::JumpToSelectedError);
    assert_eq!(app.screen, Screen::Logs);
    assert_eq!(app.logs.query, "user query");
    assert_eq!(app.logs.filter, Some(Severity::Warning));
    assert_eq!(
        app.logs.selected().map(|entry| entry.message.as_str()),
        Some("compile failed")
    );
}
