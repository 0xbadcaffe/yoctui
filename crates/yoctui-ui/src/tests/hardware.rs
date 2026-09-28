use super::*;
use yoctui_model::{
    HardwareBrowserEntry, HardwareBrowserState, HardwareCategory, HardwareDocument,
    HardwareDocumentKind, HardwarePresentation, HardwarePreview, HardwareViewerState,
};

fn document() -> HardwareDocument {
    HardwareDocument {
        path: PathBuf::from("/tmp/romulus-board.pdf"),
        category: HardwareCategory::Board,
        kind: HardwareDocumentKind::Pdf,
    }
}

#[test]
fn hardware_library_and_browser_render_responsively() {
    let mut app = App::new(100, 100_000);
    app.screen = Screen::Hardware;
    app.focus = FocusTarget::Workspace;
    app.hardware.documents.push(document());
    let wide = rendered_text(&app, 160, 42);
    assert!(wide.contains("Hardware · document library"));
    assert!(wide.contains("romulus-board.pdf"));

    app.hardware.browser = Some(HardwareBrowserState {
        directory: PathBuf::from("/tmp"),
        entries: vec![HardwareBrowserEntry {
            path: PathBuf::from("/tmp/board.kicad_sch"),
            name: "board.kicad_sch".into(),
            is_directory: false,
            kind: Some(HardwareDocumentKind::Kicad),
        }],
        selection: 0,
        category: HardwareCategory::Soc,
        generation: 1,
        loading: false,
        error: None,
    });
    let narrow = rendered_text(&app, 80, 24);
    assert!(narrow.contains("Directory: /tmp"), "{narrow}");
    assert!(narrow.contains("board.kicad_sch"), "{narrow}");
    assert!(narrow.contains("Add to: SoC"), "{narrow}");
}

#[test]
fn hardware_viewer_keeps_navigator_and_shows_search_tools() {
    let mut app = App::new(100, 100_000);
    app.screen = Screen::Hardware;
    app.focus = FocusTarget::Workspace;
    app.hardware.viewer = Some(HardwareViewerState {
        document: document(),
        generation: 1,
        page: 2,
        page_count: 4,
        zoom_percent: 125,
        presentation: HardwarePresentation::Text,
        pan_x: 0,
        pan_y: 0,
        loading: false,
        preview: Some(HardwarePreview::Text {
            lines: vec!["Romulus board schematic".into(), "power rail".into()],
            limitation: None,
        }),
        searchable_text: vec!["Romulus board schematic".into(), "power rail".into()],
        query: "power".into(),
        searching: false,
        matches: vec![1],
        match_selection: 0,
        error: None,
    });
    let output = rendered_text(&app, 120, 32);
    assert!(output.contains("page 2/4 · zoom 125%"));
    assert!(output.contains("Romulus board schematic"));
    assert!(output.contains("Search: power · 1/1"));
    assert!(output.contains("Navigator"));
    assert!(!output.contains("Inspector"));
    assert!(output.contains("Tab Navigator"));
}

#[test]
fn hardware_image_raster_keeps_complete_page_half_block_fit() {
    let mut app = App::new(100, 100_000);
    app.hardware.viewer = Some(HardwareViewerState {
        document: HardwareDocument {
            path: PathBuf::from("/tmp/board.png"),
            category: HardwareCategory::Board,
            kind: HardwareDocumentKind::Raster,
        },
        generation: 1,
        page: 1,
        page_count: 1,
        zoom_percent: 100,
        presentation: HardwarePresentation::Page,
        pan_x: 0,
        pan_y: 0,
        loading: false,
        preview: None,
        searchable_text: Vec::new(),
        query: String::new(),
        searching: false,
        matches: Vec::new(),
        match_selection: 0,
        error: None,
    });
    let raster = yoctui_model::HardwareRaster {
        width: 900,
        height: 1200,
        pixels: Vec::new(),
    };
    let fit = crate::hardware_raster_render::raster_geometry(
        Rect::new(0, 0, 200, 48),
        app.hardware.viewer.as_ref().unwrap(),
        &raster,
    );
    assert_eq!((fit.target_width, fit.target_height), (72, 96));
    assert_eq!((fit.offset_x, fit.offset_y), (64, 0));
}

#[test]
fn hardware_pdf_uses_crisp_text_without_native_graphics_and_projects_sixel_area() {
    let mut app = App::new(100, 100_000);
    app.screen = Screen::Hardware;
    app.focus = FocusTarget::Workspace;
    app.hardware.viewer = Some(HardwareViewerState {
        document: document(),
        generation: 7,
        page: 1,
        page_count: 71,
        zoom_percent: 100,
        presentation: HardwarePresentation::Text,
        pan_x: 0,
        pan_y: 0,
        loading: false,
        preview: Some(HardwarePreview::Raster(yoctui_model::HardwareRaster {
            width: 1,
            height: 1,
            pixels: vec![yoctui_model::HardwareRgb {
                red: 255,
                green: 255,
                blue: 255,
            }],
        })),
        searchable_text: vec!["SMARC carrier user guide".into()],
        query: String::new(),
        searching: false,
        matches: Vec::new(),
        match_selection: 0,
        error: None,
    });
    let fallback = rendered_text(&app, 120, 32);
    assert!(fallback.contains("Text"), "{fallback}");
    assert!(fallback.contains("SMARC carrier user guide"), "{fallback}");
    assert!(
        !fallback.contains("Native terminal graphics unavailable"),
        "{fallback}"
    );
    assert!(fallback.contains("Esc/Backspace library"), "{fallback}");

    app.hardware.viewer.as_mut().unwrap().presentation = HardwarePresentation::Page;
    for size in [(160, 45), (100, 30), (80, 24)] {
        let unavailable = rendered_text(&app, size.0, size.1);
        assert!(
            unavailable.contains("xterm -ti vt340 -e yoctui attach"),
            "{unavailable}"
        );
        assert!(
            unavailable.contains("cannot display PDF page graphics"),
            "{unavailable}"
        );
        assert!(unavailable.contains("Press v"), "{unavailable}");
        assert!(!unavailable.contains("Terminal page"));
        assert!(
            !unavailable
                .chars()
                .any(|c| ('\u{2801}'..='\u{28ff}').contains(&c))
        );
    }
    app.hardware
        .viewer
        .as_mut()
        .unwrap()
        .searchable_text
        .clear();
    let unavailable = rendered_text(&app, 160, 45);
    assert!(unavailable.contains("no readable embedded text"));
    assert!(crate::hardware_native_raster_projection(&app, 160, 45).is_none());

    app.hardware.graphics_capability = yoctui_model::HardwareGraphicsCapability::Sixel;
    app.hardware.viewer.as_mut().unwrap().presentation = HardwarePresentation::Page;
    let projection = crate::hardware_native_raster_projection(&app, 120, 32).unwrap();
    assert_eq!(projection.area.x, 22);
    assert!(projection.area.width > 0 && projection.area.height > 0);
}
