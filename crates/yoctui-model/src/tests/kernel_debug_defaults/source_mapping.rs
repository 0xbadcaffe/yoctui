use super::*;
use std::path::PathBuf;

#[test]
fn remote_defaults_install_source_map_only_for_unchanged_matching_symbols() {
    for edited in [false, true] {
        let mut app = app();
        app.kernel_debug.selection = 0;
        update(
            &mut app,
            Action::KernelDebug(KernelDebugAction::OpenSelected),
        );
        let build = PathBuf::from(
            "/build/tmp/work/romulus-poky-linux/linux-yocto/7.2.8+git/linux-romulus-build",
        );
        let symbols = build.join("vmlinux");
        if edited {
            let Some(Dialog::KernelDebug(dialog)) = app.active_dialog_mut() else {
                panic!()
            };
            dialog.draft.symbols = "/manual/vmlinux".into();
            app.kernel_debug
                .default_edits
                .push(KernelDebugField::Symbols);
        }
        let generation = app.kernel_debug.generation;
        update(
            &mut app,
            Action::KernelDebug(KernelDebugAction::Finished {
                generation,
                result: Ok(KernelDebugResult::Defaults(KernelDebugDefaults {
                    values: vec![(KernelDebugField::Symbols, symbols.display().to_string())],
                    source_map: Some(Box::new(crate::KernelDebugSourceMap {
                        build_dir: "/build".into(),
                        machine: "romulus".into(),
                        symbols,
                        kernel_source: "/build/tmp/work-shared/romulus/kernel-source".into(),
                        kernel_build: build,
                    })),
                    ..Default::default()
                })),
            }),
        );
        let Some(Dialog::KernelDebug(dialog)) = app.active_dialog() else {
            panic!()
        };
        assert_eq!(dialog.draft.source_map.is_some(), !edited);
        if edited {
            assert_eq!(dialog.draft.symbols, "/manual/vmlinux");
        }
    }
}
