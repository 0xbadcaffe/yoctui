use super::*;
use yoctui_model::{CapabilityId, DaemonCompatibilitySnapshot};

pub(super) fn poll(services: &mut DaemonServices) -> Result<()> {
    if let Some(result) = services.startup_compatibility.try_result() {
        match result {
            Ok(Some(compatibility)) => install(services, compatibility)?,
            Ok(None) => {}
            Err(error) => {
                tracing::warn!(%error, "daemon compatibility startup probe failed");
                publish_startup_metadata_log(
                    &mut services.daemon_journal,
                    &format!("Compatibility discovery failed: {error:#}"),
                    true,
                )?;
                yoctui_app::reduce_daemon_state(
                    &mut services.daemon_state,
                    yoctui_model::DaemonStateAction::RecordError(format!(
                        "Compatibility discovery failed: {error:#}"
                    )),
                )?;
            }
        }
    }
    if let Some(result) = services.backend_recovery.poll(
        services.daemon_state.compatibility.as_ref(),
        &services.startup_environment,
    ) {
        match result {
            Ok(Some(recovered)) => {
                if services
                    .daemon_state
                    .compatibility
                    .as_ref()
                    .is_some_and(|current| {
                        daemon_compatibility::backend_recovery::is_current_recovery(
                            current, &recovered,
                        )
                    })
                {
                    install(services, recovered)?;
                }
            }
            Ok(None) => {}
            Err(error) => publish_startup_metadata_log(
                &mut services.daemon_journal,
                &format!("BitBake API discovery failed: {error:#}. Retrying in 30 seconds."),
                true,
            )?,
        }
    }
    Ok(())
}

fn install(
    services: &mut DaemonServices,
    compatibility: DaemonCompatibilitySnapshot,
) -> Result<()> {
    services
        .devtool_supervisor
        .replace_compatibility(Some(compatibility.clone()))?;
    services
        .raw_supervisor
        .replace_compatibility(Some(compatibility.clone()))?;
    services
        .bitbake_supervisor
        .replace_compatibility(Some(compatibility.clone()))
        .map_err(anyhow::Error::msg)?;
    yoctui_app::reduce_daemon_state(
        &mut services.daemon_state,
        yoctui_model::DaemonStateAction::ReplaceCompatibility(Box::new(compatibility.clone())),
    )?;
    let wire = daemon_protocol_snapshot(&services.daemon_state)
        .compatibility
        .expect("installed compatibility has wire authority");
    services
        .daemon_journal
        .publish(yoctui_protocol::daemon::DaemonEvent::CompatibilityChanged(
            Box::new(wire),
        ))?;
    if !compatibility
        .snapshot
        .allows(CapabilityId::BitBakeWorkspaceInspection)
    {
        if daemon_compatibility::backend_recovery::needed(&compatibility) {
            publish_startup_metadata_log(
                &mut services.daemon_journal,
                "Waiting for BitBake API discovery; retrying the backend probe",
                false,
            )?;
        }
        return Ok(());
    }
    if services
        .startup_metadata
        .as_ref()
        .is_some_and(|scan| scan.pending())
    {
        return Ok(());
    }
    publish_startup_metadata_log(
        &mut services.daemon_journal,
        "Loading initial workspace and recipe inventory",
        false,
    )?;
    let environment = services.startup_environment.clone();
    services.startup_metadata = Some(daemon_metadata::StartupMetadata::spawn(
        |cancelled| async move {
            inspect_daemon_startup_workspace(&environment, Some(compatibility), cancelled).await
        },
    ));
    Ok(())
}
