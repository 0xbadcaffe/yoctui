//! State transitions beginning with OpenOnboarding.
use super::*;

fn selected_platform_dtc(
    app: &App,
    kernel: bool,
) -> Result<(PlatformFile, PathBuf, PlatformComponent), String> {
    let workbench = if kernel { &app.kernel } else { &app.firmware };
    let file = workbench
        .selected_file()
        .cloned()
        .ok_or_else(|| "Select a device-tree file first.".to_owned())?;
    let inventory = workbench
        .inventory()
        .ok_or_else(|| "Refresh the platform inventory before running dtc.".to_owned())?;
    if !yoctui_utils::is_absolute_normal_path(&file.root)
        || !yoctui_utils::is_absolute_normal_path(&file.path)
        || file.path.strip_prefix(&file.root).is_err()
        || !inventory.roots.iter().any(|root| root == &file.root)
    {
        return Err("The selected file is outside its authoritative root.".into());
    }
    let program = inventory
        .dtc
        .clone()
        .ok_or_else(|| "No authoritative dtc executable was found in PATH.".to_owned())?;
    if !yoctui_utils::is_absolute_normal_path(&program) {
        return Err("The reported dtc executable path is unsafe.".into());
    }
    Ok((file, program, inventory.component))
}

fn ensure_output_absent(app: &mut App, output: &Path) -> bool {
    match yoctui_utils::path_entry_exists(output) {
        Ok(false) => true,
        Ok(true) => {
            app.notification = Some(format!(
                "Refusing to overwrite {}; move or remove it first.",
                output.display()
            ));
            false
        }
        Err(error) => {
            app.notification = Some(format!(
                "Cannot verify that {} is available: {error}",
                output.display()
            ));
            false
        }
    }
}

fn begin_platform_dtc_compile(app: &mut App, kernel: bool) {
    let (file, program, component) = match selected_platform_dtc(app, kernel) {
        Ok(values) => values,
        Err(message) => {
            app.notification = Some(message);
            return;
        }
    };
    if file.kind != PlatformFileKind::Dts {
        app.notification = Some("Select a DTS source before compiling.".into());
        return;
    }
    let dialog = DtcCompileDialog::new(component, &file, program);
    if !ensure_output_absent(app, &dialog.output) {
        return;
    }
    open_dialog(app, Dialog::DtcCompile(dialog));
}

fn begin_platform_dtc_decompile(app: &mut App, kernel: bool) {
    let (file, program, component) = match selected_platform_dtc(app, kernel) {
        Ok(values) => values,
        Err(message) => {
            app.notification = Some(message);
            return;
        }
    };
    if !matches!(file.kind, PlatformFileKind::Dtb | PlatformFileKind::Dtbo) {
        app.notification = Some("Select a DTB or DTBO before decompiling.".into());
        return;
    }
    let stem = file
        .path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("device-tree");
    let output = file.path.with_file_name(format!("{stem}.yoctui.dts"));
    if !ensure_output_absent(app, &output) {
        return;
    }
    open_dialog(
        app,
        Dialog::TerminalLaunch(TerminalLaunchDialog {
            request: TerminalLaunchRequest {
                name: format!(
                    "decompile {} device tree",
                    component.label().to_ascii_lowercase()
                ),
                kind: TerminalCreationKind::Utility,
                cwd: file.root,
                program,
                arguments: vec![
                    "-I".into(),
                    "dtb".into(),
                    "-O".into(),
                    "dts".into(),
                    "-o".into(),
                    output.display().to_string(),
                    file.path.display().to_string(),
                ],
            },
            destination: TerminalLaunchDestination::Embedded,
            output_must_not_exist: Some(output),
        }),
    );
}

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
        Action::ExploreSelectedFirmwareRoot => {
            let Some(file) = app.firmware.selected_file() else {
                app.notification = Some("Select a firmware file first.".into());
                return None;
            };
            return Some(Effect::OpenWorkspaceEditor {
                label: "Firmware".into(),
                root: file.root.clone(),
            });
        }
        Action::CompileSelectedFirmwareDts => begin_platform_dtc_compile(app, false),
        Action::DecompileSelectedFirmwareDtb => begin_platform_dtc_decompile(app, false),
        Action::SelectDtcCompileOption { delta } => {
            if let Some(Dialog::DtcCompile(dialog)) = app.active_dialog_mut() {
                dialog.select(delta);
            }
        }
        Action::AdjustDtcCompileOption { delta } => {
            if let Some(Dialog::DtcCompile(dialog)) = app.active_dialog_mut() {
                dialog.adjust(delta);
            }
        }
        Action::ConfirmDtcCompileOptions => {
            if let Some(Dialog::DtcCompile(dialog)) = app.active_dialog().cloned()
                && ensure_output_absent(app, &dialog.output)
            {
                let output = dialog.output.clone();
                replace_dialog(
                    app,
                    Dialog::TerminalLaunch(TerminalLaunchDialog {
                        request: dialog.terminal_request(),
                        destination: TerminalLaunchDestination::Embedded,
                        output_must_not_exist: Some(output),
                    }),
                );
            }
        }
        Action::CancelDtcCompileOptions => {
            if matches!(app.active_dialog(), Some(Dialog::DtcCompile(_))) {
                close_dialog(app);
            }
        }
        Action::ShiftOverviewView { delta } => {
            app.overview_view = app.overview_view.shifted(delta);
        }
        Action::SelectOverviewView(view) => {
            app.overview_view = view;
        }
        Action::Open(s) => {
            let correlated_log_id = (s == Screen::Logs)
                .then(|| selected_correlated_log_id(app))
                .flatten();
            app.screen = s;
            app.focus = FocusTarget::Navigator;
            app.focus_return = None;
            app.workspace_subfocus = WorkspaceSubfocus::Main;
            if app.zoomed_pane.is_some() {
                app.zoomed_pane = Some(app.focus);
            }
            if let Some(index) = NAVIGATOR_SCREENS
                .iter()
                .position(|candidate| *candidate == s)
            {
                app.navigator_selection = index;
            }
            if let Some(id) = correlated_log_id {
                app.logs.jump_to(id);
            }
            if s == Screen::Packages
                && matches!(app.package_inventory, PackageInventoryState::NotLoaded)
            {
                return Some(begin_package_inventory(app));
            }
            if s == Screen::Kernel
                && matches!(app.kernel.inventory, PlatformInventoryState::NotLoaded)
            {
                app.kernel.inventory = PlatformInventoryState::Loading;
                return Some(Effect::InspectKernel);
            }
            if s == Screen::Firmware
                && matches!(app.firmware.inventory, PlatformInventoryState::NotLoaded)
            {
                app.firmware.inventory = PlatformInventoryState::Loading;
                return Some(Effect::InspectFirmware);
            }
            if s == Screen::Images
                && matches!(app.image_artifacts, ImageArtifactInventoryState::NotLoaded)
            {
                return begin_image_artifact_inventory(app);
            }
            if s == Screen::Sdk
                && matches!(app.sdk_tool_capability, SdkToolCapability::NotInspected)
            {
                return Some(Effect::InspectSdkTools);
            }
            if s == Screen::Testing
                && matches!(
                    app.test_capability.oe_selftest,
                    TestExecutableCapability::NotInspected
                )
                && matches!(
                    app.test_capability.bitbake_selftest,
                    TestExecutableCapability::NotInspected
                )
            {
                return Some(Effect::InspectTestCapability);
            }
            if s == Screen::Testing
                && matches!(
                    app.result_tool_capability,
                    ResultToolCapability::NotInspected
                )
            {
                return Some(Effect::InspectResultToolCapability);
            }
            if s == Screen::Security
                && matches!(app.security.capability, SecurityCapability::NotInspected)
            {
                return Some(Effect::Security(SecurityEffect::InspectCapability));
            }
            if s == Screen::Qa && matches!(app.qa.capability, QaCapability::NotInspected) {
                return Some(Effect::Qa(QaEffect::InspectCapability {
                    scope: app.qa.scope.clone(),
                }));
            }
            if s == Screen::Maintenance
                && matches!(
                    app.maintenance.capability,
                    MaintenanceCapability::NotInspected
                )
            {
                return update(
                    app,
                    Action::Maintenance(MaintenanceAction::InspectCapability),
                );
            }
        }
        Action::SelectNavigator { delta } => {
            let visible = (0..NAVIGATOR_SCREENS.len())
                .filter(|selection| app.navigator_selection_is_visible(*selection))
                .collect::<Vec<_>>();
            let next = if let Some(current) = visible
                .iter()
                .position(|selection| *selection == app.navigator_selection)
            {
                shifted_index(current, delta, visible.len())
            } else if delta.is_negative() {
                let current = visible
                    .iter()
                    .rposition(|selection| *selection < app.navigator_selection)
                    .unwrap_or(0);
                shifted_index(current, delta.saturating_add(1), visible.len())
            } else {
                let current = visible
                    .iter()
                    .position(|selection| *selection > app.navigator_selection)
                    .unwrap_or_else(|| visible.len().saturating_sub(1));
                shifted_index(current, delta.saturating_sub(1), visible.len())
            };
            app.navigator_selection = visible.get(next).copied().unwrap_or(0);
        }
        Action::SelectNavigatorAt { index } => {
            if index < NAVIGATOR_SCREENS.len() && app.navigator_selection_is_visible(index) {
                app.navigator_selection = index;
                app.focus = FocusTarget::Navigator;
                app.focus_return = None;
            }
        }
        Action::ToggleNavigatorGroup { group } => {
            if let Some(expanded) = app.navigator_groups_expanded.get_mut(group) {
                *expanded = !*expanded;
                app.navigator_selection = NAVIGATOR_GROUPS[group].start;
                app.focus = FocusTarget::Navigator;
                app.focus_return = None;
            }
        }
        Action::CollapseNavigatorGroup => {
            let group = app.navigator_group_index();
            app.navigator_groups_expanded[group] = false;
            app.navigator_selection = NAVIGATOR_GROUPS[group].start;
        }
        Action::ExpandNavigatorGroup => {
            let group = app.navigator_group_index();
            if app.navigator_groups_expanded[group] {
                return update(app, Action::ActivateNavigator);
            }
            app.navigator_groups_expanded[group] = true;
        }
        Action::SelectPtySession { delta } => {
            let count = app.daemon.pty_sessions.len();
            app.pty_selection = if count == 0 {
                0
            } else if delta.is_negative() {
                app.pty_selection
                    .saturating_sub(delta.unsigned_abs())
                    .min(count.saturating_sub(1))
            } else {
                app.pty_selection
                    .saturating_add(delta as usize)
                    .min(count.saturating_sub(1))
            };
        }
        Action::SelectPtyPane { pane, index } => {
            if index < app.daemon.pty_sessions.len() && app.pane_layout.focus(pane).is_ok() {
                app.pty_selection = index;
            }
        }
        Action::TerminalTakeControl => {
            if app.daemon.status != ClientReplicaStatus::Current {
                app.notification = Some(
                    "Reconnect to a current daemon replica before taking terminal control.".into(),
                );
                return None;
            }
            if let Some(session) = app.selected_terminal_session() {
                if session.lifecycle != ClientDaemonLifecycle::Running {
                    app.notification =
                        Some("Only a running terminal can grant writer control.".into());
                } else if app.selected_terminal_is_writer() {
                    app.notification =
                        Some("This client already owns the terminal writer lease.".into());
                } else if app
                    .selected_terminal_details()
                    .is_some_and(|details| details.writer.is_some())
                {
                    app.notification =
                        Some("Terminal already has a writer; it remains read-only here.".into());
                } else if let Some(details) = app.selected_terminal_details() {
                    return Some(Effect::Terminal(TerminalEffect::TakeControl {
                        session_id: session.id,
                        expected_epoch: details.writer_epoch,
                    }));
                }
            }
        }
        Action::TerminalReleaseControl => {
            if let (Some(session), Some(details)) = (
                app.selected_terminal_session(),
                app.selected_terminal_details(),
            ) && app.selected_terminal_is_writer()
            {
                return Some(Effect::Terminal(TerminalEffect::ReleaseControl {
                    session_id: session.id,
                    writer_epoch: details.writer_epoch,
                }));
            }
            app.notification = Some("This client does not own the terminal writer lease.".into());
        }
        Action::TerminalCreateBuildShell => {
            begin_terminal_creation(app, None);
        }
        Action::TerminalCreateSelectedDevshell => {
            begin_terminal_creation(app, Some("devshell"));
        }
        Action::TerminalCreateSelectedMenuconfig => {
            begin_terminal_creation(app, Some("menuconfig"));
        }
        Action::DetachedTerminalAvailabilityDetected(availability) => {
            app.detached_terminal = availability;
        }
        Action::SelectTerminalLaunchDestination { delta } => {
            let detached_available = matches!(
                app.detached_terminal,
                DetachedTerminalAvailability::Available { .. }
            );
            if let Some(Dialog::TerminalLaunch(dialog)) = app.active_dialog_mut() {
                dialog.destination = match (dialog.destination, delta.is_positive()) {
                    (TerminalLaunchDestination::Embedded, true) if detached_available => {
                        TerminalLaunchDestination::Detached
                    }
                    (TerminalLaunchDestination::Detached, false) => {
                        TerminalLaunchDestination::Embedded
                    }
                    (current, _) => current,
                };
            }
        }
        Action::ConfirmTerminalLaunch => {
            if let Some(Dialog::TerminalLaunch(dialog)) = app.active_dialog().cloned() {
                if dialog
                    .output_must_not_exist
                    .as_deref()
                    .is_some_and(|output| !ensure_output_absent(app, output))
                {
                    return None;
                }
                close_dialog(app);
                return Some(match dialog.destination {
                    TerminalLaunchDestination::Embedded => {
                        Effect::Terminal(TerminalEffect::Create {
                            name: dialog.request.name,
                            kind: dialog.request.kind,
                            cwd: dialog.request.cwd,
                            program: dialog.request.program,
                            arguments: dialog.request.arguments,
                        })
                    }
                    TerminalLaunchDestination::Detached => {
                        Effect::LaunchDetachedTerminal(dialog.request)
                    }
                });
            }
        }
        Action::CancelTerminalLaunch => {
            if matches!(app.active_dialog(), Some(Dialog::TerminalLaunch(_))) {
                close_dialog(app);
            }
        }
        Action::TerminalEnterCopyMode => {
            if let Some(row) = app
                .selected_terminal_screen()
                .map(|screen| screen.rows.len().saturating_sub(1))
            {
                app.terminal.mode = TerminalWorkbenchMode::Copy;
                app.terminal.copy_row = row;
            }
        }
        Action::TerminalMoveCopyRow { delta } => {
            if app.terminal.mode == TerminalWorkbenchMode::Copy {
                let count = app
                    .selected_terminal_screen()
                    .map_or(0, |screen| screen.rows.len());
                app.terminal.copy_row = shifted_index(app.terminal.copy_row, delta, count);
            }
        }
        Action::TerminalCopyViewport => {
            if app.terminal.mode == TerminalWorkbenchMode::Copy
                && let Some(screen) = app.selected_terminal_screen()
                && let Some(row) = screen.rows.get(app.terminal.copy_row)
            {
                let content = row.clone();
                app.terminal.reset_transient_mode();
                return Some(Effect::CopyToClipboard(content));
            }
        }
        Action::TerminalBeginSearch => app.terminal.mode = TerminalWorkbenchMode::Search,
        _ => unreachable!("action routed to the wrong reducer"),
    }
    synchronize_focus(app);
    None
}
