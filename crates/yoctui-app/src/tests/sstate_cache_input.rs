use super::*;
#[test]
fn cache_keys_require_focus_and_preserve_maintenance_tools() {
    let mut app = App::new(100, 4096);
    assert_eq!(sstate_cache_action(&app, Input::Char('e')), None);
    app.screen = Screen::Maintenance;
    app.sstate_cache.dashboard = true;
    app.focus = FocusTarget::Workspace;
    assert_eq!(
        sstate_cache_action(&app, Input::Char('e')),
        Some(Action::SstateCache(C::Configure))
    );
    assert_eq!(
        sstate_cache_action(&app, Input::End),
        Some(Action::SstateCache(C::Last))
    );
    assert_eq!(
        sstate_cache_action(&app, Input::Esc),
        Some(Action::SstateCache(C::Tools))
    );
    for key in [
        Input::Char('c'),
        Input::Char('d'),
        Input::Char('r'),
        Input::Tab,
        Input::Char('['),
    ] {
        assert_eq!(sstate_cache_action(&app, key), None);
    }
    app.focus = FocusTarget::Navigator;
    assert_eq!(sstate_cache_action(&app, Input::Char('e')), None);
}
#[test]
fn cache_dialog_paste_is_text_and_escape_leaves_insert_before_cancel() {
    let mut app = App::new(100, 4096);
    app.screen = Screen::Maintenance;
    app.focus = FocusTarget::Workspace;
    app.workspace.build_dir = Some("/build".into());
    yoctui_model::update(&mut app, Action::SstateCache(C::Configure));
    let Some(Dialog::SstateCache(d)) = app.dialogs.back_mut() else {
        panic!("cache dialog")
    };
    let SstateCacheDialog::Configure { editor, .. } = d.as_mut() else {
        panic!("editor")
    };
    editor.editing = true;
    assert!(crate::text_paste_active(&app));
    assert!(matches!(
        crate::text_paste_actions(
            &app,
            "q\nmirror_url = \"https://host\"".into(),
            yoctui_model::TextAreaPasteSource::BracketedPaste
        )
        .unwrap()
        .unwrap()
        .as_slice(),
        [Action::EditActivePopup(_)]
    ));
    assert!(!matches!(
        sstate_cache_action(&app, Input::Esc),
        Some(Action::SstateCache(C::Cancel))
    ));
}
#[test]
fn cache_mouse_target_matches_the_visible_editor_or_report() {
    let mut app = App::new(100, 4096);
    app.screen = Screen::Maintenance;
    app.focus = FocusTarget::Workspace;
    app.workspace.build_dir = Some("/build".into());
    app.sstate_cache.dashboard = true;
    yoctui_model::update(&mut app, Action::SstateCache(C::Configure));
    assert!(crate::mouse::active_dialog_textarea(&app).is_some());
    yoctui_model::update(&mut app, Action::SstateCache(C::Review));
    assert!(crate::mouse::active_dialog_textarea(&app).is_none());
    assert_eq!(
        crate::mouse::dialog_mouse_action(
            crate::MouseInput {
                kind: crate::MouseKind::ScrollDown,
                column: 50,
                row: 10
            },
            &app,
            100,
            30
        ),
        Some(Action::SstateCache(C::Scroll(1)))
    );
}
