use super::*;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match action {
        Action::BuildCancelled { exit_code } => {
            archive_unfinished_tasks(app, TaskState::Cancelled, Some("cancelled"));
            app.build.status = BuildStatus::Cancelled;
            app.build.exit_code = exit_code;
            insert_system_log(
                app,
                Severity::Warning,
                format!(
                    "Build cancelled with exit code {}",
                    exit_code.map_or_else(|| "unknown".into(), |code| code.to_string())
                ),
            );
            app.build_history.push_back(BuildRecord {
                target: app.build.target.clone(),
                success: false,
                exit_code,
                elapsed: app.elapsed(),
                completed_tasks: app.build.completed,
                warnings: app.build.warnings,
                errors: app.build.errors,
            });
            if app.build_history.len() > MAX_BUILD_HISTORY {
                app.build_history.pop_front();
            }
            app.build_history_selection = 0;
            clamp_task_selection(app);
            enqueue_build_completion(app);
            app.notification =
                Some("Build was cancelled; this is distinct from a build failure.".into());
        }
        Action::BuildCancellationRejected(message) => {
            if app.build.status == BuildStatus::Cancelling {
                app.build.status = BuildStatus::Running;
            }
            for task in app.tasks.values_mut() {
                task.cancellation = None;
            }
            insert_system_log(
                app,
                Severity::Warning,
                format!("Build cancellation was rejected: {message}"),
            );
            app.notification = Some(format!(
                "Could not cancel the active build: {message}. The build may still be running."
            ));
        }
        Action::DismissBuildCompletion => {
            if matches!(app.active_dialog(), Some(Dialog::BuildCompletion)) {
                close_dialog(app);
            }
        }
        Action::OpenBuildCompletionErrors => {
            if matches!(app.active_dialog(), Some(Dialog::BuildCompletion)) {
                close_dialog(app);
            }
            app.screen = Screen::Errors;
            app.error_selection = app.logs.diagnostics().count().saturating_sub(1);
            app.notification = None;
        }
        Action::SelectBuildHistory { delta } => {
            let count = app.job_history_rows().len();
            app.build_history_selection = if delta.is_negative() {
                app.build_history_selection
                    .saturating_sub(delta.unsigned_abs())
            } else {
                app.build_history_selection
                    .saturating_add(delta as usize)
                    .min(count.saturating_sub(1))
            };
        }
        Action::Cancel => {
            if matches!(
                app.build.status,
                BuildStatus::Running | BuildStatus::Parsing
            ) {
                open_dialog(app, Dialog::BuildCancellationConfirmation);
            }
        }
        Action::ConfirmBuildCancellation => {
            if matches!(
                app.active_dialog(),
                Some(Dialog::BuildCancellationConfirmation)
            ) && matches!(
                app.build.status,
                BuildStatus::Running | BuildStatus::Parsing
            ) {
                close_dialog(app);
                app.build.status = BuildStatus::Cancelling;
                for task in app.tasks.values_mut() {
                    task.cancellation = Some("cancellation requested".into());
                }
                insert_system_log(
                    app,
                    Severity::Warning,
                    "Build cancellation requested".into(),
                );
                return Some(Effect::Cancel);
            }
        }
        Action::CancelBuildCancellation => {
            if matches!(
                app.active_dialog(),
                Some(Dialog::BuildCancellationConfirmation)
            ) {
                close_dialog(app);
            }
        }
        Action::CycleLogWorkspaceView => {
            app.log_workspace_view = match app.log_workspace_view {
                LogWorkspaceView::BitBake => LogWorkspaceView::Yoctui,
                LogWorkspaceView::Yoctui => LogWorkspaceView::BitBake,
            };
            app.logs.searching = false;
            app.internal_logs.searching = false;
        }
        Action::InternalLog(record) => app.internal_logs.insert(record),
        Action::InternalLogIngressDropped(count) => app.internal_logs.note_ingress_dropped(count),
        Action::ToggleInternalLogFollow => app.internal_logs.toggle_follow(),
        Action::ScrollInternalLogs { delta } => app.internal_logs.scroll(delta),
        Action::BeginInternalLogSearch => {
            app.internal_logs.searching = true;
            app.internal_logs.follow = false;
            app.internal_logs.paused_len = Some(app.internal_logs.entries.len());
        }
        Action::ClearInternalLogQuery => {
            let selected_id = app.internal_logs.selected().map(|entry| entry.id);
            app.internal_logs.query.clear();
            app.internal_logs.reconcile_selection(selected_id);
        }
        Action::FinishInternalLogSearch => app.internal_logs.searching = false,
        Action::CycleInternalLogLevelFilter => app.internal_logs.cycle_level_filter(),
        Action::CycleInternalLogTargetFilter => app.internal_logs.cycle_target_filter(),
        Action::ClearInternalLogs => {
            app.internal_logs.clear();
            app.notification =
                Some("Cleared retained Yoctui diagnostics; loss counters remain.".into());
        }
        Action::ExportInternalLogs => {
            let export = format_internal_log_export(&app.internal_logs);
            app.notification = Some(format!(
                "Prepared bounded internal diagnostic export: {} entries, {} omitted.",
                export.included, export.omitted
            ));
            return Some(Effect::CopyToClipboard(export.content));
        }
        Action::ToggleLogFollow => {
            app.logs.follow = !app.logs.follow;
            app.logs.paused_len = (!app.logs.follow).then_some(app.logs.entries.len());
            if app.logs.follow {
                app.logs.selection = app.logs.filtered().count().saturating_sub(1);
                app.logs.scroll_offset = 0;
            }
        }
        Action::ToggleLogWrap => {
            app.logs.wrap = !app.logs.wrap;
            if app.logs.wrap {
                app.logs.horizontal_offset = 0;
            }
        }
        Action::CycleLogSeverity => {
            let selected_id = app.logs.selected().map(|entry| entry.id);
            app.logs.filter = match app.logs.filter {
                None => Some(Severity::Info),
                Some(Severity::Info) => Some(Severity::Warning),
                Some(Severity::Warning) => Some(Severity::Error),
                Some(Severity::Error) | Some(Severity::Trace) => None,
            };
            app.logs.jump_target = None;
            app.logs.reconcile_selection(selected_id);
        }
        Action::ScrollLogs { delta } => {
            app.logs.follow = false;
            app.logs.paused_len = Some(app.logs.entries.len());
            let count = app.logs.filtered().count();
            app.logs.selection = shifted_index(app.logs.selection, delta.saturating_neg(), count);
            app.logs.scroll_offset = count.saturating_sub(app.logs.selection.saturating_add(1));
        }
        Action::BeginLogSearch => {
            app.logs.jump_target = None;
            app.logs.searching = true;
            app.logs.follow = false;
            app.logs.paused_len = Some(app.logs.entries.len());
        }
        Action::ClearLogQuery => {
            let selected_id = app.logs.selected().map(|entry| entry.id);
            app.logs.query.clear();
            app.logs.reconcile_selection(selected_id);
        }
        Action::FinishLogSearch => app.logs.searching = false,
        Action::ScrollLogsHorizontally { delta } => {
            if app.logs.wrap {
                return None;
            }
            app.logs.horizontal_offset = if delta.is_negative() {
                app.logs
                    .horizontal_offset
                    .saturating_sub(delta.unsigned_abs())
            } else {
                let maximum = app.logs.maximum_horizontal_offset();
                app.logs
                    .horizontal_offset
                    .saturating_add(delta as usize)
                    .min(maximum)
            };
        }
        Action::CycleLogRecipeFilter => {
            let selected_id = app.logs.selected().map(|entry| entry.id);
            app.logs.jump_target = None;
            let mut values = app
                .logs
                .entries
                .iter()
                .filter_map(|entry| entry.recipe.clone())
                .collect::<Vec<_>>();
            values.sort();
            values.dedup();
            app.logs.recipe_filter = next_filter(&values, app.logs.recipe_filter.take());
            app.logs.reconcile_selection(selected_id);
        }
        Action::CycleLogTaskFilter => {
            let selected_id = app.logs.selected().map(|entry| entry.id);
            app.logs.jump_target = None;
            let mut values = app
                .logs
                .entries
                .iter()
                .filter_map(|entry| entry.task.clone())
                .collect::<Vec<_>>();
            values.sort();
            values.dedup();
            app.logs.task_filter = next_filter(&values, app.logs.task_filter.take());
            app.logs.reconcile_selection(selected_id);
        }
        Action::CycleLogBuildFilter => {
            let selected_id = app.logs.selected().map(|entry| entry.id);
            app.logs.jump_target = None;
            let mut values = app
                .logs
                .entries
                .iter()
                .filter_map(|entry| entry.build.clone())
                .collect::<Vec<_>>();
            values.sort();
            values.dedup();
            app.logs.build_filter = next_filter(&values, app.logs.build_filter.take());
            app.logs.reconcile_selection(selected_id);
        }
        Action::CycleLogSourceFilter => {
            let selected_id = app.logs.selected().map(|entry| entry.id);
            app.logs.jump_target = None;
            let mut values = app
                .logs
                .entries
                .iter()
                .filter_map(|entry| entry.path.clone())
                .collect::<Vec<_>>();
            values.sort();
            values.dedup();
            app.logs.source_filter = next_filter(&values, app.logs.source_filter.take());
            app.logs.reconcile_selection(selected_id);
        }
        Action::CycleLogTimeRange => {
            let selected_id = app.logs.selected().map(|entry| entry.id);
            app.logs.jump_target = None;
            app.logs.time_range = app.logs.time_range.next();
            app.logs.reconcile_selection(selected_id);
        }
        Action::ToggleSelectedLogBookmark => {
            if !app.logs.toggle_selected_bookmark() {
                app.notification = Some("No retained log entry is selected to bookmark.".into());
            }
        }
        Action::NextLogBookmark => {
            if !app.logs.select_bookmark(true) {
                app.notification = Some("No retained log bookmarks are available.".into());
            }
        }
        Action::PreviousLogBookmark => {
            if !app.logs.select_bookmark(false) {
                app.notification = Some("No retained log bookmarks are available.".into());
            }
        }
        Action::OpenSelectedLogSource => {
            if let Some(path) = app.logs.selected().and_then(|entry| entry.path.clone()) {
                return Some(Effect::OpenInEditor(path));
            }
            app.notification = Some("The selected log entry has no source path.".into());
        }
        Action::CopySelectedLog => {
            if let Some(entry) = app.logs.selected() {
                return Some(Effect::CopyToClipboard(format_log_details_bounded(entry)));
            }
            app.notification = Some("No log entry is selected to copy.".into());
        }
        Action::ExportFilteredLogs => {
            let export = format_log_export(&app.logs);
            app.notification = Some(format!(
                "Prepared bounded log export: {} entries, {} omitted.",
                export.included, export.omitted
            ));
            return Some(Effect::CopyToClipboard(export.content));
        }
        Action::SelectError { delta } => {
            let count = app.logs.diagnostics().count();
            app.error_selection = shifted_index(app.error_selection, delta, count);
        }
        Action::JumpToSelectedError => {
            let id = {
                app.logs
                    .diagnostics()
                    .nth(app.error_selection)
                    .map(|entry| entry.id)
            };
            if let Some(id) = id
                && app.logs.jump_to(id)
            {
                app.screen = Screen::Logs;
            }
        }
        Action::OpenSelectedErrorSource => {
            let selected = app.logs.diagnostics().nth(app.error_selection);
            if let Some(path) = selected.and_then(|entry| entry.path.clone()) {
                return Some(Effect::OpenInEditor(path));
            }
            app.notification = Some("The selected diagnostic has no source log path.".into());
        }
        _ => unreachable!("action routed to the wrong reducer"),
    }
    synchronize_focus(app);
    None
}
