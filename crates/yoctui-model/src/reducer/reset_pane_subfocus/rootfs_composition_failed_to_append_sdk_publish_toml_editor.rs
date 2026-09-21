use super::*;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match action {
        Action::RootfsCompositionFailed { request, message } => {
            if matches!(
                &app.rootfs_composition,
                RootfsCompositionState::Loading { request: pending } if pending == &request
            ) {
                app.rootfs_composition = RootfsCompositionState::Failed {
                    request,
                    message: message.clone(),
                };
                app.notification = Some(format!("Rootfs composition failed: {message}"));
            }
        }
        Action::SelectRootfsGroup { delta } => {
            let rows = rootfs_group_rows(app);
            if rows.is_empty() {
                app.rootfs_group_selection = None;
                app.rootfs_package_selection = None;
                return None;
            }
            let current = app
                .rootfs_group_selection
                .as_ref()
                .and_then(|identity| rows.iter().position(|row| &row.identity == identity))
                .unwrap_or(0);
            let next = shifted_index(current, delta, rows.len());
            app.rootfs_group_selection = Some(rows[next].identity.clone());
            app.rootfs_package_selection = rows[next].members.first().cloned();
        }
        Action::SelectRootfsPackage { delta } => {
            let rows = rootfs_group_rows(app);
            let Some(row) = app
                .rootfs_group_selection
                .as_ref()
                .and_then(|identity| rows.iter().find(|row| &row.identity == identity))
            else {
                app.rootfs_package_selection = None;
                return None;
            };
            let current = app
                .rootfs_package_selection
                .as_ref()
                .and_then(|identity| row.members.iter().position(|member| member == identity))
                .unwrap_or(0);
            if !row.members.is_empty() {
                app.rootfs_package_selection =
                    Some(row.members[shifted_index(current, delta, row.members.len())].clone());
            }
        }
        Action::SelectRootfsEntry { delta } => {
            let entries = app
                .rootfs_composition
                .composition()
                .and_then(RootfsComposition::filesystem_tree)
                .map(|tree| tree.entries.as_slice())
                .unwrap_or_default();
            if entries.is_empty() {
                app.rootfs_entry_selection = None;
                return None;
            }
            let current = app
                .rootfs_entry_selection
                .as_ref()
                .and_then(|identity| entries.iter().position(|entry| &entry.identity == identity))
                .unwrap_or(0);
            app.rootfs_entry_selection = Some(
                entries[shifted_index(current, delta, entries.len())]
                    .identity
                    .clone(),
            );
        }
        Action::SelectRootfsSystemdService { delta } => {
            let len = app
                .rootfs_composition
                .composition()
                .and_then(RootfsComposition::system_inventory)
                .map_or(0, |inventory| inventory.systemd_services.len());
            app.rootfs_systemd_selection = shifted_index(app.rootfs_systemd_selection, delta, len);
        }
        Action::SelectRootfsDbusService { delta } => {
            let len = app
                .rootfs_composition
                .composition()
                .and_then(RootfsComposition::system_inventory)
                .map_or(0, |inventory| inventory.dbus_services.len());
            app.rootfs_dbus_selection = shifted_index(app.rootfs_dbus_selection, delta, len);
        }
        Action::SelectRootfsUdevRule { delta } => {
            let len = app
                .rootfs_composition
                .composition()
                .and_then(RootfsComposition::system_inventory)
                .map_or(0, |inventory| inventory.udev_rules.len());
            app.rootfs_udev_selection = shifted_index(app.rootfs_udev_selection, delta, len);
            app.rootfs_udev_preview_offset = 0;
        }
        Action::ScrollRootfsUdevPreview { delta } => {
            let len = app
                .rootfs_composition
                .composition()
                .and_then(RootfsComposition::system_inventory)
                .and_then(|inventory| inventory.udev_rules.get(app.rootfs_udev_selection))
                .map_or(0, |rule| rule.preview.lines().count());
            app.rootfs_udev_preview_offset =
                shifted_index(app.rootfs_udev_preview_offset, delta, len);
        }
        Action::BrowseRootfsFilesystem => {
            if let Some((root, image)) =
                app.rootfs_composition
                    .composition()
                    .and_then(|composition| {
                        composition
                            .root_directory
                            .clone()
                            .map(|root| (root, composition.image.image.clone()))
                    })
            {
                return Some(Effect::LoadLayerBrowserDirectory {
                    layer: format!("Rootfs: {image}"),
                    root: root.clone(),
                    directory: root,
                });
            }
            app.notification =
                Some("The selected image has no available IMAGE_ROOTFS tree.".into());
        }
        Action::EditSelectedRootfsSystemFile => {
            let composition = app.rootfs_composition.composition();
            let root = composition.and_then(|composition| composition.root_directory.clone());
            let path = match app.images_view {
                ImagesView::SystemdServices => composition
                    .and_then(RootfsComposition::system_inventory)
                    .and_then(|inventory| {
                        inventory.systemd_services.get(app.rootfs_systemd_selection)
                    })
                    .map(|service| service.host_path.clone()),
                ImagesView::SystemDbus => composition
                    .and_then(RootfsComposition::system_inventory)
                    .and_then(|inventory| inventory.dbus_services.get(app.rootfs_dbus_selection))
                    .map(|service| service.host_path.clone()),
                _ => None,
            };
            if let (Some(root), Some(path)) = (root, path)
                && let Ok(file) = path.strip_prefix(&root)
            {
                return Some(Effect::OpenLayerBrowserEditor {
                    layer: "Rootfs system".into(),
                    root,
                    file: file.to_path_buf(),
                });
            }
            app.notification = Some("No editable rootfs system file is selected.".into());
        }
        Action::BeginSdkBuild(action) => {
            let Some(image) = app.build.target.clone() else {
                app.notification = Some("Select an SDK image target with i first.".into());
                return None;
            };
            let machine = app
                .workspace
                .variables
                .get("MACHINE")
                .cloned()
                .unwrap_or_default();
            let distro = app
                .workspace
                .variables
                .get("DISTRO")
                .cloned()
                .unwrap_or_default();
            match SdkBuildPreview::new(machine, distro, image, action) {
                Ok(preview) => open_dialog(app, Dialog::SdkBuildConfirmation(preview)),
                Err(message) => app.notification = Some(message.into()),
            }
        }
        Action::ConfirmSdkBuild => {
            let Some(Dialog::SdkBuildConfirmation(preview)) = app.active_dialog().cloned() else {
                return None;
            };
            let current = SdkBuildPreview::new(
                app.workspace
                    .variables
                    .get("MACHINE")
                    .cloned()
                    .unwrap_or_default(),
                app.workspace
                    .variables
                    .get("DISTRO")
                    .cloned()
                    .unwrap_or_default(),
                app.build.target.clone().unwrap_or_default(),
                preview.action,
            );
            if current.as_ref() != Ok(&preview) {
                app.notification = Some("The SDK build preview is stale; review it again.".into());
                return None;
            }
            close_dialog(app);
            return Some(Effect::Start(preview.request));
        }
        Action::CancelSdkBuild => {
            if matches!(app.active_dialog(), Some(Dialog::SdkBuildConfirmation(_))) {
                close_dialog(app);
            }
        }
        Action::BeginSdkArtifactInventory | Action::RefreshSdkArtifactInventory => {
            if matches!(app.sdk_artifacts, SdkArtifactInventoryState::Loading { .. }) {
                app.notification = Some("An SDK artifact scan is already running.".into());
                return None;
            }
            return begin_sdk_artifact_inventory(app);
        }
        Action::SdkArtifactInventoryLoaded {
            request,
            artifacts,
            limitations,
        } => {
            if !matches!(
                &app.sdk_artifacts,
                SdkArtifactInventoryState::Loading { request: pending } if pending == &request
            ) {
                note_stale_sdk_event(app);
                return None;
            }
            let previous = app.sdk_artifact_selection.take();
            match normalize_sdk_artifacts(&request, artifacts) {
                Ok(artifacts) => {
                    let limitations = normalize_sdk_limitations(limitations);
                    app.sdk_artifacts = if artifacts.is_empty() && limitations.is_empty() {
                        SdkArtifactInventoryState::AvailableEmpty { request }
                    } else if limitations.is_empty() {
                        SdkArtifactInventoryState::Available { request, artifacts }
                    } else {
                        SdkArtifactInventoryState::Partial {
                            request,
                            artifacts,
                            limitations,
                        }
                    };
                    set_sdk_artifact_selection_to_current_or_first(app, previous);
                }
                Err(message) => {
                    app.sdk_artifacts = SdkArtifactInventoryState::Failed {
                        request,
                        message: message.into(),
                    };
                    app.notification =
                        Some("SDK artifact inventory failed model validation.".into());
                }
            }
        }
        Action::SdkArtifactInventoryFailed { request, message } => {
            if !matches!(
                &app.sdk_artifacts,
                SdkArtifactInventoryState::Loading { request: pending } if pending == &request
            ) {
                note_stale_sdk_event(app);
                return None;
            }
            app.sdk_artifacts = SdkArtifactInventoryState::Failed { request, message };
            app.sdk_artifact_selection = None;
        }
        Action::SelectSdkArtifact { delta } => {
            let visible = app
                .filtered_sdk_artifacts()
                .into_iter()
                .map(|artifact| artifact.identity.clone())
                .collect::<Vec<_>>();
            if visible.is_empty() {
                app.sdk_artifact_selection = None;
                return None;
            }
            let current = app
                .sdk_artifact_selection
                .as_ref()
                .and_then(|identity| visible.iter().position(|candidate| candidate == identity))
                .unwrap_or(0);
            let next = shifted_index(current, delta, visible.len());
            app.sdk_artifact_selection = Some(visible[next].clone());
        }
        Action::BeginSdkArtifactSearch => app.sdk_artifact_searching = true,
        Action::AppendSdkArtifactQuery(character) => {
            if app.sdk_artifact_searching
                && !character.is_control()
                && app.sdk_artifact_query.len() < 256
            {
                app.sdk_artifact_query.push(character);
                set_sdk_artifact_selection_to_current_or_first(
                    app,
                    app.sdk_artifact_selection.clone(),
                );
            }
        }
        Action::BackspaceSdkArtifactQuery => {
            if app.sdk_artifact_searching {
                app.sdk_artifact_query.pop();
                set_sdk_artifact_selection_to_current_or_first(
                    app,
                    app.sdk_artifact_selection.clone(),
                );
            }
        }
        Action::ClearSdkArtifactQuery => {
            app.sdk_artifact_query.clear();
            set_sdk_artifact_selection_to_current_or_first(app, app.sdk_artifact_selection.clone());
        }
        Action::FinishSdkArtifactSearch => app.sdk_artifact_searching = false,
        Action::OpenSelectedSdkArtifact => {
            if let Some(path) = app
                .selected_sdk_artifact()
                .map(|artifact| artifact.identity.path.clone())
            {
                return Some(Effect::OpenInEditor(path));
            }
            app.notification = Some("No SDK artifact is selected.".into());
        }
        Action::SdkToolCapabilityLoaded(capability) => app.sdk_tool_capability = capability,
        Action::BeginSelectedSdkPublish => {
            let Some(artifact) = app.selected_sdk_artifact() else {
                app.notification = Some("Select an SDK artifact to publish.".into());
                return None;
            };
            if artifact.kind != SdkArtifactKind::Installer {
                app.notification = Some("Only an SDK installer can be published.".into());
                return None;
            }
            if let Err(message) = app.sdk_tool_capability.publish_executable() {
                app.notification = Some(message.into());
                return None;
            }
            let mut editor = PopupEditor::new(popup_toml_document("destination", "", None));
            let _ = editor.select_toml_value("destination");
            open_dialog(app, Dialog::SdkPublishTomlEditor(editor));
        }
        Action::ToggleSdkPublishTomlEditor => {
            if let Some(Dialog::SdkPublishTomlEditor(editor)) = app.active_dialog_mut() {
                editor.editing = !editor.editing;
            }
        }
        Action::AppendSdkPublishTomlEditor(character) => {
            if let Some(Dialog::SdkPublishTomlEditor(editor)) = app.active_dialog_mut()
                && editor.editing
                && !character.is_control()
                && editor.text.len() < 4_096
            {
                editor.insert(&character.to_string());
            }
        }
        _ => unreachable!("action routed to the wrong reducer"),
    }
    synchronize_focus(app);
    None
}
