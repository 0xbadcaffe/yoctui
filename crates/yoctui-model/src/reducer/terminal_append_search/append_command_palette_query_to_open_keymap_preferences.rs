use super::*;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match action {
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
        _ => unreachable!("action routed to the wrong reducer"),
    }
    synchronize_focus(app);
    None
}
