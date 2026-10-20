use super::*;

#[test]
fn sanitizer_and_lockdep_forms_share_the_long_path_viewport() {
    for (tool, config_field, output_field) in [
        (KernelDebugTool::Sanitizers, 1, 2),
        (KernelDebugTool::Lockdep, 0, 1),
    ] {
        for (selection, expected) in [
            (config_field, ".config_"),
            (output_field, "kernel-debug-fragment.cfg_"),
        ] {
            let mut app = app();
            let mut draft = KernelDebugDraft::new(tool);
            draft.instrumentation.config = format!("/{}.config", "retained-kernel/".repeat(24));
            draft.instrumentation.output = format!(
                "/{}kernel-debug-fragment.cfg",
                "private-captures/".repeat(24)
            );
            app.dialogs
                .push_back(Dialog::KernelDebug(KernelDebugDialog {
                    draft,
                    selection,
                    guide_scroll: 0,
                    error: None,
                }));
            for (width, height) in [(80, 24), (100, 30), (160, 50)] {
                let text = rendered_text(&app, width, height);
                assert!(text.contains(expected), "{tool:?} {width}x{height}: {text}");
                assert!(text.contains("Ctrl+V paste"), "{text}");
            }
        }
    }
}

#[test]
fn long_debug_paths_show_the_filename_and_cursor_without_mutating_values() {
    let mut app = app();
    let mut draft = KernelDebugDraft::new(KernelDebugTool::GdbCore);
    draft.symbols = format!("/{}vmlinux-debug.elf", "retained-kernel/".repeat(24));
    draft.data = format!("/{}userspace.core", "private-captures/".repeat(24));
    for selection in [0, 1] {
        app.dialogs.clear();
        app.dialogs
            .push_back(Dialog::KernelDebug(KernelDebugDialog {
                draft: draft.clone(),
                selection,
                guide_scroll: 0,
                error: None,
            }));
        for (width, height) in [(80, 24), (100, 30), (160, 50)] {
            let text = rendered_text(&app, width, height);
            let expected = if selection == 0 {
                "vmlinux-debug.elf_"
            } else {
                "userspace.core_"
            };
            assert!(text.contains(expected), "{width}x{height}: {text}");
        }
        let Some(Dialog::KernelDebug(dialog)) = app.active_dialog() else {
            panic!("debug form disappeared");
        };
        assert_eq!(dialog.draft, draft);
    }
}

#[test]
fn debug_field_viewport_is_cell_bounded_for_unicode_and_tiny_widths() {
    let value = format!("/{}调试-vmlinux", "目录/".repeat(64));
    for active in [false, true] {
        for width in [0, 1, 2, 8, 16, 30, 76, 108] {
            let line = crate::kernel_debug_render::field_line(
                "Absolute executable/vmlinux symbols",
                &value,
                active,
                width,
            );
            assert!(
                Line::from(line.as_str()).width() <= usize::from(width),
                "{line}"
            );
            if active && width > 0 {
                assert!(line.ends_with('_'), "{line}");
            }
            if width >= 76 {
                assert!(line.contains("调试-vmlinux"), "{line}");
            }
        }
    }
}
