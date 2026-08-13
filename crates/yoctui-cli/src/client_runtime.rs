use std::{fs::File, io::Read, time::Duration};

use thiserror::Error;
use yoctui_app::DaemonClientSnapshot;
use yoctui_model::{App, ClientDaemonLifecycle, Effect};
use yoctui_protocol::daemon::{
    ClientId, CommandRequest, DaemonCommand, DaemonDevtoolOperation, DaemonQemuRequest,
    DaemonSdkArtifactIdentity, DaemonSdkContext, DaemonSdkNativeMode, DaemonSdkOperation,
    DaemonWicCreateRequest, JobId, RequestId, Subscription,
};

use crate::client_transport::{ClientServerEvent, ClientTransportError, DaemonClientTransport};

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
        let mut transport =
            DaemonClientTransport::connect(random_client_id()?, "yoctui-ratatui".into(), timeout)?;
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
        Ok(Self {
            transport,
            replica,
            next_request: 1,
        })
    }

    pub fn poll(&mut self, app: &mut App) -> Result<bool, ClientRuntimeError> {
        let Some(event) = self.transport.try_receive(Duration::from_millis(1))? else {
            return Ok(false);
        };
        match event {
            ClientServerEvent::Snapshot(snapshot) => self.replica.replace_app(app, snapshot),
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
        Ok(true)
    }

    pub fn route_effect(
        &mut self,
        app: &App,
        effect: &Effect,
    ) -> Result<RuntimeEffectRoute, ClientRuntimeError> {
        let Some(command) = daemon_command_for_effect(app, effect)? else {
            return Ok(RuntimeEffectRoute::ClientLocal);
        };
        let request_id = RequestId(self.next_request);
        self.next_request = self
            .next_request
            .checked_add(1)
            .ok_or(ClientRuntimeError::RequestSpaceExhausted)?;
        self.transport.command(CommandRequest {
            request_id,
            expected_generation: Some(app.daemon.generation),
            command,
        })?;
        Ok(RuntimeEffectRoute::Daemon(request_id))
    }

    pub fn detach(mut self, app: &mut App) -> Result<(), ClientRuntimeError> {
        self.transport.detach()?;
        self.replica.disconnect_app(app);
        Ok(())
    }
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
        _ => return Ok(None),
    }))
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
    fn client_runtime_random_identity_is_nonzero() {
        assert_ne!(random_client_id().unwrap().0, [0; 16]);
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
