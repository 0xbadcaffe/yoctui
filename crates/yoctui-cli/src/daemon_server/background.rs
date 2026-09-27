use super::*;

pub(super) fn poll(services: &mut DaemonServices) -> Result<()> {
    super::compatibility::poll(services)?;
    if let Some(result) = services
        .startup_metadata
        .as_mut()
        .and_then(|scan| scan.try_result())
    {
        match result {
            Ok(Some(workspace)) => {
                if daemon_metadata::publish_workspace(
                    &mut services.daemon_journal,
                    workspace.clone(),
                )? {
                    yoctui_app::reduce_daemon_state(
                        &mut services.daemon_state,
                        yoctui_model::DaemonStateAction::ReplaceWorkspace(workspace),
                    )?;
                }
            }
            Ok(None) if services.startup_configured => publish_startup_metadata_log(
                &mut services.daemon_journal,
                "Initial metadata unavailable: no configured build environment",
                false,
            )?,
            Ok(None) => {}
            Err(error) => publish_startup_metadata_log(
                &mut services.daemon_journal,
                &format!("Initial metadata scan failed: {error:#}"),
                true,
            )?,
        }
    }

    for _ in 0..MAX_SUPERVISOR_EVENTS_PER_TICK {
        let Some(event) = services.devtool_supervisor.try_event() else {
            break;
        };
        publish_daemon_devtool_event(&mut services.daemon_journal, event)?;
    }
    for _ in 0..MAX_SUPERVISOR_EVENTS_PER_TICK {
        let Some(event) = services.raw_supervisor.try_event() else {
            break;
        };
        publish_daemon_raw_event(&mut services.daemon_journal, event)?;
    }
    for _ in 0..MAX_SUPERVISOR_EVENTS_PER_TICK {
        let Some(event) = services.bitbake_supervisor.try_event() else {
            break;
        };
        publish_daemon_bitbake_event(&mut services.daemon_journal, event)?;
    }
    for _ in 0..MAX_SUPERVISOR_EVENTS_PER_TICK {
        let Some(event) = services.sdk_supervisor.try_event() else {
            break;
        };
        publish_daemon_sdk_event(&mut services.daemon_journal, event)?;
    }
    for _ in 0..MAX_SUPERVISOR_EVENTS_PER_TICK {
        let Some(event) = services.qemu_supervisor.try_event() else {
            break;
        };
        publish_daemon_qemu_event(&mut services.daemon_journal, event)?;
    }
    for _ in 0..MAX_SUPERVISOR_EVENTS_PER_TICK {
        let Some(event) = services.wic_supervisor.try_event() else {
            break;
        };
        publish_daemon_wic_event(&mut services.daemon_journal, event)?;
    }
    for _ in 0..MAX_SUPERVISOR_EVENTS_PER_TICK {
        let Some(event) = services.test_supervisor.try_event() else {
            break;
        };
        publish_daemon_test_event(&mut services.daemon_journal, event)?;
    }
    for _ in 0..MAX_SUPERVISOR_EVENTS_PER_TICK {
        let Some(event) = services.qa_supervisor.try_event() else {
            break;
        };
        publish_daemon_qa_event(&mut services.daemon_journal, event)?;
    }
    for _ in 0..MAX_SUPERVISOR_EVENTS_PER_TICK {
        let Some(event) = services.qa_report_supervisor.try_event() else {
            break;
        };
        publish_daemon_qa_report_event(&mut services.daemon_journal, event)?;
    }
    for _ in 0..MAX_SUPERVISOR_EVENTS_PER_TICK {
        let Some(event) = services.security_supervisor.try_event() else {
            break;
        };
        publish_daemon_security_event(&mut services.daemon_journal, event)?;
    }
    for _ in 0..MAX_SUPERVISOR_EVENTS_PER_TICK {
        let Some(event) = services.security_mapper_supervisor.try_event() else {
            break;
        };
        publish_daemon_security_mapper_event(&mut services.daemon_journal, event)?;
    }
    for _ in 0..MAX_SUPERVISOR_EVENTS_PER_TICK {
        let Some(event) = services.maintenance_supervisor.try_event() else {
            break;
        };
        publish_daemon_maintenance_event(&mut services.daemon_journal, event)?;
    }
    for _ in 0..MAX_SUPERVISOR_EVENTS_PER_TICK {
        let Some(event) = services.pty_supervisor.try_event() else {
            break;
        };
        let raw_state = match &event {
            daemon_pty::DaemonPtyEvent::Started { session_id, .. } => {
                services.raw_supervisor.pty_started(*session_id)?
            }
            daemon_pty::DaemonPtyEvent::Changed {
                session_id,
                snapshot,
            } => services
                .raw_supervisor
                .pty_attachment(*session_id, snapshot.viewers > 0)?,
            daemon_pty::DaemonPtyEvent::Exited {
                session_id,
                exit_code,
                ..
            } => services
                .raw_supervisor
                .pty_finished(*session_id, *exit_code, None)?,
            daemon_pty::DaemonPtyEvent::Lost {
                session_id,
                message,
            } => services
                .raw_supervisor
                .pty_finished(*session_id, None, Some(message.clone()))?,
            daemon_pty::DaemonPtyEvent::Output { .. } | daemon_pty::DaemonPtyEvent::Screen(..) => {
                None
            }
        };
        if let Some(state) = raw_state {
            services.daemon_journal.publish(
                yoctui_protocol::daemon::DaemonEvent::RawExecutionChanged(Box::new(
                    yoctui_app::raw_execution_snapshot_to_protocol(&state)
                        .map_err(anyhow::Error::msg)?,
                )),
            )?;
        }
        publish_daemon_pty_event(&mut services.daemon_journal, event)?;
    }

    Ok(())
}
