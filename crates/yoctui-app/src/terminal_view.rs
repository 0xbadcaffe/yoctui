//! Shared typed terminal projection for initial snapshots and incremental frames.
use crate::daemon_view::client_daemon_terminal_color;
use yoctui_model::{
    ClientDaemonPtyDetails, ClientDaemonPtyKind, ClientDaemonPtyScreen, ClientDaemonTerminalCell,
};
use yoctui_protocol::daemon::{PtyKind, PtyScreenSnapshot, PtySessionSummary};

pub(crate) fn terminal_details(pty: &PtySessionSummary) -> ClientDaemonPtyDetails {
    ClientDaemonPtyDetails {
        id: pty.id.0,
        kind: match pty.kind {
            PtyKind::BuildShell => ClientDaemonPtyKind::BuildShell,
            PtyKind::SourceShell => ClientDaemonPtyKind::SourceShell,
            PtyKind::LayerShell => ClientDaemonPtyKind::LayerShell,
            PtyKind::RecipeShell => ClientDaemonPtyKind::RecipeShell,
            PtyKind::DevtoolShell => ClientDaemonPtyKind::DevtoolShell,
            PtyKind::Devshell => ClientDaemonPtyKind::Devshell,
            PtyKind::Menuconfig => ClientDaemonPtyKind::Menuconfig,
            PtyKind::SdkShell => ClientDaemonPtyKind::SdkShell,
            PtyKind::NativeShell => ClientDaemonPtyKind::NativeShell,
            PtyKind::QemuConsole => ClientDaemonPtyKind::QemuConsole,
            PtyKind::SshConsole => ClientDaemonPtyKind::SshConsole,
            PtyKind::Utility => ClientDaemonPtyKind::Utility,
        },
        cwd: pty.cwd.clone(),
        columns: pty.dimensions.columns,
        rows: pty.dimensions.rows,
        writer: pty.writer.map(|writer| writer.0),
        writer_epoch: pty.writer_epoch,
        exit_code: pty.exit_code,
        restartable: pty.restartable,
    }
}

pub(crate) fn terminal_screen(screen: &PtyScreenSnapshot) -> ClientDaemonPtyScreen {
    let capacity = usize::from(screen.dimensions.columns) * usize::from(screen.dimensions.rows);
    let mut cells = vec![ClientDaemonTerminalCell::default(); capacity];
    for cell in &screen.cells {
        if let Some(destination) = cells.get_mut(cell.index as usize) {
            *destination = ClientDaemonTerminalCell {
                contents: cell.contents.clone(),
                foreground: client_daemon_terminal_color(cell.foreground),
                background: client_daemon_terminal_color(cell.background),
                bold: cell.bold,
                dim: cell.dim,
                italic: cell.italic,
                underline: cell.underline,
                inverse: cell.inverse,
                wide: cell.wide,
                wide_continuation: cell.wide_continuation,
            };
        }
    }
    let rows = cells
        .chunks(usize::from(screen.dimensions.columns))
        .map(|cells| {
            cells
                .iter()
                .filter(|cell| !cell.wide_continuation)
                .map(|cell| cell.contents.as_str())
                .collect::<String>()
                .trim_end()
                .to_owned()
        })
        .collect();
    ClientDaemonPtyScreen {
        session_id: screen.session_id.0,
        columns: screen.dimensions.columns,
        rows_count: screen.dimensions.rows,
        cursor_column: screen.cursor_column,
        cursor_row: screen.cursor_row,
        cursor_hidden: screen.cursor_hidden,
        application_cursor: screen.application_cursor,
        scrollback_offset: screen.scrollback_offset,
        rows,
        cells,
        scrollback_lines: screen.scrollback_lines,
        dropped_line_feeds_lower_bound: screen.dropped_line_feeds_lower_bound,
    }
}
