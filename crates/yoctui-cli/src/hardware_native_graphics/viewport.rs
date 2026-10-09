//! Pixel geometry and bounded, center-preserving zoom for native PDF pages.
use super::*;

pub(super) struct ViewportGeometry {
    pub target_width: usize,
    pub target_height: usize,
    pub visible_width: usize,
    pub visible_height: usize,
}

pub(super) fn viewport_geometry(
    raster: &yoctui_model::HardwareRaster,
    area: Rect,
    cells: (u16, u16),
    zoom: u16,
    fit_width: bool,
) -> ViewportGeometry {
    let mut width = usize::from(area.width) * usize::from(cells.0.max(1));
    let mut height = usize::from(area.height) * usize::from(cells.1.max(1));
    // Bound the visible allocation, not the enlarged full document.
    let pixels = width.saturating_mul(height);
    if pixels > MAX_NATIVE_PIXELS {
        let factor = (MAX_NATIVE_PIXELS as f64 / pixels as f64).sqrt();
        width = (width as f64 * factor).floor().max(1.0) as usize;
        height = (height as f64 * factor).floor().max(1.0) as usize;
    }
    let width_fit = width as f64 / raster.width.max(1) as f64;
    let fit = if fit_width {
        width_fit
    } else {
        width_fit.min(height as f64 / raster.height.max(1) as f64)
    };
    let scale = fit * f64::from(zoom) / 100.0;
    let target_width = (raster.width as f64 * scale).round().max(1.0) as usize;
    let target_height = (raster.height as f64 * scale).round().max(1.0) as usize;
    ViewportGeometry {
        target_width,
        target_height,
        visible_width: target_width.min(width),
        visible_height: target_height.min(height),
    }
}

pub(super) fn normalized_pan(
    viewer: &yoctui_model::HardwareViewerState,
    raster: &yoctui_model::HardwareRaster,
    area: Rect,
    cells: (u16, u16),
    previous: Option<NativeImageKey>,
) -> (usize, usize) {
    let geometry = viewport_geometry(raster, area, cells, viewer.zoom_percent, viewer.fit_width);
    let mut pan = (
        viewer.pan_x.saturating_mul(usize::from(cells.0)),
        viewer.pan_y.saturating_mul(usize::from(cells.1)),
    );
    if let Some(old) = previous.filter(|old| {
        old.generation == viewer.generation
            && old.page == viewer.page
            && old.fit_width == viewer.fit_width
            && old.zoom_percent != viewer.zoom_percent
            && viewer.zoom_percent != 100
    }) {
        let before = viewport_geometry(
            raster,
            old.area,
            (old.cell_width, old.cell_height),
            old.zoom_percent,
            old.fit_width,
        );
        pan.0 = zoomed_origin(
            old.pan_x.saturating_mul(usize::from(old.cell_width)),
            before.visible_width,
            before.target_width,
            geometry.visible_width,
            geometry.target_width,
        );
        pan.1 = zoomed_origin(
            old.pan_y.saturating_mul(usize::from(old.cell_height)),
            before.visible_height,
            before.target_height,
            geometry.visible_height,
            geometry.target_height,
        );
    }
    (
        pan.0
            .min(geometry.target_width.saturating_sub(geometry.visible_width))
            .div_ceil(usize::from(cells.0.max(1))),
        pan.1
            .min(
                geometry
                    .target_height
                    .saturating_sub(geometry.visible_height),
            )
            .div_ceil(usize::from(cells.1.max(1))),
    )
}

fn zoomed_origin(
    origin: usize,
    visible: usize,
    target: usize,
    new_visible: usize,
    new_target: usize,
) -> usize {
    (((origin as f64 + visible as f64 / 2.0) * new_target as f64 / target.max(1) as f64)
        - new_visible as f64 / 2.0)
        .max(0.0)
        .round() as usize
}
