use super::*;
use crate::{Action, update};

fn app() -> App {
    let mut app = App::new(32, 4096);
    app.onboarding.open = false;
    app.screen = Screen::Kernel;
    app.workspace.build_dir = Some("/build".into());
    app.workspace
        .variables
        .insert("MACHINE".into(), "romulus".into());
    app.build.target = Some("image".into());
    app.kernel_debug.selection = KernelDebugTool::ALL
        .iter()
        .position(|t| *t == KernelDebugTool::QemuGdb)
        .unwrap();
    app
}
fn result(app: &mut App, generation: u64) {
    update(
        app,
        Action::KernelDebug(KernelDebugAction::Finished {
            generation,
            result: Ok(KernelDebugResult::Defaults(KernelDebugDefaults {
                values: vec![
                    (KernelDebugField::Qemuboot, "/build/image.conf".into()),
                    (KernelDebugField::Memory, "512".into()),
                ],
                boot_mode: Some(crate::QemuDebugBootMode::OpenBmcRomulusFlash),
                note: "Missing: symbols".into(),
            })),
        }),
    );
}
#[test]
fn kernel_debug_defaults_seed_untouched_fields_and_preserve_clear_and_selector_edits() {
    for edit in [false, true] {
        let mut app = app();
        assert!(matches!(
            update(
                &mut app,
                Action::KernelDebug(KernelDebugAction::OpenSelected)
            ),
            Some(Effect::KernelDebug(_))
        ));
        let generation = app.kernel_debug.generation;
        if edit {
            let Some(Dialog::KernelDebug(d)) = app.active_dialog_mut() else {
                panic!()
            };
            d.selection = 2;
            update(&mut app, Action::KernelDebug(KernelDebugAction::Clear));
            let Some(Dialog::KernelDebug(d)) = app.active_dialog_mut() else {
                panic!()
            };
            d.selection = 7;
            update(
                &mut app,
                Action::KernelDebug(KernelDebugAction::ChangeScope),
            );
            update(
                &mut app,
                Action::KernelDebug(KernelDebugAction::ChangeScope),
            );
        }
        assert!(update(&mut app, Action::KernelDebug(KernelDebugAction::Review)).is_none());
        result(&mut app, generation);
        let Some(Dialog::KernelDebug(d)) = app.active_dialog() else {
            panic!()
        };
        assert_eq!(d.draft.qemu.qemuboot.is_empty(), edit);
        assert_eq!(d.draft.qemu.memory, "512");
        assert_eq!(
            d.draft.qemu.boot_mode == crate::QemuDebugBootMode::DirectKernel,
            edit
        );
        assert!(app.kernel_debug.prepared.is_none());
    }
}
#[test]
fn kernel_debug_defaults_cancel_reopen_and_changed_environment_ignore_late_results() {
    for scenario in [
        "cancel",
        "reopen",
        "build",
        "machine",
        "image",
        "covered",
        "screen",
        "draft-build",
    ] {
        let mut app = app();
        update(
            &mut app,
            Action::KernelDebug(KernelDebugAction::OpenSelected),
        );
        let generation = app.kernel_debug.generation;
        match scenario {
            "cancel" | "reopen" => {
                update(&mut app, Action::KernelDebug(KernelDebugAction::Cancel));
                if scenario == "reopen" {
                    update(
                        &mut app,
                        Action::KernelDebug(KernelDebugAction::OpenSelected),
                    );
                }
            }
            "build" => app.workspace.build_dir = Some("/other".into()),
            "machine" => {
                app.workspace
                    .variables
                    .insert("MACHINE".into(), "other".into());
            }
            "image" => app.build.target = Some("other".into()),
            "covered" => app.command_palette_open = true,
            "screen" => app.screen = Screen::Dashboard,
            "draft-build" => {
                let Some(Dialog::KernelDebug(d)) = app.active_dialog_mut() else {
                    panic!()
                };
                d.selection = 1;
                update(&mut app, Action::KernelDebug(KernelDebugAction::Clear));
                update(
                    &mut app,
                    Action::KernelDebug(KernelDebugAction::Insert("/other".into())),
                );
            }
            _ => unreachable!(),
        }
        result(&mut app, generation);
        if let Some(Dialog::KernelDebug(d)) = app.active_dialog() {
            assert!(d.draft.qemu.qemuboot.is_empty(), "{scenario}");
        }
        assert!(app.kernel_debug.prepared.is_none());
    }
}
