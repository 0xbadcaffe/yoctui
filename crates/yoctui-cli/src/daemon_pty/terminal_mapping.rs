use yoctui_model::{PtySessionId, PtySessionKind};
use yoctui_protocol::daemon::PtyKind;

pub(super) fn snapshot_to_wire(
    listing: &crate::pty_attach::PtyAttachListing,
) -> yoctui_protocol::daemon::PtySessionSummary {
    yoctui_protocol::daemon::PtySessionSummary {
        id: yoctui_protocol::daemon::PtySessionId(listing.id.0),
        name: listing.name.clone(),
        kind: match listing.kind {
            PtySessionKind::BuildShell => PtyKind::BuildShell,
            PtySessionKind::SourceShell => PtyKind::SourceShell,
            PtySessionKind::LayerShell => PtyKind::LayerShell,
            PtySessionKind::RecipeShell => PtyKind::RecipeShell,
            PtySessionKind::DevtoolShell => PtyKind::DevtoolShell,
            PtySessionKind::Devshell => PtyKind::Devshell,
            PtySessionKind::Menuconfig => PtyKind::Menuconfig,
            PtySessionKind::SdkShell => PtyKind::SdkShell,
            PtySessionKind::NativeShell => PtyKind::NativeShell,
            PtySessionKind::QemuConsole => PtyKind::QemuConsole,
            PtySessionKind::SshConsole => PtyKind::SshConsole,
            PtySessionKind::InteractiveTool | PtySessionKind::DeployShell => PtyKind::Utility,
        },
        cwd: listing.cwd.display().to_string(),
        lifecycle: match listing.lifecycle {
            crate::pty_attach::PtyAttachLifecycle::Running => {
                yoctui_protocol::daemon::LifecycleState::Running
            }
            crate::pty_attach::PtyAttachLifecycle::Exited => {
                yoctui_protocol::daemon::LifecycleState::Exited
            }
            crate::pty_attach::PtyAttachLifecycle::Lost => {
                yoctui_protocol::daemon::LifecycleState::Lost
            }
        },
        dimensions: yoctui_protocol::daemon::TerminalDimensions {
            columns: listing.dimensions.columns,
            rows: listing.dimensions.rows,
        },
        writer: listing
            .writer
            .map(|client| yoctui_protocol::daemon::ClientId(client.0)),
        writer_epoch: listing.writer_epoch,
        viewers: listing.viewers as u16,
        exit_code: listing.exit_status.and_then(|status| match status {
            yoctui_model::PtyExitStatus::Code(code) => Some(code),
            yoctui_model::PtyExitStatus::Signal(_) => None,
        }),
        restartable: listing.restartable,
    }
}

pub(super) fn terminal_to_wire(
    session_id: PtySessionId,
    terminal: &yoctui_model::TerminalSnapshot,
) -> yoctui_protocol::daemon::PtyScreenSnapshot {
    let cells = terminal
        .cells
        .iter()
        .enumerate()
        .filter(|(_, cell)| terminal_cell_is_non_default(cell))
        .map(|(index, cell)| yoctui_protocol::daemon::PtyScreenCell {
            index: index as u32,
            contents: cell.contents.clone(),
            foreground: terminal_color_to_wire(cell.foreground),
            background: terminal_color_to_wire(cell.background),
            bold: cell.bold,
            dim: cell.dim,
            italic: cell.italic,
            underline: cell.underline,
            inverse: cell.inverse,
            wide: cell.wide,
            wide_continuation: cell.wide_continuation,
        })
        .collect();
    yoctui_protocol::daemon::PtyScreenSnapshot {
        session_id: yoctui_protocol::daemon::PtySessionId(session_id.0),
        dimensions: yoctui_protocol::daemon::TerminalDimensions {
            columns: terminal.dimensions.columns,
            rows: terminal.dimensions.rows,
        },
        cursor_column: terminal.cursor.1,
        cursor_row: terminal.cursor.0,
        cursor_hidden: terminal.modes.cursor_hidden,
        scrollback_offset: terminal.scrollback_offset.min(u32::MAX as usize) as u32,
        cells,
        scrollback_lines: terminal.max_scrollback_offset.min(u32::MAX as usize) as u32,
        dropped_line_feeds_lower_bound: terminal.dropped_line_feeds_lower_bound,
    }
}

fn terminal_cell_is_non_default(cell: &yoctui_model::TerminalCell) -> bool {
    !cell.contents.is_empty()
        || cell.foreground != yoctui_model::TerminalColor::Default
        || cell.background != yoctui_model::TerminalColor::Default
        || cell.bold
        || cell.dim
        || cell.italic
        || cell.underline
        || cell.inverse
        || cell.wide
        || cell.wide_continuation
}

fn terminal_color_to_wire(
    color: yoctui_model::TerminalColor,
) -> yoctui_protocol::daemon::PtyTerminalColor {
    match color {
        yoctui_model::TerminalColor::Default => yoctui_protocol::daemon::PtyTerminalColor::Default,
        yoctui_model::TerminalColor::Indexed(index) => {
            yoctui_protocol::daemon::PtyTerminalColor::Indexed(index)
        }
        yoctui_model::TerminalColor::Rgb(red, green, blue) => {
            yoctui_protocol::daemon::PtyTerminalColor::Rgb(red, green, blue)
        }
    }
}
