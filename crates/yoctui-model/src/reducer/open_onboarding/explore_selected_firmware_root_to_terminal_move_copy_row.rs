use super::*;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match action {
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
            if s == Screen::BuildHistory {
                app.saved_builds.reload_requested = true;
            }
            app.focus = if s == Screen::BuildHistory {
                FocusTarget::Workspace
            } else {
                FocusTarget::Navigator
            };
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
            if app.is_offline() {
                return None;
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
                        if dialog.request.kind == TerminalCreationKind::GitUi && !app.is_offline() {
                            app.screen = Screen::TerminalSessions;
                            app.focus = FocusTarget::Workspace;
                            app.focus_return = None;
                            app.pty_selection = app.daemon.pty_sessions.len();
                            app.notification = Some("GitUI requested. Press o to take writer control; Ctrl+B t returns to sessions.".into());
                        }
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
        _ => unreachable!("action routed to the wrong reducer"),
    }
    synchronize_focus(app);
    None
}
