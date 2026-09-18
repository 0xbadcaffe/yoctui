use super::*;

pub(super) struct DaemonServices {
    pub(super) daemon_state: yoctui_model::DaemonGlobalState,
    pub(super) daemon_journal: DaemonSnapshotJournal,
    pub(super) startup_environment: BTreeMap<String, String>,
    pub(super) startup_compatibility:
        daemon_metadata::StartupMetadata<yoctui_model::DaemonCompatibilitySnapshot>,
    pub(super) startup_metadata: Option<daemon_metadata::StartupMetadata>,
    pub(super) startup_configured: bool,
    pub(super) rootfs_environment: BTreeMap<String, String>,
    pub(super) rootfs_query_permit: std::sync::Arc<tokio::sync::Semaphore>,
    pub(super) instance: yoctui_protocol::daemon::DaemonInstanceId,
    pub(super) shutting_down: bool,
    pub(super) devtool_supervisor: daemon_devtool::DaemonDevtoolSupervisor,
    pub(super) raw_supervisor: daemon_raw::DaemonRawSupervisor,
    pub(super) bitbake_supervisor: daemon_bitbake::DaemonBitBakeSupervisor,
    pub(super) sdk_supervisor: daemon_sdk::DaemonSdkSupervisor,
    pub(super) qemu_supervisor: daemon_qemu::DaemonQemuSupervisor,
    pub(super) wic_supervisor: daemon_wic::DaemonWicSupervisor,
    pub(super) test_supervisor: daemon_test::DaemonTestSupervisor,
    pub(super) qa_supervisor: daemon_qa::DaemonQaSupervisor,
    pub(super) qa_report_supervisor: daemon_qa::DaemonQaReportSupervisor,
    pub(super) security_supervisor: daemon_security::DaemonSecuritySupervisor,
    pub(super) security_mapper_supervisor: daemon_security::DaemonSecurityMapperSupervisor,
    pub(super) maintenance_supervisor: daemon_maintenance::DaemonMaintenanceSupervisor,
    pub(super) pty_supervisor: daemon_pty::DaemonPtySupervisor,
}

pub(super) struct DaemonClient {
    pub(super) connection: DaemonConnection,
    pub(super) negotiated: bool,
    pub(super) attached: bool,
    pub(super) last_sequence: u64,
    pub(super) client_id: ClientId,
    pub(super) rootfs_query: Option<daemon_rootfs::PendingQuery>,
}

pub(super) type ClientConnection = (
    DaemonConnection,
    bool,
    bool,
    u64,
    ClientId,
    Option<daemon_rootfs::PendingQuery>,
);
