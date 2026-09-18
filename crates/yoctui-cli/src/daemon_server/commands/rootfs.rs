use super::*;

pub(super) fn handle(
    request: CommandRequest,
    services: &mut DaemonServices,
    client: &mut DaemonClient,
) -> Result<Option<CommandOutcome>> {
    Ok(Some(match request.command {
        DaemonCommand::InspectRootfsSources { query } => {
            let result = (|| -> Result<daemon_rootfs::PendingQuery> {
                anyhow::ensure!(
                    client.attached && request.expected_generation.is_some(),
                    "rootfs queries require current attached authority"
                );
                anyhow::ensure!(
                    client.rootfs_query.is_none(),
                    "this client already has a rootfs query"
                );
                anyhow::ensure!(
                    !services.startup_compatibility.pending()
                        && !services
                            .startup_metadata
                            .as_ref()
                            .is_some_and(|scan| scan.pending())
                        && !services
                            .daemon_journal
                            .snapshot()
                            .jobs
                            .iter()
                            .any(
                                |job| job.kind == yoctui_protocol::daemon::JobKind::BitBakeBuild
                                    && matches!(
                                        job.lifecycle,
                                        yoctui_protocol::daemon::LifecycleState::Connecting
                                            | yoctui_protocol::daemon::LifecycleState::Running
                                            | yoctui_protocol::daemon::LifecycleState::Stopping
                                    )
                            ),
                    "rootfs metadata is busy; retry after the active build finishes"
                );
                let compatibility = services
                    .daemon_state
                    .compatibility
                    .clone()
                    .context("rootfs query requires compatibility authority")?;
                daemon_rootfs::PendingQuery::start(
                    request.request_id,
                    query,
                    services.instance,
                    compatibility,
                    services.rootfs_environment.clone(),
                    services.rootfs_query_permit.clone(),
                )
            })();
            match result {
                Ok(pending) => {
                    client.rootfs_query = Some(pending);
                    return Ok(None);
                }
                Err(error) => CommandOutcome::Rejected {
                    code: yoctui_protocol::daemon::ProtocolErrorCode::Conflict,
                    message: format!("rootfs metadata unavailable: {error:#}"),
                    current_generation: services.daemon_journal.snapshot().generation,
                },
            }
        }
        _ => unreachable!("command family was checked by dispatch"),
    }))
}
