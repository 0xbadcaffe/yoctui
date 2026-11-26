//! Shared filesystem pane geometry for drawing and pointer input.
use yoctui_model::LayerBrowser;

#[derive(Debug, Clone, Copy)]
pub struct LayerBrowserLayout {
    pub summary: [u16; 4],
    pub configured: [u16; 4],
    pub search: [u16; 4],
    pub breadcrumb: [u16; 4],
    pub tree: [u16; 4],
    pub details: [u16; 4],
}

pub fn layer_browser_layout(
    browser: &LayerBrowser,
    layer_count: usize,
    [x, y, width, height]: [u16; 4],
) -> LayerBrowserLayout {
    let standalone = browser.is_rootfs() || browser.is_build_directory();
    let summary = u16::from(browser.is_build_directory()).min(height);
    let body_y = y + summary;
    let body_height = height - summary;
    let (left_width, left_height, details) = if standalone && width < 120 {
        let tree_height = (u32::from(body_height) * 55).div_ceil(100) as u16;
        (
            width,
            tree_height,
            [x, body_y + tree_height, width, body_height - tree_height],
        )
    } else {
        let left = layer_browser_left_width(browser, width);
        (
            left,
            body_height,
            [x + left, body_y, width - left, body_height],
        )
    };
    let configured = if standalone {
        0
    } else {
        layer_count
            .saturating_add(2)
            .min(8)
            .min(usize::from(left_height.saturating_sub(4))) as u16
    };
    let list_y = body_y + configured;
    let list_height = left_height - configured;
    let search = list_height.min(1);
    let breadcrumb = list_height.saturating_sub(search).saturating_sub(2).min(1);
    LayerBrowserLayout {
        summary: [x, y, width, summary],
        configured: [x, body_y, left_width, configured],
        search: [x, list_y, left_width, search],
        breadcrumb: [x, list_y + search, left_width, breadcrumb],
        tree: [
            x,
            list_y + search + breadcrumb,
            left_width,
            list_height - search - breadcrumb,
        ],
        details,
    }
}

pub fn layer_browser_left_width(browser: &LayerBrowser, total_width: u16) -> u16 {
    if browser.is_rootfs() {
        return total_width
            .saturating_sub(38)
            .min(total_width.saturating_mul(65) / 100);
    }
    let configured = browser.layer.chars().count().saturating_add(18);
    let tree = browser
        .entries
        .iter()
        .map(|entry| {
            entry
                .path
                .file_name()
                .map_or(0, |name| name.to_string_lossy().chars().count())
                .saturating_add(entry.depth.saturating_mul(2))
                .saturating_add(8)
        })
        .max()
        .unwrap_or(0);
    let useful = configured.max(tree).clamp(38, 54) as u16;
    let ratio_cap = total_width.saturating_mul(42) / 100;
    let preview_floor = if total_width >= 100 { 58 } else { 32 };
    let preview_cap = total_width.saturating_sub(preview_floor);
    useful.min(ratio_cap.max(1)).min(preview_cap.max(1))
}
