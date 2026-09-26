use super::*;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match action {
        Action::TerminalAppendSearch(character) => {
            if app.terminal.mode == TerminalWorkbenchMode::Search
                && !app.terminal.append_query(character)
            {
                app.notification = Some(format!(
                    "Terminal search is limited to {MAX_TERMINAL_SEARCH_BYTES} bytes."
                ));
            }
        }
        Action::TerminalBackspaceSearch => app.terminal.backspace_query(),
        Action::TerminalFinishSearch => app.terminal.mode = TerminalWorkbenchMode::Live,
        Action::TerminalClearSearch => {
            app.terminal.query.clear();
            app.terminal.mode = TerminalWorkbenchMode::Live;
        }
        Action::TerminalStagePaste(text) => {
            if !app.selected_terminal_is_writer() {
                app.notification =
                    Some("Paste is disabled until this client owns the writer lease.".into());
            } else if !app.terminal.stage_paste(&text) {
                app.notification = Some(format!(
                    "Terminal paste must contain 1..={MAX_TERMINAL_PASTE_BYTES} bytes."
                ));
            }
        }
        Action::TerminalConfirmPaste => {
            if app.terminal.mode == TerminalWorkbenchMode::PasteReview
                && app.selected_terminal_is_writer()
                && let (Some(session), Some(details)) = (
                    app.selected_terminal_session(),
                    app.selected_terminal_details(),
                )
            {
                let effect = TerminalEffect::Input {
                    session_id: session.id,
                    writer_epoch: details.writer_epoch,
                    bytes: app.terminal.pending_paste.clone(),
                };
                app.terminal.reset_transient_mode();
                return Some(Effect::Terminal(effect));
            }
        }
        Action::TerminalBeginRename => {
            if app.daemon.status != ClientReplicaStatus::Current {
                app.notification = Some(
                    "Reconnect to a current daemon replica before renaming a terminal.".into(),
                );
                return None;
            }
            if let Some(name) = app
                .selected_terminal_session()
                .map(|session| session.name.clone())
            {
                app.terminal.rename = name;
                app.terminal.mode = TerminalWorkbenchMode::Rename;
            }
        }
        Action::TerminalAppendRename(character) => {
            if app.terminal.mode == TerminalWorkbenchMode::Rename
                && !app.terminal.append_rename(character)
            {
                app.notification = Some(format!(
                    "Terminal names are limited to {MAX_TERMINAL_RENAME_BYTES} bytes."
                ));
            }
        }
        Action::TerminalBackspaceRename => app.terminal.backspace_rename(),
        Action::TerminalConfirmRename => {
            if app.terminal.mode == TerminalWorkbenchMode::Rename
                && let Some(session) = app.selected_terminal_session()
            {
                let name = app.terminal.rename.trim().to_owned();
                if name.is_empty() {
                    app.notification = Some("Terminal name cannot be empty.".into());
                } else {
                    let effect = TerminalEffect::Rename {
                        session_id: session.id,
                        name,
                    };
                    app.terminal.reset_transient_mode();
                    return Some(Effect::Terminal(effect));
                }
            }
        }
        Action::TerminalScroll { delta } => {
            if let Some(screen) = app.selected_terminal_screen() {
                let maximum = screen.scrollback_lines as usize;
                let next = if delta.is_negative() {
                    app.terminal
                        .scrollback_offset
                        .saturating_sub(delta.unsigned_abs())
                } else {
                    app.terminal
                        .scrollback_offset
                        .saturating_add(delta as usize)
                }
                .min(maximum);
                app.terminal.set_scrollback_offset(next, maximum);
                if let Some(session) = app.selected_terminal_session() {
                    return Some(Effect::Terminal(TerminalEffect::Viewport {
                        session_id: session.id,
                        scrollback_offset: next,
                    }));
                }
            }
        }
        Action::TerminalBeginKill => {
            if app.daemon.status != ClientReplicaStatus::Current {
                app.notification = Some(
                    "Reconnect to a current daemon replica before closing or killing a terminal."
                        .into(),
                );
                return None;
            }
            if let Some(session) = app.selected_terminal_session() {
                if session.lifecycle == ClientDaemonLifecycle::Running {
                    app.terminal.mode = TerminalWorkbenchMode::KillConfirmation;
                } else {
                    return Some(Effect::Terminal(TerminalEffect::Close {
                        session_id: session.id,
                    }));
                }
            }
        }
        Action::TerminalConfirmKill => {
            if app.terminal.mode == TerminalWorkbenchMode::KillConfirmation
                && let Some(session) = app.selected_terminal_session()
            {
                let session_id = session.id;
                app.terminal.reset_transient_mode();
                return Some(Effect::Terminal(TerminalEffect::Terminate { session_id }));
            }
        }
        Action::TerminalCancelMode => app.terminal.reset_transient_mode(),
        Action::TerminalToggleHelp => {
            app.terminal.mode = if app.terminal.mode == TerminalWorkbenchMode::Help {
                TerminalWorkbenchMode::Live
            } else {
                TerminalWorkbenchMode::Help
            };
        }
        Action::ResizeFocusedPane { delta_per_mille } => {
            let focused = app.pane_layout.focused;
            let _ = app.pane_layout.resize(focused, delta_per_mille);
        }
        Action::ActivateNavigator => {
            let group = app.navigator_group_index();
            if !app.navigator_groups_expanded[group] {
                app.navigator_groups_expanded[group] = true;
                return None;
            }
            app.screen = NAVIGATOR_SCREENS[app.navigator_selection];
            app.focus = if focus_target_is_relevant(app, FocusTarget::Workspace) {
                FocusTarget::Workspace
            } else {
                FocusTarget::Navigator
            };
            app.focus_return = None;
            if app.is_offline() {
                return None;
            }
            if app.screen == Screen::Packages
                && matches!(app.package_inventory, PackageInventoryState::NotLoaded)
            {
                return Some(begin_package_inventory(app));
            }
            if app.screen == Screen::Kernel
                && matches!(app.kernel.inventory, PlatformInventoryState::NotLoaded)
            {
                app.kernel.inventory = PlatformInventoryState::Loading;
                return Some(Effect::InspectKernel);
            }
            if app.screen == Screen::Firmware
                && matches!(app.firmware.inventory, PlatformInventoryState::NotLoaded)
            {
                app.firmware.inventory = PlatformInventoryState::Loading;
                return Some(Effect::InspectFirmware);
            }
            if app.screen == Screen::Images
                && matches!(app.image_artifacts, ImageArtifactInventoryState::NotLoaded)
            {
                return begin_image_artifact_inventory(app);
            }
            if app.screen == Screen::Sdk
                && matches!(app.sdk_tool_capability, SdkToolCapability::NotInspected)
            {
                return Some(Effect::InspectSdkTools);
            }
            if app.screen == Screen::Testing
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
            if app.screen == Screen::Testing
                && matches!(
                    app.result_tool_capability,
                    ResultToolCapability::NotInspected
                )
            {
                return Some(Effect::InspectResultToolCapability);
            }
            if app.screen == Screen::Security
                && matches!(app.security.capability, SecurityCapability::NotInspected)
            {
                return Some(Effect::Security(SecurityEffect::InspectCapability));
            }
            if app.screen == Screen::Qa && matches!(app.qa.capability, QaCapability::NotInspected) {
                return Some(Effect::Qa(QaEffect::InspectCapability {
                    scope: app.qa.scope.clone(),
                }));
            }
            if app.screen == Screen::Maintenance
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
        Action::Security(action) => {
            let transition = update_security(&mut app.security, action);
            match transition.dialog {
                SecurityDialogUpdate::None => {}
                SecurityDialogUpdate::Open(dialog) => {
                    if matches!(app.active_dialog(), Some(Dialog::Security(_))) {
                        replace_dialog(app, Dialog::Security(dialog));
                    } else {
                        open_dialog(app, Dialog::Security(dialog));
                    }
                }
                SecurityDialogUpdate::Close => {
                    if matches!(app.active_dialog(), Some(Dialog::Security(_))) {
                        close_dialog(app);
                    }
                }
            }
            if let Some(message) = transition.notification {
                app.notification = Some(message);
            }
            synchronize_focus(app);
            return transition.effect.map(Effect::Security);
        }
        Action::Qa(action) => {
            let transition = update_qa(&mut app.qa, action);
            match transition.dialog {
                QaDialogUpdate::None => {}
                QaDialogUpdate::Open(dialog) => {
                    if matches!(app.active_dialog(), Some(Dialog::Qa(_))) {
                        replace_dialog(app, Dialog::Qa(*dialog));
                    } else {
                        open_dialog(app, Dialog::Qa(*dialog));
                    }
                }
                QaDialogUpdate::Close => {
                    if matches!(app.active_dialog(), Some(Dialog::Qa(_))) {
                        close_dialog(app);
                    }
                }
            }
            if let Some(message) = transition.notification {
                app.notification = Some(message);
            }
            synchronize_focus(app);
            return transition.effect.map(Effect::Qa);
        }
        Action::Maintenance(action) => {
            let transition = update_maintenance(&mut app.maintenance, action);
            match transition.dialog {
                MaintenanceDialogUpdate::None => {}
                MaintenanceDialogUpdate::Open(dialog) => {
                    if matches!(app.active_dialog(), Some(Dialog::Maintenance(_))) {
                        replace_dialog(app, Dialog::Maintenance(dialog));
                    } else {
                        open_dialog(app, Dialog::Maintenance(dialog));
                    }
                }
                MaintenanceDialogUpdate::Close => {
                    if matches!(app.active_dialog(), Some(Dialog::Maintenance(_))) {
                        close_dialog(app);
                    }
                }
            }
            if let Some(message) = transition.notification {
                app.notification = Some(message);
            }
            synchronize_focus(app);
            return transition.effect.map(Effect::Maintenance);
        }
        Action::Focus(target) => {
            let target = if focus_target_is_relevant(app, target) {
                target
            } else {
                FocusTarget::Navigator
            };
            app.focus = target;
            if app.zoomed_pane.is_some() && is_pane_focus(target) {
                app.zoomed_pane = Some(target);
            }
        }
        Action::OpenCommandPalette => {
            app.command_palette_open = true;
            app.command_palette_mode = CommandPaletteMode::Commands;
            app.command_palette_selection = 0;
            app.command_palette_query.clear();
        }
        Action::OpenGlobalSearch => {
            app.command_palette_open = true;
            app.command_palette_mode = CommandPaletteMode::GlobalRegexSearch;
            app.command_palette_selection = 0;
            app.command_palette_query.clear();
            app.global_search_content = GlobalSearchContentState::Idle;
            app.global_search_root = None;
        }
        Action::OpenRecipeEditorWorkspaceSearch => {
            let root = app.active_dialog().and_then(|dialog| match dialog {
                Dialog::RecipeEditor(editor) => Some(editor.root.clone()),
                _ => None,
            })?;
            if !root.is_absolute() {
                app.notification =
                    Some("The Devtool workspace search root is not absolute.".into());
                return None;
            }
            app.command_palette_open = true;
            app.command_palette_mode = CommandPaletteMode::GlobalRegexSearch;
            app.command_palette_selection = 0;
            app.command_palette_query.clear();
            app.global_search_content = GlobalSearchContentState::Idle;
            app.global_search_root = Some(root);
        }
        Action::SelectCommandPalette { delta } => {
            let count = app.filtered_command_palette_commands().len()
                + if app.command_palette_mode == CommandPaletteMode::GlobalRegexSearch {
                    app.global_search_content.hits().len()
                } else {
                    0
                };
            app.command_palette_selection = if delta.is_negative() {
                app.command_palette_selection
                    .saturating_sub(delta.unsigned_abs())
            } else {
                app.command_palette_selection
                    .saturating_add(delta as usize)
                    .min(count.saturating_sub(1))
            };
        }
        _ => unreachable!("action routed to the wrong reducer"),
    }
    synchronize_focus(app);
    None
}
