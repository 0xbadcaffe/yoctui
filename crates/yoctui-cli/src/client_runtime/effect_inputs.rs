use yoctui_model::App;
use yoctui_protocol::daemon::{
    DaemonQaCapabilityInput, DaemonQaCapabilityRequest, DaemonQemuRequest,
    DaemonSdkArtifactIdentity, DaemonSdkContext, DaemonSdkNativeMode, DaemonSdkOperation,
    DaemonWicCreateRequest, RequestId,
};

use super::ClientRuntimeError;

pub(super) fn qa_capability_request(
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

pub(super) fn qemu_executable(
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

pub(super) fn wire_qemu_request(request: &yoctui_model::QemuLaunchRequest) -> DaemonQemuRequest {
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

pub(super) fn wic_executable(app: &App) -> Result<String, ClientRuntimeError> {
    let yoctui_model::WicCapability::Available { executable, .. } = &app.wic_capability else {
        return Err(ClientRuntimeError::MissingWicCapability);
    };
    Ok(executable.display().to_string())
}

pub(super) fn wire_wic_create(request: &yoctui_model::WicCreateRequest) -> DaemonWicCreateRequest {
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

pub(super) fn wire_sdk_operation(operation: &yoctui_model::SdkOperation) -> DaemonSdkOperation {
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

pub(super) fn sdk_context(
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
