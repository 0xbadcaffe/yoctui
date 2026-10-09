use super::*;

#[test]
fn hardware_native_pdf_zoom_and_pan_survive_pixel_cap() {
    use yoctui_model::*;
    let raster = HardwareRaster {
        width: 512,
        height: 512,
        pixels: (0..512 * 512)
            .map(|index| HardwareRgb {
                red: (index % 512 / 2) as u8,
                green: (index / 512 / 2) as u8,
                blue: 0,
            })
            .collect(),
    };
    let mut viewer = HardwareViewerState {
        document: HardwareDocument {
            path: "/tmp/board.pdf".into(),
            category: HardwareCategory::Board,
            kind: HardwareDocumentKind::Pdf,
        },
        generation: 1,
        page: 1,
        page_count: 1,
        zoom_percent: 100,
        fit_width: false,
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
    };
    // A high-DPI viewport exceeds the cap even at fit-to-page zoom.
    let area = Rect::new(0, 0, 200, 100);
    let render =
        |viewer: &HardwareViewerState| native_page(viewer, &raster, area, 20, 40).unwrap().image;
    let fitted = render(&viewer);
    for zoom in [125, 200, 400] {
        viewer.zoom_percent = zoom;
        let enlarged = render(&viewer);
        assert!((enlarged.width() as usize * enlarged.height() as usize) <= MAX_NATIVE_PIXELS);
        assert_ne!(enlarged, fitted, "zoom {zoom} must change PDF content");
        assert!(
            enlarged.get_pixel(enlarged.width() - 1, 0)[0]
                < fitted.get_pixel(fitted.width() - 1, 0)[0]
        );
    }
    let enlarged = render(&viewer);
    viewer.pan_x = 10;
    viewer.pan_y = 10;
    assert_ne!(render(&viewer), enlarged);
    viewer.zoom_percent = 100;
    viewer.pan_x = 0;
    viewer.pan_y = 0;
    assert_eq!(render(&viewer), fitted);
    viewer.zoom_percent = 50;
    assert!(render(&viewer).width() < fitted.width());
}

#[test]
fn sixel_encoder_is_bounded_and_declares_raster_geometry() {
    let image = RgbImage::from_fn(8, 7, |x, y| {
        if (x + y) % 2 == 0 {
            Rgb([0, 0, 0])
        } else {
            Rgb([255, 255, 255])
        }
    });
    let encoded = encode_sixel(&image).unwrap();
    assert!(encoded.starts_with(b"\x1bP0;1;0q\"1;1;8;7"));
    assert!(encoded.ends_with(b"\x1b\\"));
    assert!(encoded.len() < MAX_SIXEL_BYTES);
}

#[test]
fn hardware_large_native_page_is_tiled_without_losing_edges() {
    let page = NativePage {
        image: RgbImage::from_pixel(1000, 1100, Rgb([255, 255, 255])),
        column_offset: 2,
        row_offset: 1,
    };
    let output = sixel_frame(&page, Rect::new(22, 7, 160, 50), 9, 18).unwrap();
    let output = String::from_utf8(output).unwrap();
    assert!(output.starts_with("\x1b7\x1b[9;25H"));
    assert!(output.ends_with("\x1b8"));
    assert_eq!(output.matches("\x1bP0;1;0q").count(), 9);
    // Tiles align to whole 9x18 cells: 477x468, then the final 46x164 edge.
    assert!(output.contains("\"1;1;477;468"));
    assert!(output.contains("\x1b[61;131H\x1bP0;1;0q\"1;1;46;164"));
    assert!(output.len() < MAX_SIXEL_BYTES);
}

fn pdf_viewer_fixture() -> yoctui_model::HardwareViewerState {
    use yoctui_model::*;
    let mut app = App::new(16, 1024);
    app.hardware.documents.push(HardwareDocument {
        path: "/tmp/board.pdf".into(),
        category: HardwareCategory::Board,
        kind: HardwareDocumentKind::Pdf,
    });
    let _ = update(&mut app, Action::Hardware(HardwareAction::OpenSelected));
    app.hardware.viewer.take().unwrap()
}

#[test]
fn hardware_native_zoom_keeps_center_and_pan_has_no_hidden_overscroll() {
    let raster = yoctui_model::HardwareRaster {
        width: 512,
        height: 512,
        pixels: Vec::new(),
    };
    let area = Rect::new(22, 7, 120, 30);
    let mut viewer = pdf_viewer_fixture();
    let previous = NativeImageKey {
        generation: viewer.generation,
        page: viewer.page,
        zoom_percent: 100,
        fit_width: false,
        pan_x: 0,
        pan_y: 0,
        area,
        cell_width: 9,
        cell_height: 18,
    };
    viewer.zoom_percent = 200;
    let pan = normalized_pan(&viewer, &raster, area, (9, 18), Some(previous));
    assert_eq!(
        pan,
        (0, 15),
        "zoom should retain the page's vertical center"
    );
    viewer.pan_x = usize::MAX;
    viewer.pan_y = usize::MAX;
    let clamped = normalized_pan(&viewer, &raster, area, (9, 18), None);
    assert_eq!(clamped, (0, 30));
    viewer.pan_x = clamped.0;
    viewer.pan_y = clamped.1 - 2;
    assert_eq!(
        normalized_pan(&viewer, &raster, area, (9, 18), None),
        (0, 28)
    );
    viewer.zoom_percent = 100;
    viewer.pan_x = 0;
    viewer.pan_y = 0;
    assert_eq!(
        normalized_pan(&viewer, &raster, area, (9, 18), Some(previous)),
        (0, 0)
    );
}

#[test]
fn hardware_native_fit_width_uses_full_width_and_preserves_aspect() {
    let raster = yoctui_model::HardwareRaster {
        width: 512,
        height: 512,
        pixels: Vec::new(),
    };
    let area = Rect::new(22, 7, 120, 30);
    let fitted = viewport_geometry(&raster, area, (9, 18), 100, false);
    let width = viewport_geometry(&raster, area, (9, 18), 100, true);
    assert_eq!((fitted.target_width, fitted.target_height), (540, 540));
    assert_eq!((width.target_width, width.target_height), (1080, 1080));
    assert_eq!((width.visible_width, width.visible_height), (1080, 540));
}

#[test]
fn hardware_native_mouse_zoom_drag_respects_overlay_and_viewport_ownership() {
    use crossterm::event::{KeyModifiers as M, MouseButton as B, MouseEvent, MouseEventKind as K};
    use yoctui_model::*;
    let mut app = App::new(16, 1024);
    app.screen = Screen::Hardware;
    app.hardware.graphics_capability = HardwareGraphicsCapability::Sixel;
    let mut viewer = pdf_viewer_fixture();
    viewer.loading = false;
    viewer.preview = Some(HardwarePreview::Raster(HardwareRaster {
        width: 1,
        height: 1,
        pixels: vec![HardwareRgb {
            red: 255,
            green: 255,
            blue: 255,
        }],
    }));
    app.hardware.viewer = Some(viewer);
    let mut graphics = HardwareNativeGraphics::default();
    let event = |kind, column, row, modifiers| MouseEvent {
        kind,
        column,
        row,
        modifiers,
    };
    assert_eq!(
        graphics.mouse_action(&app, 160, 48, event(K::ScrollUp, 50, 15, M::CONTROL)),
        Some(Action::Hardware(HardwareAction::Zoom { delta: 25 }))
    );
    assert!(
        graphics
            .mouse_action(&app, 160, 48, event(K::ScrollUp, 5, 15, M::CONTROL))
            .is_none()
    );
    assert!(
        graphics
            .mouse_action(&app, 160, 48, event(K::Down(B::Left), 50, 15, M::NONE))
            .is_none()
    );
    assert_eq!(
        graphics.mouse_action(&app, 160, 48, event(K::Drag(B::Left), 46, 13, M::NONE)),
        Some(Action::Hardware(HardwareAction::Pan {
            horizontal: 4,
            vertical: 2
        }))
    );
    assert!(
        graphics
            .mouse_action(&app, 160, 48, event(K::Up(B::Left), 46, 13, M::NONE))
            .is_none()
    );
    assert!(
        graphics
            .mouse_action(&app, 160, 48, event(K::Drag(B::Left), 40, 10, M::NONE))
            .is_none()
    );
    app.command_palette_open = true;
    assert!(
        graphics
            .mouse_action(&app, 160, 48, event(K::ScrollUp, 50, 15, M::CONTROL))
            .is_none()
    );
    app.command_palette_open = false;
    app.preferences.mouse_enabled = false;
    assert!(
        graphics
            .mouse_action(&app, 160, 48, event(K::ScrollUp, 50, 15, M::CONTROL))
            .is_none()
    );
}
