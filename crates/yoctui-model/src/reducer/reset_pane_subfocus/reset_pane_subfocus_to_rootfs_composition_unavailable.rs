use super::*;
use crate::image_updates::rootfs_image_identity;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match action {
        Action::ResetPaneSubfocus => match app.focus {
            FocusTarget::Workspace => app.workspace_subfocus = WorkspaceSubfocus::Main,
            FocusTarget::Inspector => app.inspector_subfocus = InspectorSubfocus::Facts,
            FocusTarget::Navigator | FocusTarget::Dialog | FocusTarget::CommandPalette => {}
        },
        Action::TogglePaneZoom if is_pane_focus(app.focus) => {
            app.zoomed_pane = if app.zoomed_pane == Some(app.focus) {
                None
            } else {
                Some(app.focus)
            };
        }
        Action::TogglePaneZoom => {}
        Action::OpenBuildOptions => {
            if app.build_environment.connected() {
                open_dialog(app, Dialog::BuildOptions);
            } else {
                app.notification = Some("Configure and verify a BitBake environment first".into());
            }
        }
        Action::CloseBuildOptions => {
            if matches!(app.active_dialog(), Some(Dialog::BuildOptions)) {
                close_dialog(app);
            }
        }
        Action::OpenImagePicker(mut images) => {
            images.sort();
            images.dedup();
            let selection = app
                .build
                .target
                .as_ref()
                .and_then(|target| images.iter().position(|image| image == target))
                .unwrap_or(0);
            if images.is_empty() {
                app.notification =
                    Some("No image recipes were discovered in the active layers.".into());
            } else {
                open_dialog(app, Dialog::ImagePicker(ImagePicker { images, selection }));
            }
        }
        Action::SelectImage { delta } => {
            if let Some(Dialog::ImagePicker(picker)) = app.active_dialog_mut() {
                picker.selection = if delta.is_negative() {
                    picker.selection.saturating_sub(delta.unsigned_abs())
                } else {
                    picker
                        .selection
                        .saturating_add(delta as usize)
                        .min(picker.images.len().saturating_sub(1))
                };
            }
        }
        Action::ConfirmImagePicker => {
            if let Some(Dialog::ImagePicker(picker)) = app.active_dialog() {
                let image = picker.images.get(picker.selection).cloned();
                if let Some(image) = image {
                    app.build.target = Some(image);
                    close_dialog(app);
                }
            }
        }
        Action::CancelImagePicker => {
            if matches!(app.active_dialog(), Some(Dialog::ImagePicker(_))) {
                close_dialog(app);
            }
        }
        Action::BeginCurrentImageBuild => {
            if matches!(
                app.active_dialog(),
                Some(Dialog::RecipeEditor(editor)) if editor.is_dirty()
            ) {
                app.notification = Some("Save the edited file with Ctrl+S before building.".into());
            } else if let Some(target) = app.build.target.clone() {
                replace_dialog(
                    app,
                    Dialog::RecipeTaskConfirmation(BuildRequest {
                        targets: vec![target],
                        task: None,
                        force: false,
                    }),
                );
            } else {
                app.notification = Some("Select an image first with i.".into());
            }
        }
        Action::BeginImageArtifactInventory | Action::RefreshImageArtifactInventory => {
            if image_artifact_operation_is_loading(app) {
                app.notification = Some("An image artifact operation is already running.".into());
                return None;
            }
            app.qemu_capability = QemuCapability::NotInspected;
            return begin_image_artifact_inventory(app);
        }
        Action::CancelImageArtifactOperation => {
            if image_artifact_operation_is_loading(app) {
                return Some(Effect::CancelImageArtifactOperation);
            }
            app.notification = Some("No image artifact operation is running.".into());
        }
        Action::ImageArtifactInventoryLoaded { request, inventory } => {
            if !matches!(
                &app.image_artifacts,
                ImageArtifactInventoryState::Loading { request: pending } if pending == &request
            ) {
                return None;
            }
            set_image_artifact_inventory(app, request, inventory, None);
        }
        Action::ImageArtifactInventoryPartial {
            request,
            inventory,
            limitations,
        } => {
            if !matches!(
                &app.image_artifacts,
                ImageArtifactInventoryState::Loading { request: pending } if pending == &request
            ) {
                return None;
            }
            set_image_artifact_inventory(app, request, inventory, Some(limitations));
        }
        Action::ImageArtifactInventoryFailed { request, message } => {
            if !matches!(
                &app.image_artifacts,
                ImageArtifactInventoryState::Loading { request: pending } if pending == &request
            ) {
                return None;
            }
            app.image_artifacts = ImageArtifactInventoryState::Failed {
                request,
                message: message.clone(),
            };
            app.notification = Some(format!(
                "Image artifact inventory is unavailable: {message}"
            ));
        }
        Action::SelectImageArtifact { delta } => {
            let visible = app
                .filtered_image_artifacts()
                .into_iter()
                .map(|artifact| artifact.identity.clone())
                .collect::<Vec<_>>();
            if visible.is_empty() {
                app.image_artifact_selection = None;
                return None;
            }
            let current = app
                .image_artifact_selection
                .as_ref()
                .and_then(|identity| visible.iter().position(|candidate| candidate == identity))
                .unwrap_or(0);
            let next = shifted_index(current, delta, visible.len());
            app.image_artifact_selection = Some(visible[next].clone());
        }
        Action::BeginImageArtifactSearch => app.image_artifact_searching = true,
        Action::AppendImageArtifactQuery(character) => {
            if app.image_artifact_searching
                && !character.is_control()
                && app.image_artifact_query.len() < 256
            {
                app.image_artifact_query.push(character);
                set_image_artifact_selection_to_current_or_first(
                    app,
                    app.image_artifact_selection.clone(),
                );
            }
        }
        Action::BackspaceImageArtifactQuery => {
            if app.image_artifact_searching {
                app.image_artifact_query.pop();
                set_image_artifact_selection_to_current_or_first(
                    app,
                    app.image_artifact_selection.clone(),
                );
            }
        }
        Action::ClearImageArtifactQuery => {
            app.image_artifact_query.clear();
            set_image_artifact_selection_to_current_or_first(
                app,
                app.image_artifact_selection.clone(),
            );
        }
        Action::FinishImageArtifactSearch => app.image_artifact_searching = false,
        Action::BeginSelectedImageArtifactBuild => {
            if let Some(target) = app
                .selected_image_artifact()
                .map(|artifact| artifact.identity.image.clone())
            {
                if app
                    .workspace
                    .recipes
                    .iter()
                    .any(|recipe| recipe.name == target)
                {
                    app.build.target = Some(target.clone());
                    open_dialog(
                        app,
                        Dialog::RecipeTaskConfirmation(BuildRequest {
                            targets: vec![target],
                            task: None,
                            force: false,
                        }),
                    );
                } else {
                    app.notification = Some(format!(
                        "{} is a deployed artifact, not a buildable image recipe. Select an image recipe with i.",
                        target
                    ));
                }
            } else if let Some(target) = app.build.target.clone() {
                open_dialog(
                    app,
                    Dialog::RecipeTaskConfirmation(BuildRequest {
                        targets: vec![target],
                        task: None,
                        force: false,
                    }),
                );
            } else {
                app.notification = Some("Select an image first with i.".into());
            }
        }
        Action::OpenSelectedImageArtifact => {
            if let Some(path) = app
                .selected_image_artifact()
                .map(|artifact| artifact.identity.path.clone())
            {
                return Some(Effect::OpenInEditor(path));
            }
            app.notification = Some("No deployed image artifact is selected.".into());
        }
        Action::OpenSelectedImageArtifactAssociation(association) => {
            if let Some(path) = app
                .selected_image_artifact()
                .and_then(|artifact| artifact.associated_paths(association))
                .and_then(|paths| paths.first())
                .cloned()
            {
                return Some(Effect::OpenInEditor(path));
            }
            let label = match association {
                ImageArtifactAssociation::Manifest => "manifest",
                ImageArtifactAssociation::License => "license",
                ImageArtifactAssociation::Spdx => "SPDX/SBOM",
                ImageArtifactAssociation::Wic => "Wic",
            };
            app.notification = Some(format!(
                "The selected image artifact has no authoritative {label} path."
            ));
        }
        Action::BeginSelectedRootfsComposition => {
            let Some(image) = rootfs_image_identity(app) else {
                app.notification =
                    Some("Select a deployed artifact owned by an image recipe first.".into());
                return None;
            };
            app.image_artifact_selection = Some(image.clone());
            app.images_view = ImagesView::RootfsPackages;
            return begin_rootfs_composition(app, image);
        }
        Action::ShiftImagesView { delta } => {
            app.images_view = app.images_view.shifted(delta);
            app.image_artifact_searching = false;
            if app.images_view != ImagesView::Artifacts {
                let Some(image) = rootfs_image_identity(app) else {
                    app.notification = Some(
                        "Select a deployed artifact owned by an image recipe before opening rootfs composition."
                            .into(),
                    );
                    app.images_view = ImagesView::Artifacts;
                    return None;
                };
                app.image_artifact_selection = Some(image.clone());
                let is_current = app
                    .rootfs_composition
                    .request()
                    .is_some_and(|request| request.image == image);
                if !is_current {
                    return begin_rootfs_composition(app, image);
                }
            }
        }
        Action::RefreshRootfsComposition => {
            let image = app
                .rootfs_composition
                .request()
                .map(|request| request.image.clone())
                .or_else(|| {
                    app.selected_image_artifact()
                        .map(|artifact| artifact.identity.clone())
                });
            let Some(image) = image else {
                app.notification =
                    Some("No rootfs composition image is available to refresh.".into());
                return None;
            };
            return begin_rootfs_composition(app, image);
        }
        Action::RootfsCompositionLoaded {
            request,
            composition,
        } => {
            if matches!(
                &app.rootfs_composition,
                RootfsCompositionState::Loading { request: pending } if pending == &request
            ) {
                set_rootfs_composition(app, request, composition, Vec::new());
            }
        }
        Action::RootfsCompositionPartial {
            request,
            composition,
            limitations,
        } => {
            if matches!(
                &app.rootfs_composition,
                RootfsCompositionState::Loading { request: pending } if pending == &request
            ) {
                set_rootfs_composition(app, request, composition, limitations);
            }
        }
        Action::RootfsCompositionUnavailable { request, reason } => {
            if matches!(
                &app.rootfs_composition,
                RootfsCompositionState::Loading { request: pending } if pending == &request
            ) {
                app.rootfs_composition = RootfsCompositionState::Unavailable { request, reason };
            }
        }
        _ => unreachable!("action routed to the wrong reducer"),
    }
    synchronize_focus(app);
    None
}
