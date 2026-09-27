//! Fit, zoom, and resample bounded Hardware rasters into terminal half-blocks.

use super::*;

pub(super) fn render_raster_preview(
    frame: &mut Frame,
    area: Rect,
    viewer: &yoctui_model::HardwareViewerState,
    raster: &yoctui_model::HardwareRaster,
) {
    let geometry = raster_geometry(area, viewer, raster);
    let lines = (0..usize::from(area.height))
        .map(|row| {
            Line::from(
                (0..usize::from(area.width))
                    .map(|column| {
                        let top = geometry.sample(raster, column, row.saturating_mul(2));
                        let bottom = geometry.sample(
                            raster,
                            column,
                            row.saturating_mul(2).saturating_add(1),
                        );
                        raster_span(top, bottom)
                    })
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(lines), area);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RasterGeometry {
    pub(crate) target_width: usize,
    pub(crate) target_height: usize,
    pub(crate) offset_x: usize,
    pub(crate) offset_y: usize,
    pan_x: usize,
    pan_y: usize,
}

pub(crate) fn raster_geometry(
    area: Rect,
    viewer: &yoctui_model::HardwareViewerState,
    raster: &yoctui_model::HardwareRaster,
) -> RasterGeometry {
    let viewport_width = usize::from(area.width).max(1);
    let viewport_height = usize::from(area.height).saturating_mul(2).max(1);
    let source_width = raster.width.max(1);
    let source_height = raster.height.max(1);
    let fit = (viewport_width as f64 / source_width as f64)
        .min(viewport_height as f64 / source_height as f64);
    let scale = fit * f64::from(viewer.zoom_percent) / 100.0;
    let target_width = (source_width as f64 * scale).round().max(1.0) as usize;
    let target_height = (source_height as f64 * scale).round().max(1.0) as usize;
    RasterGeometry {
        target_width,
        target_height,
        offset_x: viewport_width.saturating_sub(target_width) / 2,
        offset_y: viewport_height.saturating_sub(target_height) / 2,
        pan_x: viewer
            .pan_x
            .min(target_width.saturating_sub(viewport_width)),
        pan_y: viewer
            .pan_y
            .min(target_height.saturating_sub(viewport_height)),
    }
}

impl RasterGeometry {
    fn sample(
        self,
        raster: &yoctui_model::HardwareRaster,
        viewport_x: usize,
        viewport_y: usize,
    ) -> Option<yoctui_model::HardwareRgb> {
        let target_x = viewport_x
            .saturating_add(self.pan_x)
            .checked_sub(self.offset_x)?;
        let target_y = viewport_y
            .saturating_add(self.pan_y)
            .checked_sub(self.offset_y)?;
        if target_x >= self.target_width || target_y >= self.target_height {
            return None;
        }
        let source_x = sample_coordinate(target_x, self.target_width, raster.width);
        let source_y = sample_coordinate(target_y, self.target_height, raster.height);
        Some(bilinear_pixel(raster, source_x, source_y))
    }
}

fn sample_coordinate(target: usize, target_size: usize, source_size: usize) -> f64 {
    (((target as f64 + 0.5) * source_size as f64 / target_size as f64) - 0.5)
        .clamp(0.0, source_size.saturating_sub(1) as f64)
}

fn bilinear_pixel(
    raster: &yoctui_model::HardwareRaster,
    x: f64,
    y: f64,
) -> yoctui_model::HardwareRgb {
    let x0 = x.floor() as usize;
    let y0 = y.floor() as usize;
    let x1 = x0.saturating_add(1).min(raster.width.saturating_sub(1));
    let y1 = y0.saturating_add(1).min(raster.height.saturating_sub(1));
    let horizontal = x - x0 as f64;
    let vertical = y - y0 as f64;
    let top_left = raster.pixels[y0 * raster.width + x0];
    let top_right = raster.pixels[y0 * raster.width + x1];
    let bottom_left = raster.pixels[y1 * raster.width + x0];
    let bottom_right = raster.pixels[y1 * raster.width + x1];
    yoctui_model::HardwareRgb {
        red: blend_channel(
            top_left.red,
            top_right.red,
            bottom_left.red,
            bottom_right.red,
            horizontal,
            vertical,
        ),
        green: blend_channel(
            top_left.green,
            top_right.green,
            bottom_left.green,
            bottom_right.green,
            horizontal,
            vertical,
        ),
        blue: blend_channel(
            top_left.blue,
            top_right.blue,
            bottom_left.blue,
            bottom_right.blue,
            horizontal,
            vertical,
        ),
    }
}

fn blend_channel(
    top_left: u8,
    top_right: u8,
    bottom_left: u8,
    bottom_right: u8,
    horizontal: f64,
    vertical: f64,
) -> u8 {
    let top = f64::from(top_left) * (1.0 - horizontal) + f64::from(top_right) * horizontal;
    let bottom = f64::from(bottom_left) * (1.0 - horizontal) + f64::from(bottom_right) * horizontal;
    (top * (1.0 - vertical) + bottom * vertical).round() as u8
}

fn raster_span(
    top: Option<yoctui_model::HardwareRgb>,
    bottom: Option<yoctui_model::HardwareRgb>,
) -> Span<'static> {
    match (top, bottom) {
        (Some(top), Some(bottom)) => Span::styled(
            "▀",
            Style::default()
                .fg(Color::Rgb(top.red, top.green, top.blue))
                .bg(Color::Rgb(bottom.red, bottom.green, bottom.blue)),
        ),
        (Some(top), None) => Span::styled(
            "▀",
            Style::default().fg(Color::Rgb(top.red, top.green, top.blue)),
        ),
        (None, Some(bottom)) => Span::styled(
            "▄",
            Style::default().fg(Color::Rgb(bottom.red, bottom.green, bottom.blue)),
        ),
        (None, None) => Span::raw(" "),
    }
}
