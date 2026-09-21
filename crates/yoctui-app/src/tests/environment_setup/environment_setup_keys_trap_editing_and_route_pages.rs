use super::*;

#[test]
fn environment_setup_keys_trap_editing_and_route_pages() {
    let mut setup = EnvironmentSetup {
        values: Default::default(),
        field: 0,
        editor: None,
        browser: None,
        error: None,
    };
    assert_eq!(
        environment_setup_action(&setup, Input::Char('b')),
        Some(Action::EnvironmentSetup(A::Browse))
    );
    setup.editor = Some(yoctui_model::TextAreaState::new(String::new()));
    assert_eq!(
        environment_setup_action(&setup, Input::Char('b')),
        Some(Action::EnvironmentSetup(A::Insert("b".into())))
    );
    assert_eq!(
        environment_setup_action(&setup, Input::Esc),
        Some(Action::EnvironmentSetup(A::Cancel))
    );
    setup.editor = None;
    setup.browser = Some(yoctui_model::EnvironmentBrowser {
        request: 1,
        loading: false,
        directory: None,
        selection: 0,
    });
    assert_eq!(
        environment_setup_action(&setup, Input::PageDown),
        Some(Action::EnvironmentSetup(A::Select(10)))
    );
    assert_eq!(
        environment_setup_action(&setup, Input::Right),
        Some(Action::EnvironmentSetup(A::EnterDirectory))
    );
    let mut app = yoctui_model::App::new_unconfigured(20, 2000);
    app.dialogs
        .push_front(yoctui_model::Dialog::EnvironmentSetup(Box::new(setup)));
    assert_eq!(
        crate::mouse_action_for_app(
            crate::MouseInput {
                kind: crate::MouseKind::ScrollDown,
                column: 40,
                row: 12,
            },
            &app,
            80,
            24
        ),
        Some(Action::EnvironmentSetup(A::Select(1)))
    );
}
