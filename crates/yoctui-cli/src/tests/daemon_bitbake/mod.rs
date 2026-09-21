use super::*;
use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};
use tokio::sync::mpsc;
use yoctui_bitbake::BackendEvent;
use yoctui_model::{
    AuthoritativeValue, BuildRequest, CapabilityEvidence, CapabilityEvidenceKind,
    CapabilityEvidenceOutcome, CapabilityId, CapabilityImplementation,
    CapabilityImplementationKind, CapabilityRecord, CapabilitySnapshot, CapabilityState,
    DaemonCompatibilitySnapshot, IdentityAuthority, YoctoEnvironmentIdentity,
};
use yoctui_protocol::daemon::JobId;

use super::ingress::{
    bitbake_event_is_diagnostic, bitbake_event_requires_immediate_wake, send_bitbake_event,
};
use super::notification::ActivityNotification;

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

fn log_event(severity: yoctui_model::Severity, message: &str) -> DaemonBitBakeEvent {
    DaemonBitBakeEvent::Backend {
        job_id: JobId(1),
        event: Box::new(BackendEvent::Log(yoctui_model::LogEntry {
            id: 0,
            severity,
            message: message.into(),
            recipe: None,
            task: None,
            path: None,
            timestamp: std::time::SystemTime::UNIX_EPOCH,
            build: None,
            protected: false,
            diagnostic: None,
        })),
    }
}

fn cancellation_authority(build: &Path) -> DaemonCompatibilitySnapshot {
    let capabilities = [
        CapabilityId::BitBakeWorkspaceInspection,
        CapabilityId::BitBakeRecipeInventory,
        CapabilityId::BitBakeLayerInventory,
        CapabilityId::BitBakeBuild,
        CapabilityId::BitBakeCancellation,
        CapabilityId::BitBakeNativeEvents,
        CapabilityId::BitBakeServerSocket,
    ];
    DaemonCompatibilitySnapshot {
        snapshot: CapabilitySnapshot {
            generation: 1,
            environment: YoctoEnvironmentIdentity {
                build_directory: AuthoritativeValue::detected(
                    build.to_path_buf(),
                    IdentityAuthority::InitializedEnvironment,
                ),
                bitbake_version: AuthoritativeValue::detected(
                    "2.18.0".into(),
                    IdentityAuthority::BitBakeVersionProbe,
                ),
                ..YoctoEnvironmentIdentity::default()
            },
            capabilities: capabilities
                .into_iter()
                .map(|id| CapabilityRecord {
                    id,
                    state: CapabilityState::Available,
                    evidence: vec![CapabilityEvidence {
                        kind: CapabilityEvidenceKind::BackendNegotiation,
                        outcome: CapabilityEvidenceOutcome::Positive,
                        subject: id.as_str().into(),
                        detail: "Fake initialized backend exposes the required operation.".into(),
                        argv: Vec::new(),
                    }],
                })
                .collect(),
        },
        implementations: capabilities
            .into_iter()
            .map(|id| {
                (
                    id,
                    CapabilityImplementation {
                        id: "tinfoil.adapter.modern".into(),
                        kind: CapabilityImplementationKind::BackendApi,
                    },
                )
            })
            .collect(),
    }
    .normalize()
    .unwrap()
}

mod activity_notification_coalesces_and_rearms;
mod batched_activity_notification_is_rate_limited;
mod bitbake_connection_reports_real_bridge_eof_once;
mod bitbake_connection_tolerates_scheduler_delay_without_false_disconnect;
mod bounded_priority_ingress_drops_only_cosmetic_events;
mod daemon_cancellation_times_out_to_one_terminal_event;
mod daemon_compatibility_cancellation_preempts_event_flood_and_terminates_once;
mod daemon_compatibility_runtime_bitbake_rejects_missing_authority_before_spawn;
mod daemon_job_identity_cancellation_reaches_only_the_exact_bridge_owner;
mod only_correctness_boundaries_require_immediate_wake;
