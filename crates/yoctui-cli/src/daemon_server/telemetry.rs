use super::*;

#[derive(Default)]
pub(super) struct TelemetryState {
    pub(super) recovered: bool,
    pub(super) last_telemetry_ms: u64,
    pub(super) maximum_client_backlog: usize,
    pub(super) forced_client_resynchronizations: u64,
    pub(super) slow_client_disconnects: u64,
}

pub(super) async fn publish(
    services: &mut DaemonServices,
    telemetry: &mut TelemetryState,
    clients: &[ClientConnection],
    record: &DaemonRuntimeRecord,
    archive_recorder: &mut build_archive::Recorder,
) -> Result<()> {
    let now_ms = unix_ms();
    archive_recorder
        .poll(services.daemon_journal.snapshot(), unix_ms())
        .await;
    let active_work = daemon_has_active_work(services.daemon_journal.snapshot());
    let current_client_backlog = clients
        .iter()
        .filter(|(_, _, attached, _, _, _)| *attached)
        .map(|(_, _, _, sequence, _, _)| {
            usize::try_from(
                services
                    .daemon_journal
                    .snapshot()
                    .sequence
                    .saturating_sub(*sequence),
            )
            .unwrap_or(usize::MAX)
            .min(services.daemon_journal.retained_event_capacity())
        })
        .sum::<usize>();
    telemetry.maximum_client_backlog = telemetry.maximum_client_backlog.max(current_client_backlog);
    let attached_clients = clients
        .iter()
        .filter(|(_, _, attached, _, _, _)| *attached)
        .count();
    let telemetry_interval = daemon_telemetry_interval(attached_clients, active_work);
    if telemetry_interval.is_some_and(|interval| {
        now_ms.saturating_sub(telemetry.last_telemetry_ms)
            >= u64::try_from(interval.as_millis()).unwrap_or(u64::MAX)
    }) {
        let snapshot = services.daemon_journal.snapshot();
        let bitbake_pressure = services.bitbake_supervisor.pressure();
        let active_jobs = snapshot
            .jobs
            .iter()
            .filter(|job| {
                matches!(
                    job.lifecycle,
                    yoctui_protocol::daemon::LifecycleState::Connecting
                        | yoctui_protocol::daemon::LifecycleState::Running
                        | yoctui_protocol::daemon::LifecycleState::Stopping
                )
            })
            .count();
        let _ =
            services
                .daemon_journal
                .publish(yoctui_protocol::daemon::DaemonEvent::Telemetry(
                    DaemonTelemetry {
                        uptime_seconds: now_ms.saturating_sub(record.started_unix_ms) / 1_000,
                        bitbake: snapshot.bitbake.lifecycle,
                        connected_clients: attached_clients.min(u16::MAX as usize) as u16,
                        active_jobs: active_jobs.min(u16::MAX as usize) as u16,
                        pty_sessions: snapshot.pty_sessions.len().min(u16::MAX as usize) as u16,
                        queue_depth: bitbake_pressure
                            .current_queue_depth
                            .saturating_add(current_client_backlog)
                            .min(u16::MAX as usize) as u16,
                        pressure: DaemonPressureCounters {
                            current_queue_depth: bitbake_pressure
                                .current_queue_depth
                                .saturating_add(current_client_backlog)
                                .min(u32::MAX as usize)
                                as u32,
                            maximum_queue_depth: bitbake_pressure
                                .maximum_queue_depth
                                .saturating_add(telemetry.maximum_client_backlog)
                                .min(u32::MAX as usize)
                                as u32,
                            cosmetic_coalesced: 0,
                            cosmetic_dropped: bitbake_pressure.cosmetic_dropped,
                            reliable_waits: bitbake_pressure.reliable_waits,
                            forced_resynchronizations: telemetry.forced_client_resynchronizations,
                            slow_client_disconnects: telemetry.slow_client_disconnects,
                        },
                        memory_bytes: process_memory_bytes(),
                        recovery: if telemetry.recovered {
                            DaemonRecoveryState::Recovered
                        } else {
                            DaemonRecoveryState::CleanStart
                        },
                    },
                ))?;
        telemetry.last_telemetry_ms = now_ms;
    }
    Ok(())
}
