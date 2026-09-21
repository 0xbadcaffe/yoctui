pub(crate) fn compatibility_detected_model<T, U>(
    wire: &yoctui_protocol::daemon::CompatibilityDetected<T>,
    map: impl FnOnce(&T) -> Result<U, String>,
) -> Result<yoctui_model::AuthoritativeValue<U>, String> {
    use yoctui_protocol::daemon::{CompatibilityDetected, CompatibilityIdentityAuthority};
    match wire {
        CompatibilityDetected::Unknown => Ok(yoctui_model::AuthoritativeValue::Unknown),
        CompatibilityDetected::Detected { value, authority } => {
            let authority = match authority {
                CompatibilityIdentityAuthority::BackendHandshake => {
                    yoctui_model::IdentityAuthority::BackendHandshake
                }
                CompatibilityIdentityAuthority::BitBakeDatastore => {
                    yoctui_model::IdentityAuthority::BitBakeDatastore
                }
                CompatibilityIdentityAuthority::BitBakeVersionProbe => {
                    yoctui_model::IdentityAuthority::BitBakeVersionProbe
                }
                CompatibilityIdentityAuthority::ConfiguredLayerMetadata => {
                    yoctui_model::IdentityAuthority::ConfiguredLayerMetadata
                }
                CompatibilityIdentityAuthority::ExecutableProbe => {
                    yoctui_model::IdentityAuthority::ExecutableProbe
                }
                CompatibilityIdentityAuthority::InitializedEnvironment => {
                    yoctui_model::IdentityAuthority::InitializedEnvironment
                }
                CompatibilityIdentityAuthority::ProtocolNegotiation => {
                    yoctui_model::IdentityAuthority::ProtocolNegotiation
                }
                CompatibilityIdentityAuthority::ReleaseMetadata => {
                    yoctui_model::IdentityAuthority::ReleaseMetadata
                }
                CompatibilityIdentityAuthority::Unknown => {
                    return Err("unknown compatibility identity authority".into());
                }
            };
            Ok(yoctui_model::AuthoritativeValue::detected(
                map(value)?,
                authority,
            ))
        }
    }
}

pub(crate) fn stable_workspace_hash(source: &str, build: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in source.bytes().chain([0]).chain(build.bytes()) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

pub(crate) fn daemon_bitbake_lifecycle(
    lifecycle: yoctui_model::DaemonBitBakeLifecycle,
) -> yoctui_protocol::daemon::LifecycleState {
    use yoctui_protocol::daemon::LifecycleState;
    match lifecycle {
        yoctui_model::DaemonBitBakeLifecycle::Disconnected => LifecycleState::Disconnected,
        yoctui_model::DaemonBitBakeLifecycle::Connecting => LifecycleState::Connecting,
        yoctui_model::DaemonBitBakeLifecycle::Connected => LifecycleState::Running,
        yoctui_model::DaemonBitBakeLifecycle::Stopping => LifecycleState::Stopping,
        yoctui_model::DaemonBitBakeLifecycle::Failed => LifecycleState::Failed,
        yoctui_model::DaemonBitBakeLifecycle::Recovering => LifecycleState::Connecting,
    }
}

pub(crate) fn daemon_job_kind(
    kind: yoctui_model::BackgroundJobKind,
) -> yoctui_protocol::daemon::JobKind {
    use yoctui_protocol::daemon::JobKind;
    match kind {
        yoctui_model::BackgroundJobKind::Build => JobKind::BitBakeBuild,
        yoctui_model::BackgroundJobKind::CveCheck => JobKind::Security,
        yoctui_model::BackgroundJobKind::Spdx => JobKind::Qa,
        yoctui_model::BackgroundJobKind::Qemu => JobKind::Qemu,
        yoctui_model::BackgroundJobKind::Wic => JobKind::Wic,
        yoctui_model::BackgroundJobKind::Sdk => JobKind::Sdk,
        yoctui_model::BackgroundJobKind::Test => JobKind::Testing,
        yoctui_model::BackgroundJobKind::Devtool => JobKind::Devtool,
        yoctui_model::BackgroundJobKind::Maintenance => JobKind::Maintenance,
    }
}

pub(crate) fn daemon_job_lifecycle(
    status: yoctui_model::BackgroundJobStatus,
) -> yoctui_protocol::daemon::LifecycleState {
    use yoctui_protocol::daemon::LifecycleState;
    match status {
        yoctui_model::BackgroundJobStatus::Queued | yoctui_model::BackgroundJobStatus::Starting => {
            LifecycleState::Connecting
        }
        yoctui_model::BackgroundJobStatus::Running => LifecycleState::Running,
        yoctui_model::BackgroundJobStatus::Cancelling => LifecycleState::Stopping,
        yoctui_model::BackgroundJobStatus::Succeeded => LifecycleState::Exited,
        yoctui_model::BackgroundJobStatus::Failed => LifecycleState::Failed,
        yoctui_model::BackgroundJobStatus::Cancelled => LifecycleState::Exited,
        yoctui_model::BackgroundJobStatus::Lost => LifecycleState::Lost,
    }
}
