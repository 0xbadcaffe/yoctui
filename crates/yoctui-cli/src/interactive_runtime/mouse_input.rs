use super::*;

impl InteractiveRuntime {
    pub(super) async fn handle_mouse(&mut self, mouse: crossterm::event::MouseEvent) -> Result<()> {
        let runtime = self;
        let kind = mouse_kind_from_event(mouse.kind);
        let terminal_size = runtime.terminal.size()?;
        if let Some(kind) = kind
            && let Some(action) = mouse_action_for_app(
                MouseInput {
                    kind,
                    column: mouse.column,
                    row: mouse.row,
                },
                &runtime.app,
                terminal_size.width,
                terminal_size.height,
            )
        {
            match compatibility_workspace_action(&mut runtime.app, action) {
                Some(Effect::InspectKernel) => {
                    runtime.begin_platform_inspection(
                        platform_inspection_operation::PlatformInspectionRequest::Kernel,
                    );
                }
                Some(Effect::InspectFirmware) => {
                    runtime.begin_platform_inspection(
                        platform_inspection_operation::PlatformInspectionRequest::Firmware,
                    );
                }
                Some(effect @ (Effect::GetPackageInventory(_) | Effect::GetPackageDetail(_))) => {
                    begin_package_operation(
                        &mut runtime.app,
                        &runtime.package_adapter,
                        &mut runtime.package_operation,
                        effect,
                    );
                }
                Some(effect @ Effect::GetImageArtifacts(_)) => {
                    begin_image_artifact_operation(
                        &mut runtime.app,
                        runtime.image_artifact_adapter.as_ref(),
                        &mut runtime.image_artifact_operation,
                        effect,
                    );
                }
                Some(effect @ Effect::GetRootfsComposition(_)) => {
                    begin_rootfs_composition_operation(
                        runtime.backend.as_mut(),
                        &mut runtime.app,
                        &runtime.session_build_dir,
                        &mut runtime.rootfs_composition_operation,
                        effect,
                        runtime.daemon_attached,
                    )
                    .await;
                }
                Some(Effect::LoadLayerBrowserDirectory {
                    layer,
                    root,
                    directory,
                }) => {
                    load_layer_browser_directory(&mut runtime.app, layer, root, directory).await;
                }
                Some(Effect::LoadLayerBrowserPreview(path)) => {
                    load_layer_browser_preview(&mut runtime.app, path).await;
                }
                Some(Effect::OpenInEditor(path)) => {
                    open_in_editor(
                        &runtime.guard,
                        &mut runtime.app,
                        path,
                        runtime.editor_command.as_deref(),
                    )
                    .await;
                }
                Some(effect @ Effect::InspectSdkTools) => {
                    begin_sdk_capability_operation(
                        &mut runtime.app,
                        runtime.sdk_tool_adapter.as_ref(),
                        &mut runtime.sdk_capability_operation,
                        effect,
                    );
                }
                _ => {}
            }
        }
        Ok(())
    }
}
