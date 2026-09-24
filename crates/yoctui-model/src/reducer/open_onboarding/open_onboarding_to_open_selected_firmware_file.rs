use super::*;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match action {
        Action::OpenOnboarding => {
            app.onboarding.open = true;
            app.onboarding.selected = app.onboarding.progress.current;
        }
        Action::DismissOnboarding => {
            app.onboarding.open = false;
            app.onboarding.progress.dismissed = true;
            synchronize_focus(app);
            return Some(Effect::PersistOnboarding);
        }
        Action::SelectOnboarding { delta } if app.onboarding.open => {
            app.onboarding.select(delta);
        }
        Action::SelectOnboarding { .. } => {}
        Action::ActivateOnboardingStep if app.onboarding.open => {
            let route = onboarding_route(app, app.onboarding.selected);
            app.onboarding.open = false;
            return update(app, route);
        }
        Action::ActivateOnboardingStep => {}
        Action::AdvanceOnboarding if app.onboarding.open => {
            let current = app.onboarding.progress.current;
            if !onboarding_completion_evidence(app, current) {
                app.notification = Some(format!(
                    "{} is not complete: {}",
                    current.title(),
                    app.onboarding_projection()
                        .rows
                        .iter()
                        .find(|row| row.step == current)
                        .map_or("its current prerequisite is not satisfied", |row| {
                            row.prerequisite.as_str()
                        })
                ));
            } else {
                app.onboarding.progress.completed.insert(current);
                app.onboarding.progress.skipped.remove(&current);
                app.onboarding.move_after_current();
                synchronize_focus(app);
                return Some(Effect::PersistOnboarding);
            }
        }
        Action::AdvanceOnboarding => {}
        Action::SkipOnboardingStep if app.onboarding.open => {
            let current = app.onboarding.progress.current;
            app.onboarding.progress.skipped.insert(current);
            app.onboarding.progress.completed.remove(&current);
            app.onboarding.move_after_current();
            synchronize_focus(app);
            return Some(Effect::PersistOnboarding);
        }
        Action::SkipOnboardingStep => {}
        Action::RestartOnboarding if app.onboarding.open => {
            app.onboarding.progress = OnboardingProgress::default();
            app.onboarding.selected = OnboardingStep::Environment;
            synchronize_focus(app);
            return Some(Effect::PersistOnboarding);
        }
        Action::RestartOnboarding => {}
        Action::OnboardingPersisted => {}
        Action::OnboardingPersistenceFailed(message) => {
            app.notification = Some(format!("Onboarding progress was not saved: {message}"));
        }
        Action::ScrollCurrent { to_end } => {
            let action = current_collection_edge_action(app, to_end)?;
            return update(app, action);
        }
        Action::ProjectProfileAbsent => {
            app.project_profile = ProjectProfileState::Absent;
        }
        Action::ProjectProfileLoaded(profile) => match profile.validate() {
            Ok(()) => app.project_profile = ProjectProfileState::Loaded(profile),
            Err(error) => app.project_profile = ProjectProfileState::Invalid(error.to_string()),
        },
        Action::ProjectProfileLoadFailed(message) => {
            app.project_profile = ProjectProfileState::Invalid(message);
        }
        Action::PreviewProjectProfileGeneration(profile) => match profile.validate() {
            Ok(()) => app.project_profile = ProjectProfileState::GenerationPreview(profile),
            Err(error) => app.notification = Some(error.to_string()),
        },
        Action::ConfirmProjectProfileGeneration { replace } => {
            let ProjectProfileState::GenerationPreview(profile) = &app.project_profile else {
                return None;
            };
            let profile = profile.clone();
            app.project_profile = ProjectProfileState::Generating(profile.clone());
            return Some(Effect::GenerateProjectProfile { profile, replace });
        }
        Action::ProjectProfileGenerated(profile) => {
            app.project_profile = ProjectProfileState::Loaded(profile);
            app.notification = Some("Project profile generated.".into());
        }
        Action::ProjectProfileGenerationFailed(message) => {
            app.notification = Some(format!("Project profile was not generated: {message}"));
            if let ProjectProfileState::Generating(profile) = &app.project_profile {
                app.project_profile = ProjectProfileState::GenerationPreview(profile.clone());
            }
        }
        Action::SelectProjectProfileItem { delta } => {
            let count =
                project_profile_items(&app.project_profile, &app.workspace, &app.available_images)
                    .len();
            app.project_profile_selection = if delta < 0 {
                app.project_profile_selection
                    .saturating_sub(delta.unsigned_abs())
            } else {
                app.project_profile_selection
                    .saturating_add(delta as usize)
                    .min(count.saturating_sub(1))
            };
        }
        Action::ActivateProjectProfileItem => {
            let items =
                project_profile_items(&app.project_profile, &app.workspace, &app.available_images);
            let item = items.get(app.project_profile_selection)?;
            if !matches!(item.status, ProjectProfileItemStatus::Resolved) {
                app.notification = Some(format!("{} is not currently resolved.", item.label));
                return None;
            }
            let ProjectProfileState::Loaded(profile) = &app.project_profile else {
                app.notification = Some("Project profile is not ready for activation.".into());
                return None;
            };
            match item.kind {
                ProjectProfileItemKind::BuildPreset(index) => {
                    let preset = &profile.build_presets[index];
                    replace_dialog(
                        app,
                        Dialog::RecipeTaskConfirmation(BuildRequest {
                            targets: preset.targets.clone(),
                            task: None,
                            force: false,
                        }),
                    );
                }
                ProjectProfileItemKind::FavoriteRecipe(index) => {
                    let identity = &profile.favorites.recipes[index];
                    app.recipe_selection = app
                        .workspace
                        .recipes
                        .iter()
                        .position(|recipe| &recipe.name == identity)
                        .unwrap_or(0);
                    app.screen = Screen::Recipes;
                }
                ProjectProfileItemKind::FavoriteImage(_) => app.screen = Screen::Images,
                ProjectProfileItemKind::FavoriteLayer(index) => {
                    let identity = &profile.favorites.layers[index];
                    app.layer_selection = app
                        .workspace
                        .layers
                        .iter()
                        .position(|layer| &layer.name == identity)
                        .unwrap_or(0);
                    app.screen = Screen::Layers;
                }
                ProjectProfileItemKind::Workflow(_) => {
                    app.notification = Some(
                        "Workflow selected. Review each typed step; loading never executes it."
                            .into(),
                    );
                }
            }
        }
        Action::OpenRawFavorites => {
            let _ = update(app, Action::Open(Screen::RawMode));
            return update(app, Action::RawMode(RawModeAction::OpenFavorites));
        }
        Action::InspectKernel => {
            app.kernel.inventory = PlatformInventoryState::Loading;
            return Some(Effect::InspectKernel);
        }
        Action::KernelLoaded(mut inventory) => {
            inventory
                .files
                .sort_by(|left, right| left.path.cmp(&right.path));
            app.kernel.inventory = PlatformInventoryState::Available(inventory);
            app.kernel.config_selection = 0;
            app.kernel.device_tree_selection = 0;
        }
        Action::KernelFailed(message) => {
            app.kernel.inventory = PlatformInventoryState::Failed(message.clone());
            app.notification = Some(format!("Kernel inspection failed: {message}"));
        }
        Action::CycleKernelView => app.kernel.cycle_view(),
        Action::SetKernelView(view) => app.kernel.view = view,
        Action::SelectKernelFile { delta } => app.kernel.select(delta),
        Action::LaunchKernelMenuconfig => {
            if app.daemon.status != ClientReplicaStatus::Current {
                app.notification =
                    Some("Reconnect to a current daemon before opening kernel menuconfig.".into());
            } else if !app.build_environment.connected() {
                app.notification = Some("Verify the build environment first.".into());
            } else if !app.kernel.inventory().is_some_and(|inventory| {
                inventory
                    .tasks
                    .iter()
                    .any(|task| task == "menuconfig" || task == "do_menuconfig")
            }) {
                app.notification = Some(
                    "The kernel provider did not report an authoritative menuconfig task.".into(),
                );
            } else if let Some(cwd) = app.workspace.build_dir.clone() {
                open_terminal_launch(
                    app,
                    TerminalLaunchRequest {
                        name: "kernel menuconfig".into(),
                        kind: TerminalCreationKind::Menuconfig,
                        cwd,
                        program: PathBuf::from("/usr/bin/env"),
                        arguments: vec![
                            "bitbake".into(),
                            "virtual/kernel".into(),
                            "-c".into(),
                            "menuconfig".into(),
                        ],
                    },
                );
            } else {
                app.notification = Some("No authoritative build directory is available.".into());
            }
        }
        Action::OpenSelectedKernelFile => {
            let Some(file) = app.kernel.selected_file().cloned() else {
                app.notification = Some("Select a kernel file first.".into());
                return None;
            };
            if !file.kind.is_text() {
                app.notification = Some("A DTB is binary; press d to decompile it to DTS.".into());
                return None;
            }
            let Ok(relative) = file.path.strip_prefix(&file.root) else {
                app.notification =
                    Some("The selected file is outside its authoritative root.".into());
                return None;
            };
            return Some(Effect::OpenLayerBrowserEditor {
                layer: "Kernel".into(),
                root: file.root,
                file: relative.to_path_buf(),
            });
        }
        Action::ExploreSelectedKernelRoot => {
            let Some(file) = app.kernel.selected_file() else {
                app.notification = Some("Select a kernel file first.".into());
                return None;
            };
            return Some(Effect::OpenWorkspaceEditor {
                label: "Kernel".into(),
                root: file.root.clone(),
            });
        }
        Action::CompileSelectedKernelDts => begin_platform_dtc_compile(app, true),
        Action::DecompileSelectedKernelDtb => begin_platform_dtc_decompile(app, true),
        Action::InspectFirmware => {
            app.firmware.inventory = PlatformInventoryState::Loading;
            return Some(Effect::InspectFirmware);
        }
        Action::FirmwareLoaded(mut inventory) => {
            inventory
                .files
                .sort_by(|left, right| left.path.cmp(&right.path));
            app.firmware.inventory = PlatformInventoryState::Available(inventory);
            app.firmware.config_selection = 0;
            app.firmware.device_tree_selection = 0;
        }
        Action::FirmwareFailed(message) => {
            app.firmware.inventory = PlatformInventoryState::Failed(message.clone());
            app.notification = Some(format!("Firmware inspection failed: {message}"));
        }
        Action::CycleFirmwareView => app.firmware.cycle_view(),
        Action::SetFirmwareView(view) => app.firmware.view = view,
        Action::SelectFirmwareFile { delta } => app.firmware.select(delta),
        Action::LaunchFirmwareMenuconfig => {
            if app.daemon.status != ClientReplicaStatus::Current {
                app.notification = Some(
                    "Reconnect to a current daemon before opening firmware menuconfig.".into(),
                );
            } else if !app.build_environment.connected() {
                app.notification = Some("Verify the build environment first.".into());
            } else if !app.firmware.inventory().is_some_and(|inventory| {
                inventory
                    .tasks
                    .iter()
                    .any(|task| task == "menuconfig" || task == "do_menuconfig")
            }) {
                app.notification = Some(
                    "The detected firmware provider did not report an authoritative menuconfig task."
                        .into(),
                );
            } else if let (Some(cwd), Some(inventory)) =
                (app.workspace.build_dir.clone(), app.firmware.inventory())
            {
                let target = inventory.target.clone();
                let component = inventory.component.label().to_lowercase();
                open_terminal_launch(
                    app,
                    TerminalLaunchRequest {
                        name: format!("{component} menuconfig"),
                        kind: TerminalCreationKind::Menuconfig,
                        cwd,
                        program: PathBuf::from("/usr/bin/env"),
                        arguments: vec!["bitbake".into(), target, "-c".into(), "menuconfig".into()],
                    },
                );
            } else {
                app.notification = Some("No authoritative build directory is available.".into());
            }
        }
        Action::OpenSelectedFirmwareFile => {
            let Some(file) = app.firmware.selected_file().cloned() else {
                app.notification = Some("Select a firmware file first.".into());
                return None;
            };
            if !file.kind.is_text() {
                app.notification = Some("A DTB is binary; press d to decompile it to DTS.".into());
                return None;
            }
            let Ok(relative) = file.path.strip_prefix(&file.root) else {
                app.notification =
                    Some("The selected file is outside its authoritative root.".into());
                return None;
            };
            return Some(Effect::OpenLayerBrowserEditor {
                layer: "Firmware".into(),
                root: file.root,
                file: relative.to_path_buf(),
            });
        }
        _ => unreachable!("action routed to the wrong reducer"),
    }
    synchronize_focus(app);
    None
}
