use crate::*;
use KernelDebugAction as A;

fn form(index: usize) -> App {
    let mut app = App::new(32, 4096);
    app.onboarding.open = false;
    app.screen = Screen::Kernel;
    app.kernel_debug.selection = index;
    update(&mut app, Action::KernelDebug(A::OpenSelected));
    let Some(Dialog::KernelDebug(d)) = app.active_dialog_mut() else {
        panic!()
    };
    d.draft.instrumentation.config = "/work/.config".into();
    d.draft.instrumentation.output = "/work/debug.cfg".into();
    app
}

fn inspect(app: &mut App) -> (KernelDebugRequest, KernelInstrumentationPreview) {
    let Some(Effect::KernelDebug(request)) = update(app, Action::KernelDebug(A::Review)) else {
        panic!()
    };
    let KernelDebugOperation::InspectInstrumentation { draft } = &request.operation else {
        panic!()
    };
    let preview = KernelInstrumentationPreview {
        draft: draft.clone(),
        report: KernelInstrumentationReport::inspect(draft.preset, "CONFIG_DEBUG_KERNEL=y\n")
            .unwrap(),
        destination_parent: "/work".into(),
        parent_identity: None,
    };
    (request, preview)
}

fn finish(app: &mut App, generation: u64, result: Result<KernelDebugResult, String>) {
    update(app, Action::KernelDebug(A::Finished { generation, result }));
}

#[test]
fn kernel_debug_instrumentation_presets_review_requires_separate_confirmation_without_tools() {
    for (index, preset) in [
        (13, KernelInstrumentationPreset::Kasan),
        (14, KernelInstrumentationPreset::Lockdep),
    ] {
        let mut app = form(index);
        assert!(app.kernel_debug.tools.is_none());
        let (request, preview) = inspect(&mut app);
        assert_eq!(preview.draft.preset, preset);
        assert!(app.kernel_debug.instrumentation_preview.is_none());
        finish(
            &mut app,
            request.generation,
            Ok(KernelDebugResult::InstrumentationPrepared(preview.clone())),
        );
        assert_eq!(
            app.kernel_debug.instrumentation_preview,
            Some(preview.clone())
        );
        assert!(app.kernel_debug.pending.is_none());
        let Some(Effect::KernelDebug(export)) = update(&mut app, Action::KernelDebug(A::Review))
        else {
            panic!()
        };
        assert!(
            matches!(&export.operation, KernelDebugOperation::ExportInstrumentation { expected } if **expected == preview)
        );
        update(&mut app, Action::KernelDebug(A::Cancel));
        assert!(app.kernel_debug.pending.is_some());
        assert!(app.active_dialog().is_some());
        finish(
            &mut app,
            export.generation,
            Ok(KernelDebugResult::InstrumentationExported(
                preview.draft.output.into(),
            )),
        );
        assert!(app.active_dialog().is_none());
        assert!(app.notification.as_deref().unwrap().contains("Not applied"));
        assert!(app.daemon.pty_sessions.is_empty());
        assert!(app.kernel_debug.prepared.is_none());
    }
}

#[test]
fn kernel_debug_instrumentation_cancel_edit_preset_cycle_and_scroll_invalidate_safely() {
    let mut app = form(13);
    for preset in [
        KernelInstrumentationPreset::Kcsan,
        KernelInstrumentationPreset::Ubsan,
        KernelInstrumentationPreset::Kasan,
    ] {
        update(&mut app, Action::KernelDebug(A::ChangeScope));
        assert!(
            matches!(app.active_dialog(), Some(Dialog::KernelDebug(d)) if d.draft.instrumentation.preset == preset)
        );
    }
    let (request, preview) = inspect(&mut app);
    finish(
        &mut app,
        request.generation,
        Ok(KernelDebugResult::InstrumentationPrepared(preview)),
    );
    update(&mut app, Action::KernelDebug(A::ScrollGuide(5)));
    assert!(app.kernel_debug.instrumentation_preview.is_some());
    update(&mut app, Action::KernelDebug(A::Cancel));
    assert!(app.active_dialog().is_some());
    assert!(app.kernel_debug.instrumentation_preview.is_none());
    let (request, preview) = inspect(&mut app);
    finish(
        &mut app,
        request.generation,
        Ok(KernelDebugResult::InstrumentationPrepared(preview)),
    );
    update(&mut app, Action::KernelDebug(A::Field(2)));
    update(&mut app, Action::KernelDebug(A::Clear));
    assert!(app.kernel_debug.instrumentation_preview.is_none());
    assert!(update(&mut app, Action::KernelDebug(A::Review)).is_none());
    assert!(matches!(app.active_dialog(), Some(Dialog::KernelDebug(d)) if d.error.is_some()));
}

#[test]
fn kernel_debug_instrumentation_late_changed_covered_wrong_and_error_results_never_export() {
    for case in ["cancel", "stale", "changed", "covered", "wrong", "error"] {
        let mut app = form(13);
        let (request, preview) = inspect(&mut app);
        let mut result = Ok(KernelDebugResult::InstrumentationPrepared(preview));
        match case {
            "cancel" => {
                update(&mut app, Action::KernelDebug(A::Cancel));
            }
            "stale" => app.kernel_debug.generation += 1,
            "changed" => {
                let Some(Dialog::KernelDebug(d)) = app.active_dialog_mut() else {
                    panic!()
                };
                d.draft.instrumentation.output = "/work/changed.cfg".into();
            }
            "covered" => app.command_palette_open = true,
            "wrong" => {
                result = Ok(KernelDebugResult::InstrumentationExported(
                    "/work/debug.cfg".into(),
                ))
            }
            "error" => result = Err("config unavailable".into()),
            _ => unreachable!(),
        }
        finish(&mut app, request.generation, result);
        assert!(app.kernel_debug.instrumentation_preview.is_none(), "{case}");
        assert!(app.kernel_debug.prepared.is_none());
        assert!(app.daemon.pty_sessions.is_empty());
    }
}

#[test]
fn kernel_debug_instrumentation_failed_export_returns_to_edit_and_requires_reinspection() {
    let mut app = form(14);
    let (request, preview) = inspect(&mut app);
    finish(
        &mut app,
        request.generation,
        Ok(KernelDebugResult::InstrumentationPrepared(preview)),
    );
    let Some(Effect::KernelDebug(request)) = update(&mut app, Action::KernelDebug(A::Review))
    else {
        panic!()
    };
    finish(
        &mut app,
        request.generation,
        Err("output exists; not overwritten".into()),
    );
    assert!(app.kernel_debug.instrumentation_preview.is_none());
    assert!(
        matches!(app.active_dialog(), Some(Dialog::KernelDebug(d)) if d.draft.instrumentation.preset == KernelInstrumentationPreset::Lockdep && d.error.as_deref() == Some("output exists; not overwritten"))
    );
    assert!(matches!(
        update(&mut app, Action::KernelDebug(A::Review)),
        Some(Effect::KernelDebug(KernelDebugRequest {
            operation: KernelDebugOperation::InspectInstrumentation { .. },
            ..
        }))
    ));
}

#[test]
fn kernel_debug_instrumentation_seeds_only_inventory_reported_config_never_output_or_guessed_path()
{
    let mut app = form(13);
    update(&mut app, Action::KernelDebug(A::Cancel));
    app.workspace.build_dir = Some("/build".into());
    update(&mut app, Action::KernelDebug(A::OpenSelected));
    assert!(
        matches!(app.active_dialog(), Some(Dialog::KernelDebug(d)) if d.draft.instrumentation.config.is_empty() && d.draft.instrumentation.output.is_empty())
    );
    update(&mut app, Action::KernelDebug(A::Cancel));
    app.kernel.inventory = PlatformInventoryState::Available(PlatformInventory {
        component: PlatformComponent::Kernel,
        target: "virtual/kernel".into(),
        provider: None,
        tasks: vec![],
        roots: vec![],
        dtc: None,
        limitations: vec![],
        files: vec![PlatformFile {
            path: "/build/exact/.config".into(),
            root: "/build/exact".into(),
            kind: PlatformFileKind::DotConfig,
            size_bytes: 12,
        }],
    });
    update(&mut app, Action::KernelDebug(A::OpenSelected));
    assert!(
        matches!(app.active_dialog(), Some(Dialog::KernelDebug(d)) if d.draft.instrumentation.config == "/build/exact/.config" && d.draft.instrumentation.output.is_empty())
    );
}
