use super::*;
use yoctui_model::{BitBakeSetupAction as A, TextAreaPasteSource, update};

fn wizard_app() -> App {
    let mut app = App::new(20, 2000);
    app.screen = Screen::BuildEnvironment;
    app.focus = FocusTarget::Workspace;
    app.bitbake_setup.tool = "bitbake-setup".into();
    app.bitbake_setup.top_dir = "/tmp/setups".into();
    update(&mut app, Action::BitBakeSetup(A::New));
    app
}

#[test]
fn setup_input_respects_focus_review_and_modal_text() {
    let mut app = wizard_app();
    let action = bitbake_setup_action(&app, Input::Enter).unwrap();
    update(&mut app, action);
    for key in ['q', 'w', 'r', 's'] {
        assert_eq!(
            bitbake_setup_action(&app, Input::Char(key)),
            Some(Action::BitBakeSetup(A::Insert(key.to_string())))
        );
    }
    update(&mut app, Action::BitBakeSetup(A::Cancel)); // discard field edit
    assert!(app.active_dialog().is_some());
    update(&mut app, Action::BitBakeSetup(A::Review));
    assert_eq!(
        bitbake_setup_action(&app, Input::Enter),
        Some(Action::BitBakeSetup(A::Confirm))
    );
    update(&mut app, Action::BitBakeSetup(A::Cancel)); // back from review
    update(&mut app, Action::BitBakeSetup(A::Cancel)); // close form
    app.focus = FocusTarget::Navigator;
    assert_eq!(bitbake_setup_action(&app, Input::Char('w')), None);
    app.focus = FocusTarget::Workspace;
    assert_eq!(
        bitbake_setup_action(&app, Input::Char('w')),
        Some(Action::BitBakeSetup(A::New))
    );
}

#[test]
fn setup_paste_is_one_unicode_text_action_and_rejects_newlines() {
    let mut app = wizard_app();
    update(&mut app, Action::BitBakeSetup(A::Edit));
    let text = "/tmp/é example/bitbake-setup";
    let actions = crate::text_paste_actions(&app, text.into(), TextAreaPasteSource::Clipboard)
        .unwrap()
        .unwrap();
    assert_eq!(actions, vec![Action::BitBakeSetup(A::Insert(text.into()))]);
    for action in actions {
        update(&mut app, action);
    }
    let Some(Dialog::BitBakeSetup(wizard)) = app.active_dialog() else {
        panic!("missing wizard")
    };
    assert_eq!(wizard.editor.as_ref().unwrap().text, text);
    assert!(
        crate::text_paste_actions(&app, "bad\nvalue".into(), TextAreaPasteSource::Clipboard)
            .is_err()
    );
}

#[test]
fn setup_management_typing_is_literal_and_default_update_options_are_explicit() {
    use yoctui_model::{SetupManageAction as M, SetupOperation};
    let mut app = wizard_app();
    update(&mut app, Action::BitBakeSetup(A::Cancel));
    app.daemon.status = yoctui_model::ClientReplicaStatus::Current;
    app.build_environment =
        yoctui_model::BuildEnvironmentState::Configured(yoctui_model::BuildEnvironmentProfile {
            source_dir: "/tmp/setup/layers/core".into(),
            build_dir: "/tmp/setup/build".into(),
            init_script: "/tmp/setup/build/init-build-env".into(),
        });
    update(&mut app, Action::BitBakeSetup(A::Manage(M::Custom)));
    let action = bitbake_setup_action(&app, Input::Char('e')).unwrap();
    update(&mut app, action);
    for key in ['q', 'c', 'b', 'n'] {
        assert_eq!(
            bitbake_setup_action(&app, Input::Char(key)),
            Some(Action::BitBakeSetup(A::Manage(M::Insert(key.to_string()))))
        );
    }
    let actions = crate::text_paste_actions(
        &app,
        "machine/qemuarm64".into(),
        TextAreaPasteSource::Clipboard,
    )
    .unwrap()
    .unwrap();
    assert_eq!(actions.len(), 1);
    for action in actions {
        update(&mut app, action);
    }
    update(&mut app, Action::BitBakeSetup(A::Manage(M::AcceptEdit)));
    let Some(Dialog::BitBakeSetupOperation(d)) = app.active_dialog() else {
        panic!("missing review")
    };
    assert_eq!(d.fragment, "machine/qemuarm64");
    assert_eq!(d.operation, SetupOperation::Enable);
    assert_eq!(
        bitbake_setup_action(&app, Input::Enter),
        Some(Action::BitBakeSetup(A::Manage(M::Confirm)))
    );
}
