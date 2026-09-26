use super::*;
use yoctui_model::{
    HardwareBrowserEntry, HardwareBrowserState, HardwareCategory, HardwareDocument,
    HardwareDocumentKind, HardwarePreview, HardwareViewerState,
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
fn hardware_viewer_uses_the_full_body_and_shows_search_tools() {
    let mut app = App::new(100, 100_000);
    app.screen = Screen::Hardware;
    app.focus = FocusTarget::Workspace;
    app.hardware.viewer = Some(HardwareViewerState {
        document: document(),
        generation: 1,
        page: 2,
        page_count: 4,
        zoom_percent: 125,
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
    assert!(!output.contains("Navigator"));
}
