use super::*;

fn document(name: &str, category: HardwareCategory) -> HardwareDocument {
    HardwareDocument {
        path: PathBuf::from(format!("/tmp/{name}.pdf")),
        category,
        kind: HardwareDocumentKind::Pdf,
    }
}

#[test]
fn hardware_catalog_categories_selection_and_persistence_are_bounded() {
    let mut app = App::new(100, 100_000);
    let board = document("board", HardwareCategory::Board);
    let sensor = document("sensor", HardwareCategory::Sensors);
    update(
        &mut app,
        Action::Hardware(HardwareAction::Install(vec![board.clone(), sensor.clone()])),
    );
    assert_eq!(app.hardware.selected_document(), Some(&board));
    update(
        &mut app,
        Action::Hardware(HardwareAction::SelectCategory { delta: 4 }),
    );
    assert_eq!(app.hardware.selected_document(), Some(&sensor));
    update(&mut app, Action::Hardware(HardwareAction::BeginRemove));
    assert_eq!(
        update(&mut app, Action::Hardware(HardwareAction::ConfirmRemove)),
        Some(Effect::Hardware(HardwareEffect::Persist(vec![board])))
    );
}

#[test]
fn hardware_viewer_rejects_stale_and_invalid_raster_results() {
    let mut app = App::new(100, 100_000);
    update(
        &mut app,
        Action::Hardware(HardwareAction::Install(vec![document(
            "board",
            HardwareCategory::Board,
        )])),
    );
    let effect = update(&mut app, Action::Hardware(HardwareAction::OpenSelected));
    assert!(matches!(
        effect,
        Some(Effect::Hardware(HardwareEffect::Load(_)))
    ));
    update(
        &mut app,
        Action::Hardware(HardwareAction::PreviewLoaded {
            generation: 99,
            page_count: 1,
            preview: HardwarePreview::Text {
                lines: vec!["stale".into()],
                limitation: None,
            },
            searchable_text: vec![],
        }),
    );
    assert!(app.hardware.viewer.as_ref().unwrap().loading);
    update(
        &mut app,
        Action::Hardware(HardwareAction::PreviewLoaded {
            generation: 1,
            page_count: 1,
            preview: HardwarePreview::Raster(HardwareRaster {
                width: 2,
                height: 2,
                pixels: vec![],
            }),
            searchable_text: vec![],
        }),
    );
    assert!(app.hardware.viewer.as_ref().unwrap().error.is_some());
}

#[test]
fn hardware_search_zoom_page_and_pan_transitions_are_pure() {
    let mut app = App::new(100, 100_000);
    update(
        &mut app,
        Action::Hardware(HardwareAction::Install(vec![document(
            "board",
            HardwareCategory::Board,
        )])),
    );
    update(&mut app, Action::Hardware(HardwareAction::OpenSelected));
    update(
        &mut app,
        Action::Hardware(HardwareAction::PreviewLoaded {
            generation: 1,
            page_count: 3,
            preview: HardwarePreview::Text {
                lines: vec!["alpha".into(), "beta alpha".into()],
                limitation: None,
            },
            searchable_text: vec!["alpha".into(), "beta alpha".into()],
        }),
    );
    update(&mut app, Action::Hardware(HardwareAction::BeginSearch));
    for value in "alpha".chars() {
        update(
            &mut app,
            Action::Hardware(HardwareAction::AppendSearch(value)),
        );
    }
    update(
        &mut app,
        Action::Hardware(HardwareAction::NextMatch { backwards: false }),
    );
    update(
        &mut app,
        Action::Hardware(HardwareAction::Zoom { delta: 50 }),
    );
    update(
        &mut app,
        Action::Hardware(HardwareAction::Pan {
            horizontal: 3,
            vertical: 2,
        }),
    );
    let viewer = app.hardware.viewer.as_ref().unwrap();
    assert_eq!(
        (
            viewer.match_selection,
            viewer.pan_x,
            viewer.pan_y,
            viewer.zoom_percent
        ),
        (1, 3, 3, 150)
    );
    assert!(matches!(
        update(&mut app, Action::Hardware(HardwareAction::LastPage)),
        Some(Effect::Hardware(HardwareEffect::Load(
            HardwareLoadRequest { page: 3, .. }
        )))
    ));
}

#[test]
fn hardware_pdf_presentation_uses_capability_and_close_preserves_selection() {
    let mut app = App::new(100, 100_000);
    update(
        &mut app,
        Action::Hardware(HardwareAction::Install(vec![document(
            "board",
            HardwareCategory::Board,
        )])),
    );
    update(&mut app, Action::Hardware(HardwareAction::OpenSelected));
    assert_eq!(
        app.hardware.viewer.as_ref().unwrap().presentation,
        HardwarePresentation::Text
    );
    update(
        &mut app,
        Action::Hardware(HardwareAction::TogglePresentation),
    );
    assert_eq!(
        app.hardware.viewer.as_ref().unwrap().presentation,
        HardwarePresentation::Page
    );
    update(&mut app, Action::Hardware(HardwareAction::CloseViewer));
    assert!(app.hardware.viewer.is_none());
    assert_eq!(app.hardware.selection, 0);

    update(
        &mut app,
        Action::Hardware(HardwareAction::GraphicsCapabilityDetected(
            HardwareGraphicsCapability::Sixel,
        )),
    );
    update(&mut app, Action::Hardware(HardwareAction::OpenSelected));
    assert_eq!(
        app.hardware.viewer.as_ref().unwrap().presentation,
        HardwarePresentation::Page
    );
}

#[test]
fn hardware_pdf_without_readable_text_selects_raster_page_fallback() {
    let mut app = App::new(100, 100_000);
    update(
        &mut app,
        Action::Hardware(HardwareAction::Install(vec![document(
            "board",
            HardwareCategory::Board,
        )])),
    );
    update(&mut app, Action::Hardware(HardwareAction::OpenSelected));
    update(
        &mut app,
        Action::Hardware(HardwareAction::PreviewLoaded {
            generation: 1,
            page_count: 71,
            preview: HardwarePreview::Raster(HardwareRaster {
                width: 1,
                height: 1,
                pixels: vec![HardwareRgb {
                    red: 255,
                    green: 255,
                    blue: 255,
                }],
            }),
            searchable_text: Vec::new(),
        }),
    );
    let viewer = app.hardware.viewer.as_ref().unwrap();
    assert_eq!(viewer.presentation, HardwarePresentation::Page);
    assert_eq!(viewer.page_count, 71);
}
