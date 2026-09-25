use yoctui_app::PrefixCommand;
use yoctui_model::{App, ClientDaemonLifecycle, Effect};
use yoctui_protocol::daemon::{
    ClientLayoutEvent, CommandRequest, DaemonCommand, DaemonDevtoolOperation,
    DaemonTestSelftestRequest, JobId, PaneId, PtySessionId, RequestId, TerminalDimensions,
};

use super::{
    ClientRuntimeError, InteractiveDaemonRuntime, RuntimeEffectRoute,
    effect_inputs::{
        qa_capability_request, qemu_executable, sdk_context, wic_executable, wire_qemu_request,
        wire_sdk_operation, wire_wic_create,
    },
    terminal_control::prefix_daemon_command,
};

impl InteractiveDaemonRuntime {
    pub fn route_effect(
        &mut self,
        app: &App,
        effect: &Effect,
    ) -> Result<RuntimeEffectRoute, ClientRuntimeError> {
        if let Effect::Terminal(effect) = effect {
            return self.route_terminal_effect(app, effect);
        }
        let Some(command) = daemon_command_for_effect(app, effect)? else {
            return Ok(RuntimeEffectRoute::ClientLocal);
        };
        let request_id = RequestId(self.next_request);
        self.next_request = self
            .next_request
            .checked_add(1)
            .ok_or(ClientRuntimeError::RequestSpaceExhausted)?;
        let command = match command {
            DaemonCommand::InspectQaCapability { mut request } => {
                request.request_id = request_id;
                DaemonCommand::InspectQaCapability { request }
            }
            command => command,
        };
        self.transport.command(CommandRequest {
            request_id,
            expected_generation: Some(app.daemon.generation),
            command,
        })?;
        Ok(RuntimeEffectRoute::Daemon(request_id))
    }

    pub fn route_prefix(
        &mut self,
        app: &App,
        command: PrefixCommand,
    ) -> Result<RuntimeEffectRoute, ClientRuntimeError> {
        let Some(daemon_command) = prefix_daemon_command(app, command)? else {
            return Ok(RuntimeEffectRoute::ClientLocal);
        };
        if command == PrefixCommand::TakeControl
            && let Some(session) = app.selected_terminal_session()
        {
            self.transport
                .pty_layout(ClientLayoutEvent::AttachSession {
                    pane_id: PaneId(app.pane_layout.focused.0),
                    session_id: PtySessionId(session.id),
                })?;
        }
        let request_id = RequestId(self.next_request);
        self.next_request = self
            .next_request
            .checked_add(1)
            .ok_or(ClientRuntimeError::RequestSpaceExhausted)?;
        self.transport.command(CommandRequest {
            request_id,
            expected_generation: Some(app.daemon.generation),
            command: daemon_command,
        })?;
        Ok(RuntimeEffectRoute::Daemon(request_id))
    }
}

pub(super) fn daemon_command_for_effect(
    app: &App,
    effect: &Effect,
) -> Result<Option<DaemonCommand>, ClientRuntimeError> {
    let build_directory = || {
        app.workspace
            .build_dir
            .as_ref()
            .map(|path| path.display().to_string())
            .ok_or(ClientRuntimeError::MissingBuildDirectory)
    };
    Ok(Some(match effect {
        Effect::Start(request) => DaemonCommand::StartBuild {
            targets: request.targets.clone(),
            task: request.task.clone(),
            force: request.force,
        },
        Effect::Cancel => {
            let job = app
                .daemon
                .jobs
                .iter()
                .find(|job| {
                    matches!(
                        job.lifecycle,
                        ClientDaemonLifecycle::Connecting | ClientDaemonLifecycle::Running
                    )
                })
                .ok_or(ClientRuntimeError::NoActiveDaemonJob)?;
            DaemonCommand::CancelJob {
                job_id: JobId(job.id),
            }
        }
        Effect::StartRaw(request) => {
            let request = yoctui_app::raw_execution_request_to_protocol(request)
                .map_err(ClientRuntimeError::RawExecution)?;
            match request.interaction {
                yoctui_protocol::daemon::RawInteractionData::NoninteractiveJob => {
                    DaemonCommand::StartRaw { request }
                }
                yoctui_protocol::daemon::RawInteractionData::InteractivePty => {
                    DaemonCommand::StartRawPty {
                        request,
                        dimensions: TerminalDimensions {
                            columns: 120,
                            rows: 40,
                        },
                    }
                }
                yoctui_protocol::daemon::RawInteractionData::Unknown => {
                    return Err(ClientRuntimeError::RawExecution(
                        "Raw interaction mode is not supported".into(),
                    ));
                }
            }
        }
        Effect::CancelRaw(request_id) => DaemonCommand::CancelRaw {
            request_id: request_id.as_str().into(),
        },
        Effect::SetRawAttachment { request, attached } => DaemonCommand::SetRawAttachment {
            request_id: request.as_str().into(),
            attached: *attached,
        },
        Effect::InspectDevtoolStatus(identity) => DaemonCommand::InspectDevtoolStatus {
            recipe: identity.name.clone(),
            recipe_file: identity.file.display().to_string(),
            build_directory: build_directory()?,
        },
        Effect::DevtoolModify(identity) => DaemonCommand::StartDevtool {
            operation: DaemonDevtoolOperation::Modify {
                recipe: identity.name.clone(),
            },
            build_directory: build_directory()?,
        },
        Effect::DevtoolReset(plan) => DaemonCommand::StartDevtool {
            operation: DaemonDevtoolOperation::Reset {
                recipe: plan.identity.name.clone(),
            },
            build_directory: build_directory()?,
        },
        Effect::DevtoolUpdateRecipe(identity) => DaemonCommand::StartDevtool {
            operation: DaemonDevtoolOperation::UpdateRecipe {
                recipe: identity.name.clone(),
            },
            build_directory: build_directory()?,
        },
        Effect::DevtoolUpdateRecipePatch(plan) => DaemonCommand::StartDevtool {
            operation: DaemonDevtoolOperation::UpdateRecipePatch {
                recipe: plan.identity.name.clone(),
                destination: plan.layer.path.display().to_string(),
            },
            build_directory: build_directory()?,
        },
        Effect::DevtoolFinish(plan) => DaemonCommand::StartDevtool {
            operation: DaemonDevtoolOperation::Finish {
                recipe: plan.identity.name.clone(),
                destination: plan.layer.path.display().to_string(),
            },
            build_directory: build_directory()?,
        },
        Effect::DevtoolDeploy(plan) => DaemonCommand::StartDevtool {
            operation: DaemonDevtoolOperation::DeployTarget {
                recipe: plan.identity.name.clone(),
                target: plan.target.clone(),
            },
            build_directory: build_directory()?,
        },
        Effect::DevtoolUndeploy(plan) => DaemonCommand::StartDevtool {
            operation: DaemonDevtoolOperation::UndeployTarget {
                recipe: plan.identity.name.clone(),
                target: plan.target.clone(),
            },
            build_directory: build_directory()?,
        },
        Effect::DevtoolUpgrade(plan) => DaemonCommand::StartDevtool {
            operation: DaemonDevtoolOperation::Upgrade {
                recipe: plan.identity.name.clone(),
            },
            build_directory: build_directory()?,
        },
        Effect::StartSdkSession { id, operation } => DaemonCommand::StartSdk {
            session_id: id.0,
            operation: wire_sdk_operation(operation),
            context: sdk_context(app, operation)?,
        },
        Effect::CancelSdkSession(id) => DaemonCommand::CancelSdk { session_id: id.0 },
        Effect::StartQemuSession { id, request } => DaemonCommand::StartQemu {
            session_id: id.0,
            request: wire_qemu_request(request),
            build_directory: build_directory()?,
            executable: qemu_executable(app, request)?,
        },
        Effect::CancelQemuSession(id) => DaemonCommand::CancelQemu { session_id: id.0 },
        Effect::StartWicSession { id, operation } => match operation {
            yoctui_model::WicOperation::Create(request) => DaemonCommand::StartWicCreate {
                session_id: id.0,
                request: wire_wic_create(request),
                build_directory: build_directory()?,
                executable: wic_executable(app)?,
            },
            yoctui_model::WicOperation::Write(request) => DaemonCommand::StartWicWrite {
                session_id: id.0,
                executable: request.executable.display().to_string(),
                image_path: request.image.path.display().to_string(),
                device_path: request.device.path.display().to_string(),
                device_major_minor: request.device.major_minor.clone(),
                device_size_bytes: request.device.size_bytes,
                device_model: request.device.model.clone(),
                device_serial: request.device.serial.clone(),
                device_transport: request.device.transport.clone(),
                build_directory: build_directory()?,
            },
        },
        Effect::CancelWicSession(id) => DaemonCommand::CancelWic { session_id: id.0 },
        Effect::StartTestSession {
            id,
            operation: yoctui_model::TestOperation::Selftest(request),
        } => DaemonCommand::StartTestSession {
            session_id: id.0,
            request: DaemonTestSelftestRequest {
                executable: request.executable.display().to_string(),
                family: format!("{:?}", request.family),
                selector: request.selector.clone(),
                parallelism: request.parallelism,
                verbose: request.verbose,
                skip_network: request.skip_network,
            },
            build_directory: build_directory()?,
            path_directories: app
                .workspace
                .source_dir
                .iter()
                .map(|path| path.display().to_string())
                .collect(),
        },
        Effect::StartTestSession { .. } => return Ok(None),
        Effect::CancelTestSession(id) => DaemonCommand::CancelTestSession { session_id: id.0 },
        Effect::ImportTestResults(request) => DaemonCommand::ImportTestResults {
            generation: request.generation,
            roots: request
                .roots
                .iter()
                .map(|path| path.display().to_string())
                .collect(),
        },
        Effect::CompareTestResults(request) => DaemonCommand::CompareTestResults {
            generation: request.generation,
            baseline_identity: format!("{:?}", request.baseline),
            candidate_identity: format!("{:?}", request.candidate),
        },
        Effect::ExportTestJunit(request) => DaemonCommand::ExportTestJunit {
            generation: request.generation,
            result_identity: format!("{:?}", request.result),
            destination: request.destination.display().to_string(),
        },
        Effect::InspectResultToolCapability => DaemonCommand::InspectTestResultTool {
            path_directories: app
                .workspace
                .source_dir
                .iter()
                .map(|path| path.display().to_string())
                .collect(),
        },
        Effect::Qa(yoctui_model::QaEffect::InspectCapability { scope }) => {
            DaemonCommand::InspectQaCapability {
                request: qa_capability_request(app, scope.as_ref())?,
            }
        }
        Effect::Qa(yoctui_model::QaEffect::ImportReports(request)) => {
            DaemonCommand::StartQaReportScan {
                generation: request.generation,
                build_directory: build_directory()?,
                paths: request
                    .paths
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect(),
            }
        }
        Effect::Security(yoctui_model::SecurityEffect::ImportReports(request)) => {
            DaemonCommand::StartSecurityReportScan {
                generation: request.generation,
                paths: request
                    .paths
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect(),
            }
        }
        Effect::Security(yoctui_model::SecurityEffect::StartPackageMap {
            id,
            executable,
            arguments,
        }) => {
            let preview = app
                .security
                .sessions
                .iter()
                .find(|session| session.preview.id == *id)
                .ok_or(ClientRuntimeError::MissingSecuritySession)?
                .preview
                .clone();
            DaemonCommand::StartSecurityPackageMap {
                session_id: id.0,
                executable: executable.display().to_string(),
                arguments: arguments.clone(),
                report_roots: preview
                    .report_roots
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect(),
            }
        }
        Effect::Security(yoctui_model::SecurityEffect::CancelSession(id)) => {
            DaemonCommand::CancelSecurityPackageMap { session_id: id.0 }
        }
        Effect::Maintenance(yoctui_model::MaintenanceEffect::InspectCapability { request }) => {
            DaemonCommand::InspectMaintenanceCapability {
                request: *request,
                build_directory: build_directory()?,
                sstate_directory: app.workspace.variables.get("SSTATE_DIR").cloned(),
                tmp_directory: app.workspace.variables.get("TMPDIR").cloned(),
                stamps_directories: app
                    .workspace
                    .variables
                    .get("STAMPS_DIR")
                    .or_else(|| app.workspace.variables.get("STAMP"))
                    .into_iter()
                    .cloned()
                    .collect(),
                executable_search_path: app
                    .workspace
                    .source_dir
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect(),
            }
        }
        Effect::Maintenance(yoctui_model::MaintenanceEffect::InspectServices { request }) => {
            DaemonCommand::InspectMaintenanceServices {
                request: *request,
                build_directory: build_directory()?,
                prserv_host: app.workspace.variables.get("PRSERV_HOST").cloned(),
                hashserve: app.workspace.variables.get("BB_HASHSERVE").cloned(),
                hashserve_upstream: app
                    .workspace
                    .variables
                    .get("BB_HASHSERVE_UPSTREAM")
                    .cloned(),
                signature_handler: app.workspace.variables.get("BB_SIGNATURE_HANDLER").cloned(),
                executable_search_path: app
                    .workspace
                    .source_dir
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect(),
                process_root: "/proc".into(),
            }
        }
        Effect::Maintenance(yoctui_model::MaintenanceEffect::StartOperation { id, preview }) => {
            if let yoctui_model::MaintenanceOperation::SstateReadiness(request) = &preview.operation
            {
                DaemonCommand::StartMaintenanceSstateReadiness {
                    session_id: id.0,
                    capability_request: preview.capability_request,
                    operation_id: preview.id,
                    build_directory: build_directory()?,
                    sstate_directory: app.workspace.variables.get("SSTATE_DIR").cloned(),
                    tmp_directory: app.workspace.variables.get("TMPDIR").cloned(),
                    stamps_directories: app
                        .workspace
                        .variables
                        .get("STAMPS_DIR")
                        .or_else(|| app.workspace.variables.get("STAMP"))
                        .into_iter()
                        .cloned()
                        .collect(),
                    executable_search_path: app
                        .workspace
                        .source_dir
                        .iter()
                        .map(|path| path.display().to_string())
                        .collect(),
                    targets: request.targets.clone(),
                    mode: format!("{:?}", request.mode).to_lowercase(),
                    output: request
                        .output
                        .as_ref()
                        .map(|path| path.display().to_string()),
                    log: request.log.as_ref().map(|path| path.display().to_string()),
                    timeout_seconds: request.timeout_seconds,
                }
            } else {
                let tool = match &preview.operation {
                    yoctui_model::MaintenanceOperation::LockedSignatureCache(_) => {
                        yoctui_model::MaintenanceTool::LockedSignatureCache
                    }
                    yoctui_model::MaintenanceOperation::BuildHistoryComparison(_) => {
                        yoctui_model::MaintenanceTool::BuildHistoryDiff
                    }
                    yoctui_model::MaintenanceOperation::BuildCompare(_) => {
                        yoctui_model::MaintenanceTool::BuildCompare
                    }
                    yoctui_model::MaintenanceOperation::GitArchive(_) => {
                        yoctui_model::MaintenanceTool::GitArchive
                    }
                    _ => return Ok(None),
                };
                let executable = app
                    .maintenance
                    .capability
                    .snapshot()
                    .and_then(|snapshot| {
                        snapshot.tools.iter().find_map(|entry| match entry {
                            yoctui_model::MaintenanceToolCapability::Available {
                                tool: candidate,
                                executable,
                                ..
                            } if *candidate == tool => Some(executable.path.clone()),
                            _ => None,
                        })
                    })
                    .ok_or(ClientRuntimeError::MissingMaintenanceTool)?;
                DaemonCommand::StartMaintenanceExternal {
                    session_id: id.0,
                    executable: executable.display().to_string(),
                    expected_name: executable
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or_default()
                        .into(),
                    arguments: preview.arguments.clone(),
                    current_directory: build_directory()?,
                }
            }
        }
        Effect::Maintenance(yoctui_model::MaintenanceEffect::CancelOperation(id)) => {
            DaemonCommand::CancelMaintenance { session_id: id.0 }
        }
        Effect::Qa(yoctui_model::QaEffect::StartLayerCheck {
            session,
            layer,
            executable,
            arguments,
        }) => {
            let operation = app
                .qa
                .layer_sessions
                .iter()
                .find(|candidate| candidate.id == *session)
                .ok_or(ClientRuntimeError::MissingQaLayerSession)?
                .operation
                .clone();
            DaemonCommand::StartQaLayerCheck {
                session_id: session.0,
                operation_id: operation.id.0,
                check_id: operation.check.0,
                layer_name: layer.name.clone(),
                layer_root: layer.root.display().to_string(),
                executable: executable.path.display().to_string(),
                arguments: arguments.clone(),
                report_roots: operation
                    .report_roots
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect(),
            }
        }
        Effect::Qa(yoctui_model::QaEffect::CancelLayerCheck(session)) => {
            DaemonCommand::CancelQaLayerCheck {
                session_id: session.0,
            }
        }
        _ => return Ok(None),
    }))
}
