use super::*;

#[test]
fn compact_rootfs_keeps_last_selected_package_and_exact_values_visible() {
    let mut app = concept_rootfs_app();
    app.rootfs_package_selection = Some(yoctui_model::PackageIdentity::new("base-system-089"));
    for (width, height) in [
        (80, 24),
        (99, 24),
        (100, 24),
        (120, 30),
        (129, 30),
        (130, 24),
        (140, 30),
        (160, 24),
    ] {
        let output = rendered_text(&app, width, height);
        assert!(
            output.contains("base-system-089"),
            "selected package lost at {width}x{height}: {output}"
        );
        assert!(
            output.contains("520000"),
            "exact selected bytes lost: {output}"
        );
        assert!(output.contains("Exact bytes"), "{output}");
        assert!(output.contains("j/k package"), "{output}");
    }
}

#[test]
fn hiding_inspector_expands_workspace_on_every_wide_screen() {
    let mut app = concept_rootfs_app();
    for screen in [
        Screen::Dashboard,
        Screen::Insights,
        Screen::Tasks,
        Screen::BuildHistory,
        Screen::Dependencies,
        Screen::Signatures,
        Screen::LayerRelationships,
        Screen::Recipes,
        Screen::Devtool,
        Screen::Packages,
        Screen::Images,
        Screen::Hardware,
        Screen::Kernel,
        Screen::Firmware,
        Screen::Sdk,
        Screen::Testing,
        Screen::Security,
        Screen::Qa,
        Screen::Layers,
        Screen::Configuration,
        Screen::Bbmask,
        Screen::RawMode,
        Screen::TerminalSessions,
        Screen::Maintenance,
        Screen::Logs,
        Screen::Errors,
        Screen::Help,
        Screen::BuildEnvironment,
        Screen::Compatibility,
        Screen::Settings,
    ] {
        app.screen = screen;
        app.inspector_visible = true;
        let before = yoctui_app::workbench_pane_widths(&app, 160, 50);
        let _ = update(&mut app, Action::ToggleInspector);
        let after = yoctui_app::workbench_pane_widths(&app, 160, 50);
        assert_eq!(after, [before[0], before[1] + before[2], 0], "{screen:?}");
        // The production renderer must accept the new width on every screen.
        let _ = rendered_text(&app, 160, 50);
        let _ = update(&mut app, Action::ToggleInspector);
        assert_eq!(yoctui_app::workbench_pane_widths(&app, 160, 50), before);
    }
}

#[test]
fn opening_pdf_reveals_native_viewport_and_zoom_matches_small_screen_layout() {
    use yoctui_model::{
        HardwareAction, HardwareCategory, HardwareDocument, HardwareDocumentKind, HardwarePreview,
        HardwareRaster, HardwareRgb,
    };
    let mut app = App::new(100, 100_000);
    app.screen = Screen::Hardware;
    app.hardware.documents.push(HardwareDocument {
        path: "/tmp/board.pdf".into(),
        category: HardwareCategory::Board,
        kind: HardwareDocumentKind::Pdf,
    });
    app.hardware.graphics_capability = yoctui_model::HardwareGraphicsCapability::Sixel;
    let _ = update(&mut app, Action::Hardware(HardwareAction::OpenSelected));
    assert_eq!(app.focus, FocusTarget::Workspace);
    let generation = app.hardware.viewer.as_ref().unwrap().generation;
    let _ = update(
        &mut app,
        Action::Hardware(HardwareAction::PreviewLoaded {
            generation,
            page_count: 2,
            preview: HardwarePreview::Raster(HardwareRaster {
                width: 1,
                height: 1,
                pixels: vec![HardwareRgb {
                    red: 255,
                    green: 255,
                    blue: 255,
                }],
            }),
            searchable_text: vec!["board text".into()],
        }),
    );
    app.notification = Some("Terminal pane zoom toggled".into());
    assert!(crate::hardware_native_raster_projection(&app, 80, 24).is_some());
    app.notification = Some("No hardware graphics available".into());
    assert!(crate::hardware_native_raster_projection(&app, 80, 24).is_none());
    app.notification = None;
    app.onboarding.open = true;
    assert!(crate::hardware_native_raster_projection(&app, 80, 24).is_none());
    app.onboarding.open = false;
    app.keymap_preferences_ui.open = true;
    assert!(crate::hardware_native_raster_projection(&app, 80, 24).is_none());
    app.keymap_preferences_ui.open = false;
    for (width, height) in [
        (80, 24),
        (99, 24),
        (100, 30),
        (129, 30),
        (130, 30),
        (160, 50),
    ] {
        for zoomed in [false, true] {
            app.zoomed_pane = zoomed.then_some(FocusTarget::Workspace);
            let output = rendered_text(&app, width, height);
            assert!(
                output.contains("board.pdf"),
                "PDF hidden at {width}x{height}: {output}"
            );
            let projection = crate::hardware_native_raster_projection(&app, width, height).unwrap();
            assert_eq!(
                projection.area.x,
                if zoomed || width < 100 { 0 } else { 22 }
            );
            assert_eq!(projection.area.y, if zoomed || width < 100 { 8 } else { 7 });
            assert_eq!(projection.area.bottom(), height - 5);
            assert!(projection.area.right() <= width);
            assert!(!projection.area.is_empty());
        }
        app.zoomed_pane = Some(FocusTarget::Navigator);
        assert!(crate::hardware_native_raster_projection(&app, width, height).is_none());
    }
}

#[test]
fn opening_rootfs_from_navigator_reveals_compact_workspace() {
    for action in [
        Action::BeginSelectedRootfsComposition,
        Action::ShiftImagesView { delta: 1 },
    ] {
        let mut app = concept_rootfs_app();
        app.focus = FocusTarget::Navigator;
        app.images_view = ImagesView::Artifacts;
        let _ = update(&mut app, action);
        assert_eq!(app.focus, FocusTarget::Workspace);
        assert!(rendered_text(&app, 80, 24).contains("Rootfs composition"));
    }
}

#[test]
fn preferences_keep_each_selected_setting_default_and_reset_controls_visible() {
    let mut app = App::new(32, 4096);
    app.screen = Screen::Settings;
    app.focus = FocusTarget::Workspace;
    for (width, height) in [(80, 24), (100, 30), (160, 50)] {
        for (index, row) in app.preference_rows().iter().enumerate() {
            app.settings_selection = index;
            let output = rendered_text(&app, width, height);
            assert!(
                output.contains(row.label),
                "{} at {width}x{height}: {output}",
                row.label
            );
            assert!(output.contains("default:"), "{output}");
            assert!(output.contains("Backspace reset selected"), "{output}");
        }
        app.settings_selection = yoctui_model::SETTINGS
            .iter()
            .position(|s| *s == yoctui_model::Setting::Inspector)
            .unwrap();
        let output = rendered_text(&app, width, height);
        assert!(output.contains("false"), "{output}");
        let _ = update(&mut app, Action::ChangeSelectedSetting { backwards: false });
        let output = rendered_text(&app, width, height);
        assert!(output.contains("custom"), "{output}");
        assert!(output.contains("true"), "{output}");
        let _ = update(&mut app, Action::ResetSelectedPreference);
    }
}
