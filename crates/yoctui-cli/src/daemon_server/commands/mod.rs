use super::*;
use yoctui_protocol::daemon::CommandRequest;

mod build;
mod devtool;
mod maintenance;
mod pty;
mod qa;
mod qemu;
mod raw;
mod rootfs;
mod sdk;
mod security;
mod testing;
mod wic;

pub(super) fn dispatch(
    request: CommandRequest,
    services: &mut DaemonServices,
    client: &mut DaemonClient,
) -> Result<Option<CommandOutcome>> {
    match &request.command {
        DaemonCommand::InspectRootfsSources { .. } => rootfs::handle(request, services, client),
        DaemonCommand::StartBuild { .. } | DaemonCommand::CancelJob { .. } => {
            build::handle(request, services, client)
        }
        DaemonCommand::StartDevtool { .. } => devtool::handle(request, services, client),
        DaemonCommand::StartRaw { .. }
        | DaemonCommand::StartRawPty { .. }
        | DaemonCommand::CancelRaw { .. }
        | DaemonCommand::SetRawAttachment { .. } => raw::handle(request, services, client),
        DaemonCommand::StartSdk { .. } | DaemonCommand::CancelSdk { .. } => {
            sdk::handle(request, services, client)
        }
        DaemonCommand::StartQemu { .. } | DaemonCommand::CancelQemu { .. } => {
            qemu::handle(request, services, client)
        }
        DaemonCommand::StartWicCreate { .. }
        | DaemonCommand::StartWicWrite { .. }
        | DaemonCommand::CancelWic { .. } => wic::handle(request, services, client),
        DaemonCommand::StartTestSession { .. }
        | DaemonCommand::CancelTestSession { .. }
        | DaemonCommand::ImportTestResults { .. }
        | DaemonCommand::CompareTestResults { .. }
        | DaemonCommand::ExportTestJunit { .. }
        | DaemonCommand::InspectTestResultTool { .. } => testing::handle(request, services, client),
        DaemonCommand::InspectQaCapability { .. }
        | DaemonCommand::StartQaLayerCheck { .. }
        | DaemonCommand::CancelQaLayerCheck { .. }
        | DaemonCommand::StartQaReportScan { .. }
        | DaemonCommand::CancelQaReportScan { .. } => qa::handle(request, services, client),
        DaemonCommand::StartSecurityReportScan { .. }
        | DaemonCommand::CancelSecurityReportScan { .. }
        | DaemonCommand::StartSecurityPackageMap { .. }
        | DaemonCommand::CancelSecurityPackageMap { .. } => {
            security::handle(request, services, client)
        }
        DaemonCommand::InspectMaintenanceCapability { .. }
        | DaemonCommand::StartMaintenanceSstateReadiness { .. }
        | DaemonCommand::CancelMaintenance { .. }
        | DaemonCommand::StartMaintenanceExternal { .. }
        | DaemonCommand::InspectMaintenanceServices { .. } => {
            maintenance::handle(request, services, client)
        }
        DaemonCommand::CreatePty { .. }
        | DaemonCommand::TakePtyControl { .. }
        | DaemonCommand::ReleasePtyControl { .. }
        | DaemonCommand::TerminatePty { .. }
        | DaemonCommand::RenamePty { .. }
        | DaemonCommand::ClosePty { .. } => pty::handle(request, services, client),
        _ => Ok(Some(CommandOutcome::Rejected {
            code: yoctui_protocol::daemon::ProtocolErrorCode::UnsupportedCapability,
            message: "daemon command is not implemented by this runtime".into(),
            current_generation: services.daemon_journal.snapshot().generation,
        })),
    }
}
