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
