use std::{
    fs::File,
    io::Read,
    time::{Duration, Instant},
};

use thiserror::Error;
use yoctui_app::{DaemonClientSnapshot, PrefixCommand};
use yoctui_model::{App, ClientDaemonLifecycle, Effect, TerminalEffect};
use yoctui_protocol::daemon::{
    ClientId, ClientLayoutEvent, CommandRequest, DaemonCommand, DaemonDevtoolOperation,
    DaemonQaCapabilityInput, DaemonQaCapabilityRequest, DaemonQemuRequest,
    DaemonSdkArtifactIdentity, DaemonSdkContext, DaemonSdkNativeMode, DaemonSdkOperation,
    DaemonTestSelftestRequest, DaemonWicCreateRequest, JobId, PaneId, PtyInput, PtySessionId,
    PtyViewport, RequestId, Subscription, TerminalDimensions,
};

use crate::client_transport::{ClientServerEvent, ClientTransportError, DaemonClientTransport};

const MAX_EVENTS_PER_POLL: usize = 64;
const MAX_POLL_DURATION: Duration = Duration::from_millis(8);

pub struct InteractiveDaemonRuntime {
    transport: DaemonClientTransport,
    replica: DaemonClientSnapshot,
    next_request: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeEffectRoute {
    Daemon(RequestId),
    ClientLocal,
}

impl InteractiveDaemonRuntime {
    pub fn connect(app: &mut App, timeout: Duration) -> Result<Self, ClientRuntimeError> {
        let client_id = random_client_id()?;
        let mut transport =
            DaemonClientTransport::connect(client_id, "yoctui-ratatui".into(), timeout)?;
        let attached = transport.attach(
            None,
            Subscription {
                state: true,
                jobs: true,
                logs: true,
                pty_sessions: Vec::new(),
            },
            None,
        )?;
        let mut replica = DaemonClientSnapshot::default();
        replica.begin_synchronization();
        replica.replace_app(app, attached.snapshot);
        for event in attached.replayed_events {
            replica.apply_event_to_app(app, &event)?;
        }
        app.terminal.client_id = Some(client_id.0);
        Ok(Self {
            transport,
            replica,
            next_request: 1,
        })
    }

    pub fn poll(&mut self, app: &mut App) -> Result<bool, ClientRuntimeError> {
        let mut received = false;
        let started = Instant::now();
        for _ in 0..MAX_EVENTS_PER_POLL {
            let Some(event) = self.transport.try_receive(Duration::from_millis(1))? else {
                break;
            };
            received = true;
            match event {
                ClientServerEvent::Snapshot(snapshot) => self.replica.replace_app(app, *snapshot),
                ClientServerEvent::Event(event) => self.replica.apply_event_to_app(app, &event)?,
                ClientServerEvent::ResyncRequired { reason, .. } => {
                    self.replica.begin_synchronization();
                    self.replica.install_app(app);
                    app.notification = Some(format!("Daemon resynchronization required: {reason}"));
                }
                ClientServerEvent::CommandResult(result) => {
                    app.notification = Some(format!(
                        "Daemon request {}: {:?}",
                        result.request_id.0, result.outcome
                    ));
                }
                ClientServerEvent::ShuttingDown => {
                    self.replica.disconnect_app(app);
                    app.notification = Some("Yoctui daemon is shutting down.".into());
                }
            }
            if started.elapsed() >= MAX_POLL_DURATION {
                break;
            }
        }
        Ok(received)
    }

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

    fn route_terminal_effect(
        &mut self,
        app: &App,
        effect: &TerminalEffect,
    ) -> Result<RuntimeEffectRoute, ClientRuntimeError> {
        let request_id = RequestId(self.next_request);
        self.next_request = self
            .next_request
            .checked_add(1)
            .ok_or(ClientRuntimeError::RequestSpaceExhausted)?;
        match effect {
            TerminalEffect::Create {
                name,
                kind,
                cwd,
                program,
                arguments,
            } => self.transport.command(CommandRequest {
                request_id,
                expected_generation: Some(app.daemon.generation),
                command: DaemonCommand::CreatePty {
                    name: name.clone(),
                    kind: match kind {
                        yoctui_model::TerminalCreationKind::BuildShell => {
                            yoctui_protocol::daemon::PtyKind::BuildShell
                        }
                        yoctui_model::TerminalCreationKind::Devshell => {
                            yoctui_protocol::daemon::PtyKind::Devshell
                        }
                        yoctui_model::TerminalCreationKind::Menuconfig => {
                            yoctui_protocol::daemon::PtyKind::Menuconfig
                        }
                    },
                    cwd: cwd.display().to_string(),
                    command: yoctui_protocol::daemon::PtyCommand {
                        program: program.display().to_string(),
                        arguments: arguments.clone(),
                        environment_profile_id: None,
                    },
                    dimensions: TerminalDimensions {
                        columns: 120,
                        rows: 40,
                    },
                },
            })?,
            TerminalEffect::TakeControl {
                session_id,
                expected_epoch,
            } => {
                self.transport
                    .pty_layout(ClientLayoutEvent::AttachSession {
                        pane_id: PaneId(app.pane_layout.focused.0),
                        session_id: PtySessionId(*session_id),
                    })?;
                self.transport.command(CommandRequest {
                    request_id,
                    expected_generation: Some(app.daemon.generation),
                    command: DaemonCommand::TakePtyControl {
                        session_id: PtySessionId(*session_id),
                        expected_epoch: *expected_epoch,
                    },
                })?;
            }
            TerminalEffect::ReleaseControl {
                session_id,
                writer_epoch,
            } => self.transport.command(CommandRequest {
                request_id,
                expected_generation: Some(app.daemon.generation),
                command: DaemonCommand::ReleasePtyControl {
                    session_id: PtySessionId(*session_id),
                    expected_epoch: *writer_epoch,
                },
            })?,
            TerminalEffect::Input {
                session_id,
                writer_epoch,
                bytes,
            } => self.transport.pty_input(PtyInput {
                request_id,
                session_id: PtySessionId(*session_id),
                writer_epoch: *writer_epoch,
                bytes: bytes.clone(),
            })?,
            TerminalEffect::Viewport {
                session_id,
                scrollback_offset,
            } => self.transport.pty_viewport(PtyViewport {
                request_id,
                session_id: PtySessionId(*session_id),
                scrollback_offset: u32::try_from(*scrollback_offset)
                    .map_err(|_| ClientRuntimeError::InvalidTerminalViewport)?,
            })?,
            TerminalEffect::Rename { session_id, name } => {
                self.transport.command(CommandRequest {
                    request_id,
                    expected_generation: Some(app.daemon.generation),
                    command: DaemonCommand::RenamePty {
                        session_id: PtySessionId(*session_id),
                        name: name.clone(),
                    },
                })?;
            }
            TerminalEffect::Terminate { session_id } => {
                self.transport.command(CommandRequest {
                    request_id,
                    expected_generation: Some(app.daemon.generation),
                    command: DaemonCommand::TerminatePty {
                        session_id: PtySessionId(*session_id),
                        force: true,
                        confirmation: None,
                    },
                })?;
            }
            TerminalEffect::Close { session_id } => {
                self.transport.command(CommandRequest {
                    request_id,
                    expected_generation: Some(app.daemon.generation),
                    command: DaemonCommand::ClosePty {
                        session_id: PtySessionId(*session_id),
                    },
                })?;
            }
        }
        Ok(RuntimeEffectRoute::Daemon(request_id))
    }

    pub fn detach(mut self, app: &mut App) -> Result<(), ClientRuntimeError> {
        self.transport.detach()?;
        self.replica.disconnect_app(app);
        Ok(())
    }

    pub fn detach_terminal(&mut self, app: &App) -> Result<(), ClientRuntimeError> {
        let session = app
            .selected_terminal_session()
            .ok_or(ClientRuntimeError::MissingPtySession)?;
        self.transport
            .pty_layout(ClientLayoutEvent::DetachSession {
                pane_id: PaneId(app.pane_layout.focused.0),
                session_id: PtySessionId(session.id),
            })?;
        Ok(())
    }
}

fn prefix_daemon_command(
    app: &App,
    command: PrefixCommand,
) -> Result<Option<DaemonCommand>, ClientRuntimeError> {
    let daemon_command = match command {
        PrefixCommand::CreateSession => {
            let cwd = app
                .workspace
                .build_dir
                .as_ref()
                .ok_or(ClientRuntimeError::MissingBuildDirectory)?;
            DaemonCommand::CreatePty {
                name: "build shell".into(),
                kind: yoctui_protocol::daemon::PtyKind::BuildShell,
                cwd: cwd.display().to_string(),
                command: yoctui_protocol::daemon::PtyCommand {
                    program: "/bin/sh".into(),
                    arguments: Vec::new(),
                    environment_profile_id: None,
                },
                dimensions: yoctui_protocol::daemon::TerminalDimensions {
                    columns: 120,
                    rows: 40,
                },
            }
        }
        PrefixCommand::TakeControl => {
            let session = app
                .daemon
                .pty_sessions
                .get(app.pty_selection)
                .filter(|session| matches!(session.lifecycle, ClientDaemonLifecycle::Running))
                .ok_or(ClientRuntimeError::MissingPtySession)?;
            let details = app
                .selected_terminal_details()
                .ok_or(ClientRuntimeError::MissingPtySession)?;
            DaemonCommand::TakePtyControl {
                session_id: yoctui_protocol::daemon::PtySessionId(session.id),
                expected_epoch: details.writer_epoch,
            }
        }
        PrefixCommand::CommandPalette
        | PrefixCommand::Help
        | PrefixCommand::OpenTerminalSessions
        | PrefixCommand::CopyMode
        | PrefixCommand::Search
        | PrefixCommand::Rename
        | PrefixCommand::ReleaseControl
        | PrefixCommand::Kill
        | PrefixCommand::Zoom => return Ok(None),
        PrefixCommand::NextSession
        | PrefixCommand::PreviousSession
        | PrefixCommand::SplitHorizontal
        | PrefixCommand::SplitVertical
        | PrefixCommand::ClosePane
        | PrefixCommand::Detach => return Ok(None),
    };
    Ok(Some(daemon_command))
}

fn daemon_command_for_effect(
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

fn qa_capability_request(
    app: &App,
    requested_scope: Option<&yoctui_model::QaScope>,
) -> Result<DaemonQaCapabilityRequest, ClientRuntimeError> {
    let selected = requested_scope
        .map(|scope| scope.recipe.clone())
        .or_else(|| app.qa.scope.as_ref().map(|scope| scope.recipe.clone()))
        .or_else(|| {
            app.workspace
                .recipes
                .get(app.recipe_selection)
                .and_then(|recipe| {
                    recipe
                        .file
                        .clone()
                        .map(|file| yoctui_model::RecipeIdentity {
                            name: recipe.name.clone(),
                            file,
                        })
                })
        })
        .or_else(|| {
            app.workspace.recipes.iter().find_map(|recipe| {
                recipe
                    .file
                    .clone()
                    .map(|file| yoctui_model::RecipeIdentity {
                        name: recipe.name.clone(),
                        file,
                    })
            })
        })
        .ok_or(ClientRuntimeError::MissingQaLayerSession)?;
    let recipe_names = app
        .workspace
        .recipes
        .iter()
        .map(|recipe| recipe.name.clone())
        .collect();
    let report_roots = app
        .workspace
        .variables
        .iter()
        .filter(|(name, _)| name.ends_with("_REPORT_ROOT"))
        .map(|(_, value)| value.clone())
        .collect();
    Ok(DaemonQaCapabilityRequest {
        request_id: RequestId(0),
        input: DaemonQaCapabilityInput {
            generation: app.daemon.generation,
            build_directory: app
                .workspace
                .build_dir
                .as_ref()
                .map(|path| path.display().to_string())
                .ok_or(ClientRuntimeError::MissingBuildDirectory)?,
            source_directory: app
                .workspace
                .source_dir
                .as_ref()
                .map(|path| path.display().to_string()),
            layer_directories: app
                .workspace
                .layers
                .iter()
                .map(|layer| layer.path.display().to_string())
                .collect(),
            recipe_names,
            report_roots,
            selected_recipe_name: selected.name,
            selected_recipe_file: selected.file.display().to_string(),
        },
    })
}

fn qemu_executable(
    app: &App,
    request: &yoctui_model::QemuLaunchRequest,
) -> Result<String, ClientRuntimeError> {
    let yoctui_model::QemuCapability::Available {
        executable,
        compatible_images,
    } = &app.qemu_capability
    else {
        return Err(ClientRuntimeError::MissingQemuCapability);
    };
    if !compatible_images
        .iter()
        .any(|image| image == &request.image)
    {
        return Err(ClientRuntimeError::MissingQemuCapability);
    }
    Ok(executable.display().to_string())
}

fn wire_qemu_request(request: &yoctui_model::QemuLaunchRequest) -> DaemonQemuRequest {
    DaemonQemuRequest {
        machine: request.machine.clone(),
        image_machine: request.image.machine.clone(),
        image: request.image.image.clone(),
        image_path: request.image.path.display().to_string(),
        artifact_kind: format!("{:?}", request.artifact_kind),
        kernel: request
            .kernel
            .as_ref()
            .map(|path| path.display().to_string()),
        rootfs: request
            .rootfs
            .as_ref()
            .map(|path| path.display().to_string()),
        networking: format!("{:?}", request.networking),
        display: format!("{:?}", request.display),
        serial: format!("{:?}", request.serial),
        memory_mib: request.memory_mib,
        extra_arguments: request.extra_arguments.clone(),
    }
}

fn wic_executable(app: &App) -> Result<String, ClientRuntimeError> {
    let yoctui_model::WicCapability::Available { executable, .. } = &app.wic_capability else {
        return Err(ClientRuntimeError::MissingWicCapability);
    };
    Ok(executable.display().to_string())
}

fn wire_wic_create(request: &yoctui_model::WicCreateRequest) -> DaemonWicCreateRequest {
    DaemonWicCreateRequest {
        machine: request.machine.clone(),
        image: request.image.clone(),
        kickstart_name: request.kickstart.name.clone(),
        kickstart_path: request
            .kickstart
            .path
            .as_ref()
            .map(|path| path.display().to_string()),
        output_directory: request.output_directory.display().to_string(),
        generate_bmap: request.generate_bmap,
        compression: format!("{:?}", request.compression),
    }
}

fn wire_sdk_operation(operation: &yoctui_model::SdkOperation) -> DaemonSdkOperation {
    match operation {
        yoctui_model::SdkOperation::Publish(request) => DaemonSdkOperation::Publish {
            executable: request.executable.display().to_string(),
            artifact: DaemonSdkArtifactIdentity {
                path: request.artifact.path.display().to_string(),
                size_bytes: request.artifact.size_bytes,
                modified_unix_seconds: request.artifact.modified_unix_seconds,
            },
            destination: request.destination.display().to_string(),
        },
        yoctui_model::SdkOperation::Native(request) => DaemonSdkOperation::Native {
            executable: request.executable.display().to_string(),
            mode: match request.mode {
                yoctui_model::SdkNativeMode::FindSysroot => DaemonSdkNativeMode::FindSysroot,
                yoctui_model::SdkNativeMode::RunNative => DaemonSdkNativeMode::RunNative,
            },
            extracted_root: request
                .extracted_root
                .as_ref()
                .map(|path| path.display().to_string()),
            recipe: request.recipe.clone(),
            tool: request.tool.clone(),
            arguments: request.arguments.clone(),
        },
    }
}

fn sdk_context(
    app: &App,
    operation: &yoctui_model::SdkOperation,
) -> Result<DaemonSdkContext, ClientRuntimeError> {
    let build_directory = app
        .workspace
        .build_dir
        .as_ref()
        .ok_or(ClientRuntimeError::MissingBuildDirectory)?;
    let sdk_deploy_root = app
        .workspace
        .variables
        .get("SDK_DEPLOY")
        .cloned()
        .ok_or(ClientRuntimeError::MissingSdkDeployRoot)?;
    let executable = match operation {
        yoctui_model::SdkOperation::Publish(request) => &request.executable,
        yoctui_model::SdkOperation::Native(request) => &request.executable,
    };
    let mut workspace_roots = Vec::new();
    if let Some(source) = &app.workspace.source_dir {
        workspace_roots.push(source.display().to_string());
    }
    if let Some(parent) = executable.parent() {
        let root = if parent.file_name().is_some_and(|name| name == "scripts") {
            parent.parent().unwrap_or(parent)
        } else {
            parent
        };
        let root = root.display().to_string();
        if !workspace_roots.contains(&root) {
            workspace_roots.push(root);
        }
    }
    if workspace_roots.is_empty() {
        return Err(ClientRuntimeError::MissingSdkWorkspaceRoot);
    }
    Ok(DaemonSdkContext {
        build_directory: build_directory.display().to_string(),
        sdk_deploy_root,
        workspace_roots,
    })
}

fn random_client_id() -> Result<ClientId, ClientRuntimeError> {
    let mut identity = [0_u8; 16];
    File::open("/dev/urandom")?.read_exact(&mut identity)?;
    if identity == [0; 16] {
        return Err(ClientRuntimeError::InvalidRandomIdentity);
    }
    Ok(ClientId(identity))
}

#[derive(Debug, Error)]
pub enum ClientRuntimeError {
    #[error(transparent)]
    Transport(#[from] ClientTransportError),
    #[error(transparent)]
    Replica(#[from] yoctui_app::DaemonClientSyncError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("random client identity was zero")]
    InvalidRandomIdentity,
    #[error("daemon has no active job to cancel")]
    NoActiveDaemonJob,
    #[error("daemon request ID space exhausted")]
    RequestSpaceExhausted,
    #[error("terminal viewport offset exceeds the wire range")]
    InvalidTerminalViewport,
    #[error("Raw execution request could not be encoded: {0}")]
    RawExecution(String),
    #[error("no running PTY session is available")]
    MissingPtySession,
    #[error("authoritative build directory is unavailable")]
    MissingBuildDirectory,
    #[error("authoritative SDK deploy root is unavailable")]
    MissingSdkDeployRoot,
    #[error("authoritative SDK tool root is unavailable")]
    MissingSdkWorkspaceRoot,
    #[error("runqemu capability is unavailable for the selected image")]
    MissingQemuCapability,
    #[error("Wic capability is unavailable")]
    MissingWicCapability,
    #[error("QA layer session is unavailable")]
    MissingQaLayerSession,
    #[error("security session is unavailable")]
    MissingSecuritySession,
    #[error("maintenance tool is unavailable")]
    MissingMaintenanceTool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_runtime_effect_mapping_uses_daemon_global_state() {
        let request = yoctui_model::BuildRequest {
            targets: vec!["core-image-minimal".into()],
            task: Some("build".into()),
            force: false,
        };
        let command = match Effect::Start(request.clone()) {
            Effect::Start(request) => DaemonCommand::StartBuild {
                targets: request.targets,
                task: request.task,
                force: request.force,
            },
            _ => unreachable!(),
        };
        assert!(matches!(
            command,
            DaemonCommand::StartBuild { targets, task: Some(task), force: false }
                if targets == request.targets && task == "build"
        ));
        let mut app = App::new(16, 4096);
        app.daemon.jobs.push(yoctui_model::ClientDaemonJobSummary {
            id: 71,
            label: "core-image-minimal".into(),
            lifecycle: ClientDaemonLifecycle::Running,
        });
        assert_eq!(app.daemon.jobs[0].id, 71);
        assert!(matches!(Effect::PersistSettings, Effect::PersistSettings));
    }

    #[test]
    fn raw_job_effects_map_only_to_typed_daemon_commands() {
        let request = yoctui_model::RawConfirmedExecutionRequest {
            id: yoctui_model::RawRequestId::new("raw-request:client-runtime-1").unwrap(),
            catalog_version: 1,
            command: yoctui_model::RawCommandId::new("build.target").unwrap(),
            parameters: std::collections::BTreeMap::from([(
                yoctui_model::RawParameterId::new("target").unwrap(),
                yoctui_model::RawParameterValue::Target("core-image-minimal".into()),
            )]),
            additional_arguments: vec!["--dry-run".into()],
            interaction: yoctui_model::RawInteractionMode::NoninteractiveJob,
            safety: yoctui_model::RawSafetyClass::Build,
            capability_generation: 4,
            build_directory: "/work/build".into(),
            preview_digest: yoctui_model::RawPreviewDigest([3; 32]),
        };
        let app = App::new(16, 4096);
        let Some(DaemonCommand::StartRaw { request: wire }) =
            daemon_command_for_effect(&app, &Effect::StartRaw(request.clone())).unwrap()
        else {
            panic!("expected typed Raw start command");
        };
        assert_eq!(
            yoctui_app::raw_execution_request_from_protocol(&wire).unwrap(),
            request
        );
        assert_eq!(
            daemon_command_for_effect(&app, &Effect::CancelRaw(request.id.clone())).unwrap(),
            Some(DaemonCommand::CancelRaw {
                request_id: request.id.as_str().into(),
            })
        );
    }

    #[test]
    fn raw_pty_effect_maps_to_confirmed_request_and_bounded_dimensions() {
        let request = yoctui_model::RawConfirmedExecutionRequest {
            id: yoctui_model::RawRequestId::new("raw-request:client-runtime-pty").unwrap(),
            catalog_version: 1,
            command: yoctui_model::RawCommandId::new("ui.knotty").unwrap(),
            parameters: std::collections::BTreeMap::new(),
            additional_arguments: Vec::new(),
            interaction: yoctui_model::RawInteractionMode::InteractivePty,
            safety: yoctui_model::RawSafetyClass::Build,
            capability_generation: 4,
            build_directory: "/work/build".into(),
            preview_digest: yoctui_model::RawPreviewDigest([4; 32]),
        };
        let app = App::new(16, 4096);
        let Some(DaemonCommand::StartRawPty {
            request: wire,
            dimensions,
        }) = daemon_command_for_effect(&app, &Effect::StartRaw(request.clone())).unwrap()
        else {
            panic!("expected typed Raw PTY start command");
        };
        assert_eq!(
            yoctui_app::raw_execution_request_from_protocol(&wire).unwrap(),
            request
        );
        assert!(dimensions.columns <= 512 && dimensions.rows <= 512);
    }

    #[test]
    fn raw_output_attachment_effect_maps_only_request_identity_and_state() {
        let app = App::new(16, 4096);
        let request = yoctui_model::RawRequestId::new("raw-request:client-output").unwrap();
        assert_eq!(
            daemon_command_for_effect(
                &app,
                &Effect::SetRawAttachment {
                    request: request.clone(),
                    attached: false,
                },
            )
            .unwrap(),
            Some(DaemonCommand::SetRawAttachment {
                request_id: request.as_str().into(),
                attached: false,
            })
        );
    }

    #[test]
    fn client_runtime_random_identity_is_nonzero() {
        assert_ne!(random_client_id().unwrap().0, [0; 16]);
    }

    #[test]
    fn client_runtime_qa_task_maps_capability_inspection_to_typed_daemon_input() {
        let mut app = App::new(16, 4096);
        app.workspace.build_dir = Some("/build".into());
        app.workspace.recipes.push(yoctui_model::Recipe {
            name: "busybox".into(),
            file: Some("/layers/busybox.bb".into()),
            ..Default::default()
        });
        let effect = Effect::Qa(yoctui_model::QaEffect::InspectCapability { scope: None });
        let Some(DaemonCommand::InspectQaCapability { request }) =
            daemon_command_for_effect(&app, &effect).unwrap()
        else {
            panic!("expected typed QA capability command");
        };
        assert_eq!(request.input.build_directory, "/build");
        assert_eq!(request.input.selected_recipe_name, "busybox");
        assert_eq!(request.input.recipe_names, vec!["busybox"]);
    }

    #[test]
    fn client_runtime_qa_report_maps_import_to_daemon_worker() {
        let mut app = App::new(16, 4096);
        app.workspace.build_dir = Some("/build".into());
        let request =
            yoctui_model::QaReportRequest::new(1, vec!["/tmp/report.json".into()]).unwrap();
        let effect = Effect::Qa(yoctui_model::QaEffect::ImportReports(request));
        assert!(matches!(
            daemon_command_for_effect(&app, &effect).unwrap(),
            Some(DaemonCommand::StartQaReportScan { generation: 1, build_directory, .. }) if build_directory == "/build"
        ));
    }

    #[test]
    fn client_runtime_jobs_routes_maintenance_to_daemon() {
        let mut app = App::new(16, 4096);
        app.workspace.build_dir = Some("/build".into());
        let effect =
            Effect::Maintenance(yoctui_model::MaintenanceEffect::InspectCapability { request: 1 });
        assert!(matches!(
            daemon_command_for_effect(&app, &effect).unwrap(),
            Some(DaemonCommand::InspectMaintenanceCapability { request: 1, .. })
        ));
    }

    #[test]
    fn standalone_mode_remains_an_explicit_local_fallback() {
        assert!(
            "Daemon unavailable; interactive runtime is local".starts_with("Daemon unavailable")
        );
    }

    #[test]
    fn ux_terminal_runtime_prefix_maps_create_and_writer_commands() {
        let mut app = App::new(16, 4096);
        app.workspace.build_dir = Some("/build".into());
        let Some(DaemonCommand::CreatePty { cwd, .. }) =
            prefix_daemon_command(&app, PrefixCommand::CreateSession).unwrap()
        else {
            panic!("expected typed PTY create command");
        };
        assert_eq!(cwd, "/build");
        app.daemon
            .pty_sessions
            .push(yoctui_model::ClientDaemonPtySummary {
                id: 3,
                name: "shell".into(),
                lifecycle: ClientDaemonLifecycle::Running,
                viewers: 1,
            });
        app.daemon
            .pty_details
            .push(yoctui_model::ClientDaemonPtyDetails {
                id: 3,
                kind: yoctui_model::ClientDaemonPtyKind::BuildShell,
                cwd: "/build".into(),
                columns: 120,
                rows: 40,
                writer: None,
                writer_epoch: 7,
                exit_code: None,
                restartable: true,
            });
        assert!(matches!(
            prefix_daemon_command(&app, PrefixCommand::TakeControl).unwrap(),
            Some(DaemonCommand::TakePtyControl { session_id, expected_epoch: 7 })
                if session_id.0 == 3
        ));
        assert!(
            prefix_daemon_command(&app, PrefixCommand::SplitHorizontal)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn client_runtime_devtool_maps_every_effect_to_closed_wire_type() {
        let mut app = App::new(16, 4096);
        app.workspace.build_dir = Some("/build".into());
        let identity = yoctui_model::RecipeIdentity {
            name: "busybox".into(),
            file: "/layers/busybox.bb".into(),
        };
        let cases = [
            (
                Effect::DevtoolModify(identity.clone()),
                DaemonDevtoolOperation::Modify {
                    recipe: "busybox".into(),
                },
            ),
            (
                Effect::DevtoolUpdateRecipe(identity.clone()),
                DaemonDevtoolOperation::UpdateRecipe {
                    recipe: "busybox".into(),
                },
            ),
            (
                Effect::DevtoolReset(yoctui_model::DevtoolResetPlan {
                    identity: identity.clone(),
                    source_path: "/workspace/busybox".into(),
                }),
                DaemonDevtoolOperation::Reset {
                    recipe: "busybox".into(),
                },
            ),
            (
                Effect::DevtoolFinish(yoctui_model::DevtoolFinishPlan {
                    identity: identity.clone(),
                    layer: yoctui_model::Layer {
                        name: "meta-test".into(),
                        path: "/layers/meta-test".into(),
                        priority: Some(7),
                    },
                }),
                DaemonDevtoolOperation::Finish {
                    recipe: "busybox".into(),
                    destination: "/layers/meta-test".into(),
                },
            ),
            (
                Effect::DevtoolDeploy(yoctui_model::DevtoolDeployPlan {
                    identity,
                    target: "root@example".into(),
                }),
                DaemonDevtoolOperation::DeployTarget {
                    recipe: "busybox".into(),
                    target: "root@example".into(),
                },
            ),
        ];
        for (effect, expected) in cases {
            assert_eq!(
                daemon_command_for_effect(&app, &effect).unwrap(),
                Some(DaemonCommand::StartDevtool {
                    operation: expected,
                    build_directory: "/build".into(),
                })
            );
        }
        assert_eq!(
            daemon_command_for_effect(&app, &Effect::PersistSettings).unwrap(),
            None
        );
    }
}
