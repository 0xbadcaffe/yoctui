//! Geometry-only projection for the CLI-owned native Hardware raster.

use super::*;

#[derive(Debug, Clone, Copy)]
pub struct HardwareNativeRasterProjection<'a> {
    pub area: Rect,
    pub viewer: &'a yoctui_model::HardwareViewerState,
    pub raster: &'a yoctui_model::HardwareRaster,
}

pub fn hardware_native_raster_projection(
    app: &App,
    terminal_width: u16,
    terminal_height: u16,
) -> Option<HardwareNativeRasterProjection<'_>> {
    if terminal_width < 80
        || terminal_height < 24
        || app.screen != Screen::Hardware
        || app.hardware.graphics_capability != yoctui_model::HardwareGraphicsCapability::Sixel
        || app.command_palette_open
        || app.menu.is_open()
        || app.active_dialog().is_some()
        || app.notification.is_some()
    {
        return None;
    }
    let viewer = app.hardware.viewer.as_ref()?;
    if viewer.loading || viewer.presentation != yoctui_model::HardwarePresentation::Page {
        return None;
    }
    let yoctui_model::HardwarePreview::Raster(raster) = viewer.preview.as_ref()? else {
        return None;
    };

    let [header, footer] =
        yoctui_app::workbench_chrome_heights(app, terminal_width, terminal_height);
    let body = Rect::new(
        0,
        header,
        terminal_width,
        terminal_height
            .saturating_sub(header)
            .saturating_sub(footer),
    );
    let workspace = if terminal_width >= 100 {
        let navigator_width = if app.preferences.density == yoctui_model::UiDensity::Compact {
            18
        } else {
            22
        };
        Rect::new(
            navigator_width,
            body.y,
            body.width.saturating_sub(navigator_width),
            body.height,
        )
    } else {
        if app.focus == FocusTarget::Navigator {
            return None;
        }
        Rect::new(
            body.x,
            body.y.saturating_add(1),
            body.width,
            body.height.saturating_sub(1),
        )
    };
    let area = Rect::new(
        workspace.x,
        workspace.y.saturating_add(2),
        workspace.width,
        workspace.height.saturating_sub(4),
    );
    (!area.is_empty()).then_some(HardwareNativeRasterProjection {
        area,
        viewer,
        raster,
    })
}
