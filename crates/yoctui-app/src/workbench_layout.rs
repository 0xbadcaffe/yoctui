//! Shared development-pane geometry for painting, hit testing and PTY resizing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DevelopmentWorkbenchLayout {
    pub header: [u16; 4],
    pub explorer: [u16; 4],
    pub tabs: [u16; 4],
    pub editor: [u16; 4],
    pub output: [u16; 4],
    pub status: [u16; 4],
}
pub fn development_workbench_layout(
    x: u16,
    y: u16,
    width: u16,
    height: u16,
    panel_zoom: bool,
) -> DevelopmentWorkbenchLayout {
    let header = height.min(2);
    let status = height.saturating_sub(header).min(1);
    let body = height.saturating_sub(header + status);
    let tree_width = (u32::from(width) * 28 / 100) as u16;
    let editor_width = width.saturating_sub(tree_width);
    let out = if panel_zoom {
        body
    } else {
        (if body >= 18 { 9 } else { 5 }).min(body.saturating_sub(4))
    };
    let tabs = body.saturating_sub(out).min(1);
    let editor = body.saturating_sub(tabs + out);
    DevelopmentWorkbenchLayout {
        header: [x, y, width, header],
        explorer: [x, y + header, tree_width, body],
        tabs: [x + tree_width, y + header, editor_width, tabs],
        editor: [x + tree_width, y + header + tabs, editor_width, editor],
        output: [
            x + tree_width,
            y + header + tabs + editor,
            editor_width,
            out,
        ],
        status: [x, y + header + body, width, status],
    }
}
