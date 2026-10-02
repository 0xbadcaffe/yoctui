use super::*;
use yoctui_model::{KernelDebugDialog, KernelDebugDraft, KernelDebugTool};

#[test]
fn kernel_debug_instrumentation_form_review_and_confirmed_export_trap_input() {
    let mut app = App::new(32, 4096);
    app.onboarding.open = false;
    app.screen = Screen::Kernel;
    let mut draft = KernelDebugDraft::new(KernelDebugTool::Sanitizers);
    draft.instrumentation.config = "/work/.config".into();
    draft.instrumentation.output = "/work/debug.cfg".into();
    app.dialogs
        .push_front(Dialog::KernelDebug(KernelDebugDialog {
            draft: draft.clone(),
            selection: 0,
            guide_scroll: 0,
            error: None,
        }));
    for input in [Input::Left, Input::Right, Input::Char(' ')] {
        assert_eq!(
            kernel_debug_action(&app, input),
            Some(Action::KernelDebug(A::ChangeScope))
        );
    }
    assert_eq!(
        kernel_debug_action(&app, Input::Enter),
        Some(Action::KernelDebug(A::Review))
    );
    let mouse = crate::MouseInput {
        kind: crate::MouseKind::ScrollDown,
        column: 70,
        row: 15,
    };
    assert_eq!(
        crate::mouse_action_for_app(mouse, &app, 160, 42),
        Some(Action::KernelDebug(A::Field(1)))
    );
    let preview = yoctui_model::KernelInstrumentationPreview {
        report: yoctui_model::KernelInstrumentationReport::inspect(
            draft.instrumentation.preset,
            "",
        )
        .unwrap(),
        draft: *draft.instrumentation.clone(),
        destination_parent: "/work".into(),
        parent_identity: None,
    };
    app.kernel_debug.instrumentation_preview = Some(preview.clone());
    assert_eq!(
        crate::mouse_action_for_app(mouse, &app, 160, 42),
        Some(Action::KernelDebug(A::ScrollGuide(3)))
    );
    for input in [
        Input::Char('q'),
        Input::Tab,
        Input::CtrlU,
        Input::CtrlB,
        Input::F12,
    ] {
        assert!(kernel_debug_owns_input(&app, input));
        assert_eq!(kernel_debug_action(&app, input), None);
    }
    assert_eq!(
        kernel_debug_action(&app, Input::PageDown),
        Some(Action::KernelDebug(A::ScrollGuide(3)))
    );
    assert_eq!(
        kernel_debug_action(&app, Input::Esc),
        Some(Action::KernelDebug(A::Cancel))
    );
    app.kernel_debug.pending = Some(yoctui_model::KernelDebugOperation::ExportInstrumentation {
        expected: Box::new(preview),
    });
    assert_eq!(kernel_debug_action(&app, Input::Enter), None);
    // Escape reaches the reducer, which keeps confirmed writes locked until the result.
    assert_eq!(
        kernel_debug_action(&app, Input::Esc),
        Some(Action::KernelDebug(A::Cancel))
    );
    yoctui_model::update(&mut app, Action::KernelDebug(A::Cancel));
    assert!(app.kernel_debug.pending.is_some());
    assert!(app.active_dialog().is_some());
}

#[test]
fn kernel_debug_serial_readiness_uses_lowercase_text_and_combination_controls() {
    let mut app = App::new(32, 4096);
    app.onboarding.open = false;
    app.screen = Screen::Kernel;
    app.dialogs
        .push_front(Dialog::KernelDebug(KernelDebugDialog {
            draft: KernelDebugDraft::new(KernelDebugTool::KgdbSerial),
            selection: 5,
            guide_scroll: 0,
            error: None,
        }));
    for (input, expected) in [
        (Input::Char('y'), A::Insert("y".into())),
        (Input::CtrlU, A::Clear),
        (Input::Tab, A::Field(1)),
        (Input::BackTab, A::Field(-1)),
        (Input::Enter, A::Review),
        (Input::Esc, A::Cancel),
    ] {
        assert!(kernel_debug_owns_input(&app, input));
        assert_eq!(
            kernel_debug_action(&app, input),
            Some(Action::KernelDebug(expected))
        );
    }
    assert_eq!(kernel_debug_action(&app, Input::F12), None);
    assert!(kernel_debug_owns_input(&app, Input::F12));
}

#[test]
fn kernel_debug_navigation_is_kernel_only_and_does_not_steal_menuconfig_input() {
    let mut app = App::new(32, 4096);
    app.onboarding.open = false;
    app.screen = Screen::Kernel;
    app.focus = FocusTarget::Workspace;
    assert!(kernel_debug_owns_input(&app, Input::Char('3')));
    assert_eq!(
        kernel_debug_action(&app, Input::Char('b')),
        Some(Action::KernelDebug(A::Open))
    );
    app.kernel_debug.visible = true;
    assert!(kernel_debug_owns_input(&app, Input::Enter));
    assert!(!kernel_debug_owns_input(&app, Input::Tab)); // normal three-view route
    assert!(!kernel_debug_owns_input(&app, Input::Char('m'))); // existing menuconfig
    app.screen = Screen::Firmware;
    assert!(!kernel_debug_owns_input(&app, Input::Char('3')));
    app.screen = Screen::Kernel;
    app.command_palette_open = true;
    assert!(!kernel_debug_owns_input(&app, Input::Char('j')));
}

#[test]
fn kernel_debug_mouse_tab_rows_and_wheel_share_keyboard_actions() {
    use crate::{MouseInput, MouseKind, mouse_action_for_app};
    let mut app = App::new(32, 4096);
    app.onboarding.open = false;
    app.screen = Screen::Kernel;
    app.focus = FocusTarget::Workspace;
    assert_eq!(
        mouse_action_for_app(
            MouseInput {
                kind: MouseKind::Down,
                column: 64,
                row: 6
            },
            &app,
            160,
            42
        ),
        Some(Action::KernelDebug(A::Open))
    );
    app.kernel_debug.visible = true;
    assert_eq!(
        mouse_action_for_app(
            MouseInput {
                kind: MouseKind::Down,
                column: 30,
                row: 9
            },
            &app,
            160,
            42
        ),
        Some(Action::KernelDebug(A::SelectAt(1)))
    );
    assert_eq!(
        mouse_action_for_app(
            MouseInput {
                kind: MouseKind::ScrollDown,
                column: 30,
                row: 10
            },
            &app,
            160,
            42
        ),
        Some(Action::KernelDebug(A::Select(1)))
    );
}

#[test]
fn kernel_debug_modal_keys_are_typed_and_global_shortcuts_stay_inside_form() {
    let mut app = App::new(32, 4096);
    app.onboarding.open = false;
    app.screen = Screen::Kernel;
    app.dialogs
        .push_back(Dialog::KernelDebug(KernelDebugDialog {
            draft: KernelDebugDraft::new(KernelDebugTool::Strace),
            selection: 0,
            guide_scroll: 0,
            error: None,
        }));
    for input in [Input::Char('q'), Input::CtrlB, Input::F12, Input::Tab] {
        assert!(kernel_debug_owns_input(&app, input));
    }
    assert_eq!(
        kernel_debug_action(&app, Input::Char('q')),
        Some(Action::KernelDebug(A::Insert("q".into())))
    );
    assert_eq!(
        kernel_debug_action(&app, Input::Char(' ')),
        Some(Action::KernelDebug(A::ChangeScope))
    );
    assert_eq!(
        kernel_debug_action(&app, Input::CtrlU),
        Some(Action::KernelDebug(A::Clear))
    );
    assert_eq!(
        kernel_debug_action(&app, Input::Esc),
        Some(Action::KernelDebug(A::Cancel))
    );
    let Some(Dialog::KernelDebug(dialog)) = app.dialogs.front_mut() else {
        unreachable!()
    };
    dialog.draft = KernelDebugDraft::new(KernelDebugTool::SysrqKdump);
    assert_eq!(kernel_debug_action(&app, Input::Enter), None);
    assert_eq!(
        kernel_debug_action(&app, Input::PageDown),
        Some(Action::KernelDebug(A::ScrollGuide(3)))
    );
}

#[test]
fn kernel_debug_managed_qemu_memory_and_paths_stay_in_trapped_form() {
    let mut app = App::new(32, 4096);
    app.onboarding.open = false;
    app.screen = Screen::Kernel;
    app.dialogs
        .push_back(Dialog::KernelDebug(KernelDebugDialog {
            draft: KernelDebugDraft::new(KernelDebugTool::QemuGdb),
            selection: 6,
            guide_scroll: 0,
            error: None,
        }));
    assert!(kernel_debug_owns_input(&app, Input::Char('q')));
    assert_eq!(
        kernel_debug_action(&app, Input::Char('q')),
        Some(Action::KernelDebug(A::Insert("q".into())))
    );
    assert_eq!(
        kernel_debug_action(&app, Input::Enter),
        Some(Action::KernelDebug(A::Review))
    );
    assert_eq!(
        kernel_debug_action(&app, Input::Tab),
        Some(Action::KernelDebug(A::Field(1)))
    );
    assert_eq!(
        kernel_debug_action(&app, Input::Esc),
        Some(Action::KernelDebug(A::Cancel))
    );
}
