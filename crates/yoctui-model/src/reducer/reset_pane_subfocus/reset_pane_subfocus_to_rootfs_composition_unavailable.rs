use super::*;

fn image_artifact_view_effect(app: &App, path: PathBuf) -> Option<Effect> {
    let artifact = app.selected_image_artifact()?;
    let known = artifact.identity.path == path
        || [
            ImageArtifactAssociation::Manifest,
            ImageArtifactAssociation::License,
            ImageArtifactAssociation::Spdx,
            ImageArtifactAssociation::Wic,
        ]
        .into_iter()
        .any(|association| {
            artifact
                .associated_paths(association)
                .is_some_and(|paths| paths.contains(&path))
        });
    let root = app
        .image_artifacts
        .inventory()
        .and_then(|inventory| inventory.deploy_directory.available())
        .cloned();
    if known
        && let Some(root) = root
        && yoctui_utils::is_absolute_normal_path(&path)
        && path.starts_with(&root)
        && path != root
    {
        return Some(Effect::ViewImageArtifact { root, path });
    }
    None
}

fn open_image_picker(app: &mut App, images: &mut Vec<String>) {
    images.sort();
    images.dedup();
    let selection = app
        .build
        .target
        .as_ref()
        .and_then(|target| images.iter().position(|image| image == target))
        .unwrap_or(0);
    if images.is_empty() {
        app.notification = Some("No image recipes were discovered in the active layers.".into());
    } else {
        open_dialog(
            app,
            Dialog::ImagePicker(ImagePicker {
                images: std::mem::take(images),
                selection,
            }),
        );
    }
}

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
                if app.build.target.is_some() {
                    open_dialog(app, Dialog::BuildOptions);
                } else {
                    let images = app
                        .workspace
                        .recipes
                        .iter()
                        .map(|recipe| recipe.name.as_str())
                        .filter(|name| name.contains("image"))
                        .map(str::to_owned)
                        .collect();
                    return update(app, Action::OpenImageBuildPicker(images));
                }
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
            app.pending_image_build_options = false;
            app.resume_build_options_on_image_picker_cancel = false;
            open_image_picker(app, &mut images);
        }
        Action::OpenImageBuildPicker(mut images) => {
            let resume_on_cancel = matches!(app.active_dialog(), Some(Dialog::BuildOptions));
            if resume_on_cancel {
                close_dialog(app);
            }
            app.pending_image_build_options = true;
            app.resume_build_options_on_image_picker_cancel = resume_on_cancel;
            open_image_picker(app, &mut images);
            if !matches!(app.active_dialog(), Some(Dialog::ImagePicker(_))) {
                app.pending_image_build_options = false;
                app.resume_build_options_on_image_picker_cancel = false;
                if resume_on_cancel {
                    open_dialog(app, Dialog::BuildOptions);
                }
            }
        }
        Action::OpenRecipePicker(purpose) => {
            let mut recipes = app
                .workspace
                .recipes
                .iter()
                .filter_map(|recipe| {
                    let file = recipe.file.clone()?;
                    file.is_absolute().then(|| RecipeIdentity {
                        name: recipe.name.clone(),
                        file,
                    })
                })
                .collect::<Vec<_>>();
            recipes
                .sort_by(|left, right| left.name.cmp(&right.name).then(left.file.cmp(&right.file)));
            recipes.dedup();
            let selected = selected_recipe_identity(app).ok();
            let selection = selected
                .as_ref()
                .and_then(|identity| recipes.iter().position(|recipe| recipe == identity))
                .unwrap_or(0);
            if recipes.is_empty() {
                app.notification =
                    Some("No recipes with authoritative provider paths were discovered.".into());
            } else {
                open_dialog(
                    app,
                    Dialog::RecipePicker(RecipePicker {
                        recipes,
                        selection,
                        purpose,
                    }),
                );
            }
        }
        Action::SelectRecipePicker { delta } => {
            if let Some(Dialog::RecipePicker(picker)) = app.active_dialog_mut() {
                picker.selection = shifted_index(picker.selection, delta, picker.recipes.len());
            }
        }
        Action::ConfirmRecipePicker => {
            let selected = match app.active_dialog() {
                Some(Dialog::RecipePicker(picker)) => picker
                    .recipes
                    .get(picker.selection)
                    .cloned()
                    .map(|identity| (identity, picker.purpose)),
                _ => None,
            };
            if let Some((identity, purpose)) = selected {
                if let Some(index) = app.workspace.recipes.iter().position(|recipe| {
                    recipe.name == identity.name && recipe.file.as_ref() == Some(&identity.file)
                }) {
                    app.recipe_selection = index;
                }
                close_dialog(app);
                return match purpose {
                    RecipePickerPurpose::Build => {
                        begin_recipe_task(app, None, false);
                        None
                    }
                    RecipePickerPurpose::Dependencies => {
                        app.screen = Screen::Dependencies;
                        update(app, Action::BeginSelectedRecipeDependencies)
                    }
                };
            }
        }
        Action::CancelRecipePicker => {
            if matches!(app.active_dialog(), Some(Dialog::RecipePicker(_))) {
                close_dialog(app);
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
                    if std::mem::take(&mut app.pending_image_build_options) {
                        app.resume_build_options_on_image_picker_cancel = false;
                        open_dialog(app, Dialog::BuildOptions);
                    }
                }
            }
        }
        Action::CancelImagePicker => {
            if matches!(app.active_dialog(), Some(Dialog::ImagePicker(_))) {
                close_dialog(app);
                app.pending_image_build_options = false;
                if std::mem::take(&mut app.resume_build_options_on_image_picker_cancel) {
                    open_dialog(app, Dialog::BuildOptions);
                }
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
                let images = app
                    .workspace
                    .recipes
                    .iter()
                    .map(|recipe| recipe.name.as_str())
                    .filter(|name| name.contains("image"))
                    .map(str::to_owned)
                    .collect();
                return update(app, Action::OpenImageBuildPicker(images));
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
                let effect = image_artifact_view_effect(app, path);
                if effect.is_none() {
                    app.notification =
                        Some("The artifact has no authoritative contained deploy path.".into());
                }
                return effect;
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
                let effect = image_artifact_view_effect(app, path);
                if effect.is_none() {
                    app.notification =
                        Some("The artifact has no authoritative contained deploy path.".into());
                }
                return effect;
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
        Action::ImageArtifactViewed { root, path, result } => {
            if image_artifact_view_effect(app, path.clone())
                != Some(Effect::ViewImageArtifact {
                    root: root.clone(),
                    path: path.clone(),
                })
                || app.screen != Screen::Images
                || app.active_dialog().is_some()
            {
                return None;
            }
            match result {
                Ok(ImageArtifactView::Text(content)) => {
                    let relative = path.strip_prefix(&root).ok()?.to_path_buf();
                    if matches!(
                        crate::update(
                            app,
                            Action::OpenRecipeEditor {
                                recipe: "Images".into(),
                                root,
                                files: vec![relative],
                            }
                        ),
                        Some(Effect::LoadRecipeEditorFile(_))
                    ) {
                        crate::update(app, Action::LoadRecipeEditorContent(content));
                        crate::update(app, Action::FocusRecipeEditor(RecipeEditorFocus::Document));
                    }
                }
                Ok(ImageArtifactView::DeviceTree {
                    kind,
                    program,
                    size_bytes,
                }) if matches!(kind, PlatformFileKind::Dtb | PlatformFileKind::Dtbo) => {
                    open_dialog(
                        app,
                        Dialog::DtcDecompile(DtcDecompileDialog::new(
                            PlatformComponent::Images,
                            &PlatformFile {
                                path,
                                root,
                                kind,
                                size_bytes,
                            },
                            program,
                        )),
                    );
                }
                Ok(_) => app.notification = Some("Unsupported artifact view observation.".into()),
                Err(message) => app.notification = Some(message),
            }
        }
        action @ (Action::BeginSelectedRootfsComposition
        | Action::ShiftImagesView { .. }
        | Action::RefreshRootfsComposition
        | Action::RootfsCompositionPreview { .. }
        | Action::RootfsCompositionLoaded { .. }
        | Action::RootfsCompositionPartial { .. }
        | Action::RootfsCompositionUnavailable { .. }) => {
            return super::rootfs_composition::reduce_actions(app, action);
        }
        _ => unreachable!("action routed to the wrong reducer"),
    }
    synchronize_focus(app);
    None
}
