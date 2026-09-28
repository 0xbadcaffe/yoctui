use super::*;

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
