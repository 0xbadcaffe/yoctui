//! Shared development-pane geometry for painting, hit testing and PTY resizing.
#[derive(Debug, Clone)]
pub struct WorkbenchExplorerViewport {
    pub folders: std::ops::Range<usize>,
    pub files: yoctui_model::BoundedScroll,
}

pub fn workbench_explorer_viewport(
    session: &yoctui_model::WorkbenchSession,
    height: u16,
) -> Option<WorkbenchExplorerViewport> {
    let folder = session.folders.get(session.folder)?;
    let inner = usize::from(height.saturating_sub(2));
    let details = 1 + usize::from(folder.error.is_some());
    let folder_rows = inner.saturating_sub(details + 3).max(1).min(inner);
    let folders =
        yoctui_model::file_browser_scroll(session.folder, folder_rows, session.folders.len())
            .visible_range();
    let file_rows = inner.saturating_sub(folders.len() + details);
    let files = yoctui_model::file_browser_scroll(
        folder.browser.selection,
        file_rows,
        folder.browser.entries.len(),
    );
    Some(WorkbenchExplorerViewport { folders, files })
}

pub fn workbench_editor_rect(app: &yoctui_model::App, width: u16, height: u16) -> Option<[u16; 4]> {
    let shell = crate::mouse::workbench_shell(app, width, height)?;
    let region = crate::mouse::workbench_mouse_region(
        crate::MouseInput {
            kind: crate::MouseKind::Down,
            column: width.saturating_sub(1),
            row: shell.y + 2,
        },
        app,
        shell,
    )?;
    if region.target != yoctui_model::FocusTarget::Workspace {
        return None;
    }
    let mut area = region.area;
    let offline = u16::from(app.offline_notice().is_some()) * 2;
    area.y += offline;
    area.height = area.height.saturating_sub(offline);
    Some(
        development_workbench_layout(
            area.x,
            area.y,
            area.width,
            area.height,
            app.workbench.current()?.panel_zoom,
        )
        .editor,
    )
}
/// Centered folder chooser; list rows start four cells below its top border.
pub fn workbench_folder_dialog(x: u16, y: u16, width: u16, height: u16) -> [u16; 4] {
    let popup_width = width.saturating_sub(4).min(90);
    let popup_height = height.saturating_sub(2).min(20);
    [
        x + width.saturating_sub(popup_width) / 2,
        y + height.saturating_sub(popup_height) / 2,
        popup_width,
        popup_height,
    ]
}
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
