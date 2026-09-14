//! Daemon view.

pub(crate) fn daemon_client_view(
    status: yoctui_model::ClientReplicaStatus,
    snapshot: Option<&yoctui_protocol::daemon::DaemonSnapshot>,
    telemetry: Option<yoctui_protocol::daemon::DaemonTelemetry>,
) -> yoctui_model::ClientDaemonView {
    use yoctui_model::{
        ClientDaemonJobSummary, ClientDaemonPtyDetails, ClientDaemonPtyKind, ClientDaemonPtyScreen,
        ClientDaemonPtySummary, ClientDaemonTerminalCell, ClientDaemonView,
    };
    let Some(snapshot) = snapshot else {
        return ClientDaemonView {
            status,
            ..ClientDaemonView::default()
        };
    };
    ClientDaemonView {
        status,
        instance_id: Some(yoctui_model::DaemonModelInstanceId(
            snapshot.daemon_instance_id.0,
        )),
        instance_identity: Some(
            snapshot
                .daemon_instance_id
                .0
                .iter()
                .take(4)
                .map(|byte| format!("{byte:02x}"))
                .collect(),
        ),
        sequence: snapshot.sequence,
        generation: snapshot.generation,
        bitbake: client_daemon_lifecycle(snapshot.bitbake.lifecycle),
        jobs: snapshot
            .jobs
            .iter()
            .map(|job| ClientDaemonJobSummary {
                id: job.id.0,
                kind: match job.kind {
                    yoctui_protocol::daemon::JobKind::BitBakeBuild => {
                        yoctui_model::ClientDaemonJobKind::BitBakeBuild
                    }
                    yoctui_protocol::daemon::JobKind::Devtool => {
                        yoctui_model::ClientDaemonJobKind::Devtool
                    }
                    yoctui_protocol::daemon::JobKind::Qemu => {
                        yoctui_model::ClientDaemonJobKind::Qemu
                    }
                    yoctui_protocol::daemon::JobKind::Wic => yoctui_model::ClientDaemonJobKind::Wic,
                    yoctui_protocol::daemon::JobKind::Sdk => yoctui_model::ClientDaemonJobKind::Sdk,
                    yoctui_protocol::daemon::JobKind::Testing => {
                        yoctui_model::ClientDaemonJobKind::Testing
                    }
                    yoctui_protocol::daemon::JobKind::Qa => yoctui_model::ClientDaemonJobKind::Qa,
                    yoctui_protocol::daemon::JobKind::Security => {
                        yoctui_model::ClientDaemonJobKind::Security
                    }
                    yoctui_protocol::daemon::JobKind::Maintenance => {
                        yoctui_model::ClientDaemonJobKind::Maintenance
                    }
                    yoctui_protocol::daemon::JobKind::Utility => {
                        yoctui_model::ClientDaemonJobKind::Utility
                    }
                    yoctui_protocol::daemon::JobKind::Raw => yoctui_model::ClientDaemonJobKind::Raw,
                    yoctui_protocol::daemon::JobKind::Unknown => {
                        yoctui_model::ClientDaemonJobKind::Unknown
                    }
                },
                label: job.label.clone(),
                lifecycle: client_daemon_lifecycle(job.lifecycle),
                progress_current: job.progress_current,
                progress_total: job.progress_total,
                exit_code: job.exit_code,
            })
            .collect(),
        pty_sessions: snapshot
            .pty_sessions
            .iter()
            .map(|pty| ClientDaemonPtySummary {
                id: pty.id.0,
                name: pty.name.clone(),
                lifecycle: client_daemon_lifecycle(pty.lifecycle),
                viewers: pty.viewers,
            })
            .collect(),
        pty_details: snapshot
            .pty_sessions
            .iter()
            .map(|pty| ClientDaemonPtyDetails {
                id: pty.id.0,
                kind: match pty.kind {
                    yoctui_protocol::daemon::PtyKind::BuildShell => ClientDaemonPtyKind::BuildShell,
                    yoctui_protocol::daemon::PtyKind::SourceShell => {
                        ClientDaemonPtyKind::SourceShell
                    }
                    yoctui_protocol::daemon::PtyKind::LayerShell => ClientDaemonPtyKind::LayerShell,
                    yoctui_protocol::daemon::PtyKind::RecipeShell => {
                        ClientDaemonPtyKind::RecipeShell
                    }
                    yoctui_protocol::daemon::PtyKind::DevtoolShell => {
                        ClientDaemonPtyKind::DevtoolShell
                    }
                    yoctui_protocol::daemon::PtyKind::Devshell => ClientDaemonPtyKind::Devshell,
                    yoctui_protocol::daemon::PtyKind::Menuconfig => ClientDaemonPtyKind::Menuconfig,
                    yoctui_protocol::daemon::PtyKind::SdkShell => ClientDaemonPtyKind::SdkShell,
                    yoctui_protocol::daemon::PtyKind::NativeShell => {
                        ClientDaemonPtyKind::NativeShell
                    }
                    yoctui_protocol::daemon::PtyKind::QemuConsole => {
                        ClientDaemonPtyKind::QemuConsole
                    }
                    yoctui_protocol::daemon::PtyKind::SshConsole => ClientDaemonPtyKind::SshConsole,
                    yoctui_protocol::daemon::PtyKind::Utility => ClientDaemonPtyKind::Utility,
                },
                cwd: pty.cwd.clone(),
                columns: pty.dimensions.columns,
                rows: pty.dimensions.rows,
                writer: pty.writer.map(|writer| writer.0),
                writer_epoch: pty.writer_epoch,
                exit_code: pty.exit_code,
                restartable: pty.restartable,
            })
            .collect(),
        pty_screens: snapshot
            .pty_screens
            .iter()
            .map(|screen| {
                let capacity =
                    usize::from(screen.dimensions.columns) * usize::from(screen.dimensions.rows);
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
                    scrollback_offset: screen.scrollback_offset,
                    rows,
                    cells,
                    scrollback_lines: screen.scrollback_lines,
                    dropped_line_feeds_lower_bound: screen.dropped_line_feeds_lower_bound,
                }
            })
            .collect(),
        connected_clients: snapshot.clients.len(),
        recent_logs: snapshot
            .recent_logs
            .iter()
            .map(|record| record.message.clone())
            .collect(),
        recovery_warnings: snapshot.recovery_warnings.clone(),
        telemetry: telemetry.map(|telemetry| yoctui_model::ClientDaemonTelemetry {
            uptime_seconds: telemetry.uptime_seconds,
            active_jobs: telemetry.active_jobs as usize,
            pty_sessions: telemetry.pty_sessions as usize,
            queue_depth: telemetry.queue_depth as usize,
            pressure: yoctui_model::ClientDaemonPressureCounters {
                current_queue_depth: telemetry.pressure.current_queue_depth as usize,
                maximum_queue_depth: telemetry.pressure.maximum_queue_depth as usize,
                cosmetic_coalesced: telemetry.pressure.cosmetic_coalesced,
                cosmetic_dropped: telemetry.pressure.cosmetic_dropped,
                reliable_waits: telemetry.pressure.reliable_waits,
                forced_resynchronizations: telemetry.pressure.forced_resynchronizations,
                slow_client_disconnects: telemetry.pressure.slow_client_disconnects,
            },
            memory_bytes: telemetry.memory_bytes,
            recovery: match telemetry.recovery {
                yoctui_protocol::daemon::DaemonRecoveryState::CleanStart => {
                    yoctui_model::DaemonRecoveryState::CleanStart
                }
                yoctui_protocol::daemon::DaemonRecoveryState::Recovering => {
                    yoctui_model::DaemonRecoveryState::Recovering
                }
                yoctui_protocol::daemon::DaemonRecoveryState::Recovered => {
                    yoctui_model::DaemonRecoveryState::Recovered
                }
                yoctui_protocol::daemon::DaemonRecoveryState::Degraded => {
                    yoctui_model::DaemonRecoveryState::Degraded
                }
            },
        }),
    }
}

pub(crate) fn client_daemon_terminal_color(
    color: yoctui_protocol::daemon::PtyTerminalColor,
) -> yoctui_model::ClientDaemonTerminalColor {
    match color {
        yoctui_protocol::daemon::PtyTerminalColor::Default => {
            yoctui_model::ClientDaemonTerminalColor::Default
        }
        yoctui_protocol::daemon::PtyTerminalColor::Indexed(index) => {
            yoctui_model::ClientDaemonTerminalColor::Indexed(index)
        }
        yoctui_protocol::daemon::PtyTerminalColor::Rgb(red, green, blue) => {
            yoctui_model::ClientDaemonTerminalColor::Rgb(red, green, blue)
        }
    }
}

pub(crate) fn client_daemon_lifecycle(
    lifecycle: yoctui_protocol::daemon::LifecycleState,
) -> yoctui_model::ClientDaemonLifecycle {
    use yoctui_model::ClientDaemonLifecycle;
    match lifecycle {
        yoctui_protocol::daemon::LifecycleState::Disconnected => {
            ClientDaemonLifecycle::Disconnected
        }
        yoctui_protocol::daemon::LifecycleState::Connecting => ClientDaemonLifecycle::Connecting,
        yoctui_protocol::daemon::LifecycleState::Running => ClientDaemonLifecycle::Running,
        yoctui_protocol::daemon::LifecycleState::Stopping => ClientDaemonLifecycle::Stopping,
        yoctui_protocol::daemon::LifecycleState::Exited => ClientDaemonLifecycle::Exited,
        yoctui_protocol::daemon::LifecycleState::Failed => ClientDaemonLifecycle::Failed,
        yoctui_protocol::daemon::LifecycleState::Lost => ClientDaemonLifecycle::Lost,
    }
}

#[derive(Debug)]
pub enum DaemonClientSyncError {
    MissingSnapshot,
    Protocol(yoctui_protocol::daemon::DaemonSnapshotError),
    NonLogEventInLogBatch,
    NonTaskEventInTaskBatch,
}

impl std::fmt::Display for DaemonClientSyncError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingSnapshot => formatter.write_str("daemon event arrived before a snapshot"),
            Self::Protocol(error) => error.fmt(formatter),
            Self::NonLogEventInLogBatch => {
                formatter.write_str("non-log daemon event entered a log-only batch")
            }
            Self::NonTaskEventInTaskBatch => {
                formatter.write_str("non-task daemon event entered a task-only batch")
            }
        }
    }
}

impl std::error::Error for DaemonClientSyncError {}
