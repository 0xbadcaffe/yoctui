use super::*;

#[test]
fn dashboard_quick_actions_advertise_current_modifiers_without_capital_requirements() {
    let app = App::new(32, 8192);
    assert!(!app.is_offline());
    for width in [80, 100, 130, 160] {
        let mut terminal = Terminal::new(TestBackend::new(width, 9)).unwrap();
        terminal
            .draw(|frame| {
                crate::dashboard_render::render_dashboard_quick_actions(frame, &app, frame.area());
            })
            .unwrap();
        let output: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        for hint in ["[Alt+b]", "[Alt+e]", "[t]"] {
            assert!(output.contains(hint), "{width}: missing {hint}: {output}");
        }
        assert!(!output.contains("[B]"), "{output}");
        assert!(!output.contains("[E]"), "{output}");
    }
}

#[test]
fn dashboard_offline_setup_and_empty_history_keep_current_shortcuts() {
    let mut app = App::new(32, 8192);
    app.require_daemon = true;
    assert!(app.is_offline());
    let output = rendered_text_at(&app, 160, 50, literal_now());
    assert!(
        output.contains("[Alt+e] Configure build environment"),
        "{output}"
    );
    assert!(output.contains("[F3] Saved build history"), "{output}");
    assert!(!output.contains("[E]"), "{output}");

    app.daemon.status = yoctui_model::ClientReplicaStatus::Current;
    let output = rendered_text_at(&app, 160, 50, literal_now());
    assert!(output.contains("Alt+b opens build options"), "{output}");
    assert!(!output.contains(" B opens build options"), "{output}");
}
