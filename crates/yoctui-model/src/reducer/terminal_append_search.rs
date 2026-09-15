//! State transitions beginning with TerminalAppendSearch.
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
            app.focus = FocusTarget::Navigator;
            app.focus_return = None;
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
        Action::AppendCommandPaletteQuery(character) if app.command_palette_open => {
            if !character.is_control()
                && app.command_palette_query.chars().count() < MAX_COMMAND_PALETTE_QUERY_CHARS
            {
                app.command_palette_query.push(character);
                app.command_palette_selection = 0;
                app.global_search_content = GlobalSearchContentState::Idle;
            }
        }
        Action::BackspaceCommandPaletteQuery if app.command_palette_open => {
            app.command_palette_query.pop();
            app.command_palette_selection = 0;
            app.global_search_content = GlobalSearchContentState::Idle;
        }
        Action::ClearCommandPaletteQuery if app.command_palette_open => {
            app.command_palette_query.clear();
            app.command_palette_selection = 0;
            app.global_search_content = GlobalSearchContentState::Idle;
        }
        Action::BeginGlobalContentSearch
            if app.command_palette_open
                && app.command_palette_mode == CommandPaletteMode::GlobalRegexSearch
                && !app.command_palette_query.trim().is_empty()
                && app.command_palette_regex_error().is_none() =>
        {
            app.global_search_generation = app.global_search_generation.wrapping_add(1).max(1);
            app.global_search_content = GlobalSearchContentState::Loading {
                generation: app.global_search_generation,
                query: app.command_palette_query.clone(),
            };
        }
        Action::GlobalContentSearchLoaded {
            generation,
            query,
            mut hits,
            truncated,
            mut searched_scopes,
        } => {
            if generation != app.global_search_generation
                || query != app.command_palette_query
                || app.command_palette_mode != CommandPaletteMode::GlobalRegexSearch
            {
                return None;
            }
            hits.retain(|hit| {
                hit.path.is_absolute()
                    && hit.line > 0
                    && hit.column > 0
                    && !hit.preview.chars().any(char::is_control)
            });
            hits.truncate(MAX_GLOBAL_SEARCH_HITS);
            searched_scopes.sort();
            searched_scopes.dedup();
            app.global_search_content = GlobalSearchContentState::Ready {
                generation,
                query,
                hits,
                truncated,
                searched_scopes,
            };
            let count = app.filtered_command_palette_commands().len()
                + app.global_search_content.hits().len();
            app.command_palette_selection =
                app.command_palette_selection.min(count.saturating_sub(1));
        }
        Action::GlobalContentSearchFailed {
            generation,
            query,
            message,
        } => {
            if generation == app.global_search_generation
                && query == app.command_palette_query
                && app.command_palette_mode == CommandPaletteMode::GlobalRegexSearch
            {
                app.global_search_content = GlobalSearchContentState::Failed {
                    generation,
                    query,
                    message,
                };
            }
        }
        Action::ActivateCommandPalette => {
            if !app.command_palette_open {
                return None;
            }
            let commands = app.filtered_command_palette_commands();
            if app.command_palette_mode == CommandPaletteMode::GlobalRegexSearch
                && app.command_palette_selection >= commands.len()
            {
                let hit = app
                    .global_search_content
                    .hits()
                    .get(app.command_palette_selection - commands.len())
                    .cloned()?;
                app.command_palette_open = false;
                synchronize_focus(app);
                return Some(Effect::OpenInEditor(hit.path));
            }
            let command = commands.get(app.command_palette_selection).cloned()?;
            if !command.enabled() {
                return None;
            }
            app.command_palette_open = false;
            synchronize_focus(app);
            return update(app, command_action(app, command.id));
        }
        Action::CloseCommandPalette => {
            app.command_palette_open = false;
        }
        Action::OpenApplicationMenu => {
            app.command_palette_open = false;
            app.menu.open_application();
        }
        Action::OpenContextMenu => {
            app.command_palette_open = false;
            app.menu
                .open_context(workspace_screen_destination(app.screen));
        }
        Action::SelectMenuGroup { delta } if app.menu.kind == Some(MenuKind::Application) => {
            let current = app.menu.group_selection;
            app.menu.group_selection = if delta.is_negative() {
                current.saturating_sub(delta.unsigned_abs())
            } else {
                current
                    .saturating_add(delta as usize)
                    .min(ApplicationMenuGroup::ALL.len().saturating_sub(1))
            };
            app.menu.item_selection = 0;
            app.menu.typed_prefix.clear();
        }
        Action::SelectMenuGroup { .. } => {}
        Action::SelectMenuItem { delta } if app.menu.is_open() => {
            let count = app.active_menu_items().len();
            app.menu.item_selection = if delta.is_negative() {
                app.menu.item_selection.saturating_sub(delta.unsigned_abs())
            } else {
                app.menu
                    .item_selection
                    .saturating_add(delta as usize)
                    .min(count.saturating_sub(1))
            };
            app.menu.typed_prefix.clear();
        }
        Action::SelectMenuItem { .. } => {}
        Action::AppendMenuPrefix(character)
            if app.menu.is_open()
                && !character.is_control()
                && app.menu.typed_prefix.chars().count() < MAX_MENU_PREFIX_CHARS =>
        {
            app.menu.typed_prefix.push(character);
            let prefix = app.menu.typed_prefix.to_lowercase();
            if let Some(index) = app
                .active_menu_items()
                .iter()
                .position(|item| item.label.to_lowercase().starts_with(&prefix))
            {
                app.menu.item_selection = index;
            }
        }
        Action::AppendMenuPrefix(_) => {}
        Action::BackspaceMenuPrefix if app.menu.is_open() => {
            app.menu.typed_prefix.pop();
            if !app.menu.typed_prefix.is_empty() {
                let prefix = app.menu.typed_prefix.to_lowercase();
                if let Some(index) = app
                    .active_menu_items()
                    .iter()
                    .position(|item| item.label.to_lowercase().starts_with(&prefix))
                {
                    app.menu.item_selection = index;
                }
            }
        }
        Action::BackspaceMenuPrefix => {}
        Action::CloseMenu => app.menu.close(),
        Action::SelectSetting { delta } => {
            app.settings_selection = if delta.is_negative() {
                app.settings_selection.saturating_sub(delta.unsigned_abs())
            } else {
                app.settings_selection
                    .saturating_add(delta as usize)
                    .min(SETTINGS.len().saturating_sub(1))
            };
        }
        Action::ChangeSelectedSetting { backwards } => {
            if app.settings_selection >= SETTINGS.len() {
                return None;
            }
            let row = app.preference_rows()[app.settings_selection].clone();
            if let Some(reason) = row.disabled_reason {
                app.notification = Some(reason.into());
                return None;
            }
            match row.setting {
                Setting::Theme => app.theme = cycle_theme(app.theme, backwards),
                Setting::Density => {
                    app.preferences.density = match app.preferences.density {
                        UiDensity::Comfortable => UiDensity::Compact,
                        UiDensity::Compact => UiDensity::Comfortable,
                    }
                }
                Setting::Symbols => {
                    app.preferences.symbols = match app.preferences.symbols {
                        SymbolPreference::Unicode => SymbolPreference::Ascii,
                        SymbolPreference::Ascii => SymbolPreference::Unicode,
                    }
                }
                Setting::AnimationSpeed => {
                    app.animation_speed = match app.animation_speed {
                        AnimationSpeed::Slow => AnimationSpeed::Fast,
                        AnimationSpeed::Fast => AnimationSpeed::Slow,
                    }
                }
                Setting::ReducedMotion => app.reduced_motion = !app.reduced_motion,
                Setting::Color => app.color_enabled = !app.color_enabled,
                Setting::Mouse => {
                    app.preferences.mouse_enabled = !app.preferences.mouse_enabled;
                }
                Setting::FooterShortcuts => {
                    app.preferences.footer_shortcuts = !app.preferences.footer_shortcuts;
                }
                Setting::LogWrap => {
                    app.logs.wrap = !app.logs.wrap;
                    if app.logs.wrap {
                        app.logs.horizontal_offset = 0;
                    }
                }
                Setting::LogFollow => {
                    app.logs.follow = !app.logs.follow;
                    app.logs.paused_len = (!app.logs.follow).then_some(app.logs.entries.len());
                    if app.logs.follow {
                        app.logs.selection = app.logs.filtered().count().saturating_sub(1);
                        app.logs.scroll_offset = 0;
                    }
                }
                Setting::RememberPaneSizes => {
                    app.preferences.remember_pane_sizes = !app.preferences.remember_pane_sizes;
                }
                Setting::Charts => {
                    app.preferences.charts = match app.preferences.charts {
                        ChartPreference::Automatic => ChartPreference::AccessibleText,
                        ChartPreference::AccessibleText => ChartPreference::Automatic,
                    }
                }
                Setting::ImagePreviews | Setting::TerminalPrefix => {
                    unreachable!("disabled preference rows return before their transition")
                }
                Setting::Keybindings => {
                    app.keymap_preferences_ui.open = true;
                    app.keymap_preferences_ui.searching = false;
                    app.keymap_preferences_ui.capture = None;
                    app.keymap_preferences_ui.validation_error = None;
                    synchronize_focus(app);
                    return None;
                }
            }
            app.preferences = app.effective_preferences();
            app.settings_dirty = true;
            return Some(Effect::PersistSettings);
        }
        Action::ResetPreferences => {
            let preferences = WorkbenchPreferences::default();
            app.install_preferences(preferences)
                .expect("built-in workbench preferences are valid");
            app.pane_layout = PaneLayout::new(PaneId(1)).expect("valid default pane layout");
            app.settings_selection = 0;
            app.settings_dirty = true;
            return Some(Effect::PersistSettings);
        }
        Action::RetrySettingsPersistence if app.settings_dirty => {
            return Some(Effect::PersistSettings);
        }
        Action::RetrySettingsPersistence => {}
        Action::OpenKeymapPreferences => {
            app.keymap_preferences_ui.open = true;
            app.keymap_preferences_ui.searching = false;
            app.keymap_preferences_ui.capture = None;
            app.keymap_preferences_ui.validation_error = None;
        }
        Action::CloseKeymapPreferences => {
            app.keymap_preferences_ui.open = false;
            app.keymap_preferences_ui.searching = false;
            app.keymap_preferences_ui.capture = None;
            app.keymap_preferences_ui.validation_error = None;
        }
        Action::SelectKeymapPreference { delta } if app.keymap_preferences_ui.open => {
            let count = keymap_preference_rows(
                &app.keymap_preferences,
                &app.effective_keymap,
                &app.keymap_preferences_ui.query,
            )
            .len();
            app.keymap_preferences_ui.selection = if delta.is_negative() {
                app.keymap_preferences_ui
                    .selection
                    .saturating_sub(delta.unsigned_abs())
            } else {
                app.keymap_preferences_ui
                    .selection
                    .saturating_add(delta as usize)
                    .min(count.saturating_sub(1))
            };
            app.keymap_preferences_ui.validation_error = None;
        }
        Action::SelectKeymapPreference { .. } => {}
        Action::BeginKeymapPreferenceSearch if app.keymap_preferences_ui.open => {
            app.keymap_preferences_ui.searching = true;
            app.keymap_preferences_ui.validation_error = None;
        }
        Action::BeginKeymapPreferenceSearch => {}
        Action::AppendKeymapPreferenceQuery(character)
            if app.keymap_preferences_ui.open
                && app.keymap_preferences_ui.searching
                && app.keymap_preferences_ui.query.len() < 128 =>
        {
            app.keymap_preferences_ui.query.push(character);
            app.keymap_preferences_ui.selection = 0;
        }
        Action::AppendKeymapPreferenceQuery(_) => {}
        Action::BackspaceKeymapPreferenceQuery
            if app.keymap_preferences_ui.open && app.keymap_preferences_ui.searching =>
        {
            app.keymap_preferences_ui.query.pop();
            app.keymap_preferences_ui.selection = 0;
        }
        Action::BackspaceKeymapPreferenceQuery => {}
        Action::ClearKeymapPreferenceQuery if app.keymap_preferences_ui.open => {
            app.keymap_preferences_ui.query.clear();
            app.keymap_preferences_ui.selection = 0;
        }
        Action::ClearKeymapPreferenceQuery => {}
        Action::FinishKeymapPreferenceSearch if app.keymap_preferences_ui.open => {
            app.keymap_preferences_ui.searching = false;
        }
        Action::FinishKeymapPreferenceSearch => {}
        Action::BeginKeymapCapture if app.keymap_preferences_ui.open => {
            if app
                .keymap_preferences_ui
                .selected_row(&app.keymap_preferences, &app.effective_keymap)
                .is_some()
            {
                app.keymap_preferences_ui.capture = Some(KeymapCaptureState::default());
                app.keymap_preferences_ui.validation_error = None;
            }
        }
        Action::BeginKeymapCapture => {}
        Action::AppendKeymapCapture(stroke) => {
            if let Some(capture) = app.keymap_preferences_ui.capture.as_mut() {
                if capture.strokes.len() < MAX_KEY_SEQUENCE_STROKES {
                    capture.strokes.push(stroke);
                    app.keymap_preferences_ui.validation_error = None;
                } else {
                    app.keymap_preferences_ui.validation_error = Some(format!(
                        "A key sequence may contain at most {MAX_KEY_SEQUENCE_STROKES} strokes."
                    ));
                }
            }
        }
        Action::BackspaceKeymapCapture => {
            if let Some(capture) = app.keymap_preferences_ui.capture.as_mut() {
                capture.strokes.pop();
                app.keymap_preferences_ui.validation_error = None;
            }
        }
        Action::AppendInternalLogQuery(character) if app.internal_logs.searching => {
            let selected_id = app.internal_logs.selected().map(|entry| entry.id);
            app.internal_logs.query.push(character);
            app.internal_logs.reconcile_selection(selected_id);
        }
        Action::BackspaceInternalLogQuery if app.internal_logs.searching => {
            let selected_id = app.internal_logs.selected().map(|entry| entry.id);
            app.internal_logs.query.pop();
            app.internal_logs.reconcile_selection(selected_id);
        }
        Action::AppendLogQuery(character) if app.logs.searching => {
            let selected_id = app.logs.selected().map(|entry| entry.id);
            app.logs.query.push(character);
            app.logs.reconcile_selection(selected_id);
        }
        Action::BackspaceLogQuery if app.logs.searching => {
            let selected_id = app.logs.selected().map(|entry| entry.id);
            app.logs.query.pop();
            app.logs.reconcile_selection(selected_id);
        }
        Action::NextLogMatch if !app.logs.query.is_empty() => {
            let count = app.logs.filtered().count();
            app.logs.follow = false;
            app.logs.paused_len = Some(app.logs.entries.len());
            app.logs.selection = app
                .logs
                .selection
                .saturating_add(1)
                .min(count.saturating_sub(1));
            app.logs.scroll_offset = count.saturating_sub(app.logs.selection.saturating_add(1));
        }
        Action::PreviousLogMatch if !app.logs.query.is_empty() => {
            app.logs.follow = false;
            app.logs.paused_len = Some(app.logs.entries.len());
            app.logs.selection = app.logs.selection.saturating_sub(1);
            let count = app.logs.filtered().count();
            app.logs.scroll_offset = count.saturating_sub(app.logs.selection.saturating_add(1));
        }
        Action::AppendMetadataQuery(character) if app.metadata_searching => {
            app.metadata_query.push(character);
            app.recipe_selection = 0;
            app.layer_selection = 0;
            app.config_selection = 0;
            select_first_matching_layer_entry(app);
            select_first_matching_recipe(app);
        }
        Action::BackspaceMetadataQuery if app.metadata_searching => {
            app.metadata_query.pop();
            app.recipe_selection = 0;
            app.layer_selection = 0;
            app.config_selection = 0;
            select_first_matching_layer_entry(app);
            select_first_matching_recipe(app);
        }
        Action::AppendInternalLogQuery(_)
        | Action::BackspaceInternalLogQuery
        | Action::AppendLogQuery(_)
        | Action::BackspaceLogQuery
        | Action::NextLogMatch
        | Action::PreviousLogMatch
        | Action::AppendCommandPaletteQuery(_)
        | Action::BackspaceCommandPaletteQuery
        | Action::ClearCommandPaletteQuery
        | Action::BeginGlobalContentSearch
        | Action::AppendMetadataQuery(_)
        | Action::BackspaceMetadataQuery => {}
        _ => unreachable!("action routed to the wrong reducer"),
    }
    synchronize_focus(app);
    None
}
