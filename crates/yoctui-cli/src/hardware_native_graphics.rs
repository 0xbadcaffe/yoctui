//! Bounded SIXEL presentation for model-owned Hardware rasters.

use super::*;
use image::{Rgb, RgbImage, imageops::FilterType};
use ratatui::layout::Rect;

const FALLBACK_CELL_WIDTH: u16 = 9;
const FALLBACK_CELL_HEIGHT: u16 = 18;
const MAX_NATIVE_PIXELS: usize = 2_000_000;
const MAX_SIXEL_BYTES: usize = 8 * 1024 * 1024;
const PALETTE_COLORS: usize = 80;
const TILE_EDGE: usize = 480;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NativeImageKey {
    generation: u64,
    page: usize,
    zoom_percent: u16,
    pan_x: usize,
    pan_y: usize,
    area: Rect,
    cell_width: u16,
    cell_height: u16,
}

#[derive(Default)]
pub(crate) struct HardwareNativeGraphics {
    visible: Option<NativeImageKey>,
    pending: Option<NativeImageKey>,
}

impl HardwareNativeGraphics {
    pub(crate) fn prepare_frame(&mut self, app: &App, width: u16, height: u16) -> bool {
        let desired = desired_key(app, width, height);
        if desired == self.visible {
            self.pending = None;
            return false;
        }
        let clear = self.visible.is_some();
        self.visible = None;
        self.pending = desired;
        clear
    }

    pub(crate) fn paint_frame(&mut self, app: &App, width: u16, height: u16) {
        let Some(key) = self.pending.take() else {
            return;
        };
        let Some(projection) = yoctui_ui::hardware_native_raster_projection(app, width, height)
        else {
            return;
        };
        match native_page(
            projection.viewer,
            projection.raster,
            projection.area,
            key.cell_width,
            key.cell_height,
        )
        .and_then(|page| write_sixel(page, projection.area, key.cell_width, key.cell_height))
        {
            Ok(()) => self.visible = Some(key),
            Err(message) => tracing::warn!(%message, "could not render native Hardware page"),
        }
    }
}

fn desired_key(app: &App, width: u16, height: u16) -> Option<NativeImageKey> {
    let projection = yoctui_ui::hardware_native_raster_projection(app, width, height)?;
    let (cell_width, cell_height) = terminal_cell_pixels(width, height);
    Some(NativeImageKey {
        generation: projection.viewer.generation,
        page: projection.viewer.page,
        zoom_percent: projection.viewer.zoom_percent,
        pan_x: projection.viewer.pan_x,
        pan_y: projection.viewer.pan_y,
        area: projection.area,
        cell_width,
        cell_height,
    })
}

fn terminal_cell_pixels(columns: u16, rows: u16) -> (u16, u16) {
    crossterm::terminal::window_size()
        .ok()
        .and_then(|window| {
            let width = window.width.checked_div(columns)?;
            let height = window.height.checked_div(rows)?;
            (width > 0 && height > 0).then_some((width, height))
        })
        .unwrap_or((FALLBACK_CELL_WIDTH, FALLBACK_CELL_HEIGHT))
}

struct NativePage {
    image: RgbImage,
    column_offset: u16,
    row_offset: u16,
}

fn native_page(
    viewer: &yoctui_model::HardwareViewerState,
    raster: &yoctui_model::HardwareRaster,
    area: Rect,
    cell_width: u16,
    cell_height: u16,
) -> Result<NativePage, String> {
    let viewport_width = usize::from(area.width) * usize::from(cell_width);
    let viewport_height = usize::from(area.height) * usize::from(cell_height);
    if viewport_width == 0 || viewport_height == 0 {
        return Err("native Hardware viewport is empty".into());
    }
    let fit = (viewport_width as f64 / raster.width as f64)
        .min(viewport_height as f64 / raster.height as f64);
    let scale = fit * f64::from(viewer.zoom_percent) / 100.0;
    let mut target_width = (raster.width as f64 * scale).round().max(1.0) as usize;
    let mut target_height = (raster.height as f64 * scale).round().max(1.0) as usize;
    constrain_pixels(&mut target_width, &mut target_height);

    let source = RgbImage::from_fn(raster.width as u32, raster.height as u32, |x, y| {
        let pixel = raster.pixels[y as usize * raster.width + x as usize];
        Rgb([pixel.red, pixel.green, pixel.blue])
    });
    let scaled = image::imageops::resize(
        &source,
        target_width as u32,
        target_height as u32,
        FilterType::Lanczos3,
    );
    let visible_width = target_width.min(viewport_width);
    let visible_height = target_height.min(viewport_height);
    let crop_x = viewer
        .pan_x
        .saturating_mul(usize::from(cell_width))
        .min(target_width.saturating_sub(visible_width));
    let crop_y = viewer
        .pan_y
        .saturating_mul(usize::from(cell_height))
        .min(target_height.saturating_sub(visible_height));
    let image = image::imageops::crop_imm(
        &scaled,
        crop_x as u32,
        crop_y as u32,
        visible_width as u32,
        visible_height as u32,
    )
    .to_image();
    Ok(NativePage {
        image,
        column_offset: u16::try_from(
            viewport_width.saturating_sub(visible_width) / 2 / usize::from(cell_width),
        )
        .unwrap_or(0),
        row_offset: u16::try_from(
            viewport_height.saturating_sub(visible_height) / 2 / usize::from(cell_height),
        )
        .unwrap_or(0),
    })
}

fn constrain_pixels(width: &mut usize, height: &mut usize) {
    let pixels = width.saturating_mul(*height);
    if pixels <= MAX_NATIVE_PIXELS {
        return;
    }
    let factor = (MAX_NATIVE_PIXELS as f64 / pixels as f64).sqrt();
    *width = (*width as f64 * factor).floor().max(1.0) as usize;
    *height = (*height as f64 * factor).floor().max(1.0) as usize;
}

fn write_sixel(
    page: NativePage,
    area: Rect,
    cell_width: u16,
    cell_height: u16,
) -> Result<(), String> {
    let payload = sixel_frame(&page, area, cell_width, cell_height)?;
    let mut output = io::stdout();
    output
        .write_all(&payload)
        .and_then(|()| output.flush())
        .map_err(|error| error.to_string())
}

fn sixel_frame(
    page: &NativePage,
    area: Rect,
    cell_width: u16,
    cell_height: u16,
) -> Result<Vec<u8>, String> {
    let cell_width = usize::from(cell_width.max(1));
    let cell_height = usize::from(cell_height.max(1));
    // XTerm commonly limits each image to 1000x1000 pixels. Tile at cell
    // boundaries so large pages remain complete without changing terminal settings.
    let tile_width = (TILE_EDGE / cell_width).max(1) * cell_width;
    let tile_height = (TILE_EDGE / cell_height).max(1) * cell_height;
    let width = page.image.width() as usize;
    let height = page.image.height() as usize;
    let mut output = b"\x1b7".to_vec();
    for y in (0..height).step_by(tile_height) {
        for x in (0..width).step_by(tile_width) {
            let tile = image::imageops::crop_imm(
                &page.image,
                x as u32,
                y as u32,
                (width - x).min(tile_width) as u32,
                (height - y).min(tile_height) as u32,
            )
            .to_image();
            let column = usize::from(area.x) + usize::from(page.column_offset) + x / cell_width + 1;
            let row = usize::from(area.y) + usize::from(page.row_offset) + y / cell_height + 1;
            output.extend_from_slice(format!("\x1b[{row};{column}H").as_bytes());
            output.extend(encode_sixel(&tile)?);
            if output.len() > MAX_SIXEL_BYTES - 2 {
                return Err("native Hardware frame exceeds the SIXEL byte limit".into());
            }
        }
    }
    output.extend_from_slice(b"\x1b8");
    Ok(output)
}

fn encode_sixel(image: &RgbImage) -> Result<Vec<u8>, String> {
    let width = image.width() as usize;
    let height = image.height() as usize;
    let mut output = format!("\x1bP0;1;0q\"1;1;{width};{height}").into_bytes();
    append_palette(&mut output);
    let mut planes = vec![vec![0_u8; width]; PALETTE_COLORS];
    for band_y in (0..height).step_by(6) {
        for plane in &mut planes {
            plane.fill(0);
        }
        for bit in 0..6 {
            let y = band_y + bit;
            if y >= height {
                break;
            }
            let row = image.rows().nth(y).expect("bounded SIXEL row exists");
            for (x, pixel) in row.enumerate() {
                let index = palette_index(pixel.0);
                planes[index][x] |= 1 << bit;
            }
        }
        for (index, plane) in planes.iter().enumerate() {
            let Some(last) = plane.iter().rposition(|mask| *mask != 0) else {
                continue;
            };
            output.extend_from_slice(format!("#{index}").as_bytes());
            append_runs(&mut output, &plane[..=last]);
            output.push(b'$');
            if output.len() > MAX_SIXEL_BYTES {
                return Err("native Hardware image exceeds the SIXEL byte limit".into());
            }
        }
        output.push(b'-');
    }
    output.extend_from_slice(b"\x1b\\");
    Ok(output)
}

fn append_palette(output: &mut Vec<u8>) {
    for red in 0..4 {
        for green in 0..4 {
            for blue in 0..4 {
                let index = red * 16 + green * 4 + blue;
                output.extend_from_slice(
                    format!(
                        "#{index};2;{};{};{}",
                        red * 100 / 3,
                        green * 100 / 3,
                        blue * 100 / 3
                    )
                    .as_bytes(),
                );
            }
        }
    }
    for gray in 0..16 {
        let value = gray * 100 / 15;
        output.extend_from_slice(format!("#{};2;{value};{value};{value}", 64 + gray).as_bytes());
    }
}

fn palette_index([red, green, blue]: [u8; 3]) -> usize {
    let maximum = red.max(green).max(blue);
    let minimum = red.min(green).min(blue);
    if maximum.saturating_sub(minimum) < 24 {
        let luminance = (u16::from(red) * 30 + u16::from(green) * 59 + u16::from(blue) * 11) / 100;
        return 64 + usize::from(luminance) * 15 / 255;
    }
    usize::from(red) * 3 / 255 * 16 + usize::from(green) * 3 / 255 * 4 + usize::from(blue) * 3 / 255
}

fn append_runs(output: &mut Vec<u8>, masks: &[u8]) {
    let mut start = 0;
    while start < masks.len() {
        let mask = masks[start];
        let mut end = start + 1;
        while end < masks.len() && masks[end] == mask {
            end += 1;
        }
        let count = end - start;
        let sixel = mask.saturating_add(63);
        if count >= 4 {
            output.extend_from_slice(format!("!{count}").as_bytes());
            output.push(sixel);
        } else {
            output.extend(std::iter::repeat_n(sixel, count));
        }
        start = end;
    }
}

#[cfg(test)]
#[path = "tests/hardware_native_graphics.rs"]
mod tests;
