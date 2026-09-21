use std::sync::atomic::Ordering;

use tokio::sync::mpsc;
use yoctui_bitbake::BackendEvent;

use super::{
    DaemonBitBakeEvent, DaemonBitBakePressureShared, notification::ActivityNotificationSender,
};

pub(super) async fn send_bitbake_event(
    reliable: &mpsc::Sender<DaemonBitBakeEvent>,
    cosmetic: &mpsc::Sender<DaemonBitBakeEvent>,
    pressure: &DaemonBitBakePressureShared,
    activity: Option<&ActivityNotificationSender>,
    event: DaemonBitBakeEvent,
) {
    if bitbake_event_is_cosmetic(&event) {
        match cosmetic.try_send(event) {
            Ok(()) => {
                pressure.cosmetic_enqueued.fetch_add(1, Ordering::Relaxed);
                record_queue_depth(reliable, cosmetic, pressure);
                if let Some(activity) = activity {
                    activity.signal_batched();
                }
            }
            Err(mpsc::error::TrySendError::Full(_)) => {
                pressure.cosmetic_dropped.fetch_add(1, Ordering::Relaxed);
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {}
        }
    } else {
        if reliable.capacity() == 0 {
            pressure.reliable_waits.fetch_add(1, Ordering::Relaxed);
        }
        let immediate = bitbake_event_requires_immediate_wake(&event);
        if reliable.send(event).await.is_ok() {
            pressure.reliable_enqueued.fetch_add(1, Ordering::Relaxed);
            record_queue_depth(reliable, cosmetic, pressure);
            if let Some(activity) = activity {
                if immediate {
                    activity.signal();
                } else {
                    activity.signal_batched();
                }
            }
        }
    }
}

fn record_queue_depth(
    reliable: &mpsc::Sender<DaemonBitBakeEvent>,
    cosmetic: &mpsc::Sender<DaemonBitBakeEvent>,
    pressure: &DaemonBitBakePressureShared,
) {
    let depth = (reliable.max_capacity() - reliable.capacity())
        .saturating_add(cosmetic.max_capacity() - cosmetic.capacity());
    pressure
        .maximum_queue_depth
        .fetch_max(depth, Ordering::Relaxed);
}

fn bitbake_event_is_cosmetic(event: &DaemonBitBakeEvent) -> bool {
    matches!(
        event,
        DaemonBitBakeEvent::Backend { event, .. }
            if matches!(
                event.as_ref(),
                BackendEvent::ParseProgress { .. }
                    | BackendEvent::TaskProgress { .. }
                    | BackendEvent::Log(yoctui_model::LogEntry {
                        severity: yoctui_model::Severity::Trace | yoctui_model::Severity::Info,
                        ..
                    })
                    | BackendEvent::Ignored
            )
    )
}

pub(super) fn bitbake_event_requires_immediate_wake(event: &DaemonBitBakeEvent) -> bool {
    matches!(event, DaemonBitBakeEvent::Failed { .. })
        || matches!(
            event,
            DaemonBitBakeEvent::Backend { event, .. }
                if matches!(
                    event.as_ref(),
                    BackendEvent::DependencyGraphFailed { .. }
                        | BackendEvent::SignatureDumpFailed { .. }
                        | BackendEvent::SignatureComparisonFailed { .. }
                        | BackendEvent::PackageInventoryFailed { .. }
                        | BackendEvent::PackageDetailFailed { .. }
                        | BackendEvent::ImageArtifactsFailed { .. }
                        | BackendEvent::RootfsCompositionFailed { .. }
                        | BackendEvent::TaskCompleted { success: false, .. }
                        | BackendEvent::BuildCompleted { .. }
                        | BackendEvent::CommandFailed { .. }
                        | BackendEvent::Disconnected
                )
        )
}

pub(super) fn bitbake_event_is_diagnostic(event: &DaemonBitBakeEvent) -> bool {
    matches!(event, DaemonBitBakeEvent::Failed { .. })
        || matches!(
            event,
            DaemonBitBakeEvent::Backend { event, .. }
                if matches!(
                    event.as_ref(),
                    BackendEvent::CommandFailed { .. }
                        | BackendEvent::Disconnected
                        | BackendEvent::Log(yoctui_model::LogEntry {
                            severity: yoctui_model::Severity::Warning
                                | yoctui_model::Severity::Error,
                            ..
                        })
                )
        )
}
