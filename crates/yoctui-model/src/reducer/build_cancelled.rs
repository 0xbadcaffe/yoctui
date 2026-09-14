//! State transitions beginning with BuildCancelled.
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
        Action::SelectRecipe { delta } => {
            let matches = app
                .workspace
                .recipes
                .iter()
                .enumerate()
                .filter(|(_, recipe)| recipe_matches_query(recipe, &app.metadata_query))
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            let position = matches
                .iter()
                .position(|index| *index == app.recipe_selection)
                .unwrap_or(0);
            let position = shifted_index(position, delta, matches.len());
            app.recipe_selection = matches.get(position).copied().unwrap_or(0);
            app.recipe_preview_scroll = 0;
        }
        Action::ScrollRecipePreview { delta } => {
            app.recipe_preview_scroll = if delta.is_negative() {
                app.recipe_preview_scroll
                    .saturating_sub(delta.unsigned_abs())
            } else {
                app.recipe_preview_scroll
                    .saturating_add(delta as usize)
                    .min(4_095)
            };
        }
        Action::BeginSelectedRecipeBuild => {
            begin_recipe_task(app, None, false);
        }
        Action::BeginSelectedRecipeClean => {
            begin_recipe_task(app, Some("clean".into()), false);
        }
        Action::BeginSelectedRecipeMenuConfig => {
            begin_terminal_creation(app, Some("menuconfig"));
        }
        Action::BeginSelectedRecipeCleanState => {
            begin_recipe_task(app, Some("cleansstate".into()), false);
        }
        Action::BeginSelectedRecipeDevshell => {
            begin_terminal_creation(app, Some("devshell"));
        }
        Action::BeginSelectedRecipeDevtoolWorkspaceShell => {
            if let Some(request) = devtool_terminal_request(app, false) {
                open_terminal_launch(app, request);
            }
        }
        Action::BeginSelectedRecipeDevtoolEditRecipe => {
            if let Some(request) = devtool_terminal_request(app, true) {
                open_terminal_launch(app, request);
            }
        }
        Action::BeginSelectedRecipeDiffconfig => {
            begin_recipe_task(app, Some("diffconfig".into()), false);
        }
        Action::BeginSelectedRecipeDiffsigs => {
            begin_recipe_task(app, Some("diffsigs".into()), false);
        }
        Action::BeginSelectedRecipeSignatures => {
            let identity = match selected_recipe_identity(app) {
                Ok(identity) => identity,
                Err(message) => {
                    app.notification = Some(message.replace("Devtool status", "signatures"));
                    return None;
                }
            };
            let Some(tasks) = app
                .recipe_metadata
                .get(&identity.name)
                .and_then(|metadata| metadata.tasks.as_ref())
            else {
                app.notification = Some(
                    "Load authoritative recipe tasks with Enter before inspecting signatures."
                        .into(),
                );
                return None;
            };
            let mut tasks = tasks
                .iter()
                .filter(|task| {
                    SignatureTarget {
                        recipe: identity.name.clone(),
                        task: (*task).clone(),
                    }
                    .validate()
                    .is_ok()
                })
                .cloned()
                .collect::<Vec<_>>();
            tasks.sort();
            tasks.dedup();
            if tasks.is_empty() {
                app.notification =
                    Some("BitBake reported no valid signature tasks for this recipe.".into());
                return None;
            }
            open_dialog(
                app,
                Dialog::SignatureTaskPicker(SignatureTaskPicker {
                    recipe: identity,
                    tasks,
                    selection: 0,
                }),
            );
        }
        Action::BeginSelectedRecipeCveCheck => {
            begin_recipe_task(app, Some("cve_check".into()), false);
        }
        Action::BeginSelectedRecipeSpdx => {
            begin_recipe_task(app, Some("create_spdx".into()), false);
        }
        Action::BeginSelectedRecipeTask { task, force } => {
            begin_recipe_task(app, task, force);
        }
        Action::BeginSelectedRecipeForceTask => {
            let Some(recipe) = app.workspace.recipes.get(app.recipe_selection) else {
                app.notification = Some("No recipe is selected for forced task execution.".into());
                return None;
            };
            let Some(tasks) = app
                .recipe_metadata
                .get(&recipe.name)
                .and_then(|metadata| metadata.tasks.as_ref())
            else {
                app.notification = Some(
                    "Load authoritative recipe tasks with Enter before forcing a task.".into(),
                );
                return None;
            };
            let mut tasks = tasks
                .iter()
                .map(|task| task.strip_prefix("do_").unwrap_or(task).to_owned())
                .filter(|task| {
                    !task.is_empty()
                        && task.chars().all(|character| {
                            character.is_ascii_alphanumeric()
                                || matches!(character, '-' | '_' | '.' | '+')
                        })
                })
                .collect::<Vec<_>>();
            tasks.sort();
            tasks.dedup();
            if tasks.is_empty() {
                app.notification =
                    Some("BitBake reported no forceable tasks for this recipe.".into());
            } else {
                open_dialog(
                    app,
                    Dialog::RecipeTaskPicker(RecipeTaskPicker {
                        recipe: recipe.name.clone(),
                        tasks,
                        selection: 0,
                        force: true,
                    }),
                );
            }
        }
        Action::SelectRecipeTask { delta } => {
            if let Some(Dialog::RecipeTaskPicker(picker)) = app.active_dialog_mut() {
                picker.selection = if delta.is_negative() {
                    picker.selection.saturating_sub(delta.unsigned_abs())
                } else {
                    picker
                        .selection
                        .saturating_add(delta as usize)
                        .min(picker.tasks.len().saturating_sub(1))
                };
            }
        }
        Action::PreviewSelectedRecipeTask => {
            if let Some(Dialog::RecipeTaskPicker(picker)) = app.active_dialog()
                && let Some(task) = picker.tasks.get(picker.selection)
            {
                let request = BuildRequest {
                    targets: vec![picker.recipe.clone()],
                    task: Some(task.clone()),
                    force: picker.force,
                };
                replace_dialog(app, Dialog::RecipeTaskConfirmation(request));
            }
        }
        Action::CancelRecipeTaskPicker => {
            if matches!(app.active_dialog(), Some(Dialog::RecipeTaskPicker(_))) {
                close_dialog(app);
            }
        }
        Action::SelectSignatureTask { delta } => {
            if let Some(Dialog::SignatureTaskPicker(picker)) = app.active_dialog_mut() {
                picker.selection = if delta.is_negative() {
                    picker.selection.saturating_sub(delta.unsigned_abs())
                } else {
                    picker
                        .selection
                        .saturating_add(delta as usize)
                        .min(picker.tasks.len().saturating_sub(1))
                };
            }
        }
        Action::ConfirmSignatureTask => {
            let Some(Dialog::SignatureTaskPicker(picker)) = app.active_dialog().cloned() else {
                return None;
            };
            let Some(task) = picker.tasks.get(picker.selection).cloned() else {
                app.notification = Some("No authoritative signature task is selected.".into());
                return None;
            };
            let target = SignatureTarget {
                recipe: picker.recipe.name.clone(),
                task,
            };
            if let Err(message) = target.validate() {
                app.notification = Some(message.into());
                return None;
            }
            close_dialog(app);
            app.screen = Screen::Signatures;
            app.focus = FocusTarget::Workspace;
            app.focus_return = None;
            app.signature_recipe = Some(picker.recipe);
            app.signature_selection = None;
            app.signature_comparison = SignatureComparisonState::NotSelected;
            return begin_signature_dump(app, target);
        }
        Action::CancelSignatureTaskPicker => {
            if matches!(app.active_dialog(), Some(Dialog::SignatureTaskPicker(_))) {
                close_dialog(app);
            }
        }
        Action::OpenSelectedRecipeProvider => {
            let Some(recipe) = app.workspace.recipes.get(app.recipe_selection) else {
                app.notification = Some("No recipe is selected to open.".into());
                return None;
            };
            if let Some(path) = recipe.file.clone() {
                return Some(Effect::OpenInEditor(path));
            }
            app.notification = Some(format!(
                "BitBake did not report an authoritative provider path for {}.",
                recipe.name
            ));
        }
        Action::BeginSelectedRecipeTaskLog => {
            let Some(recipe) = app.workspace.recipes.get(app.recipe_selection) else {
                app.notification = Some("No recipe is selected for task-log inspection.".into());
                return None;
            };
            let recipe_name = recipe.name.clone();
            let mut logs = app
                .tasks
                .values()
                .chain(app.completed_tasks.iter().map(|completed| &completed.task))
                .filter(|task| task.recipe == recipe_name)
                .filter_map(|task| {
                    task.log_path.clone().map(|path| RecipeTaskLogChoice {
                        task: task.task.clone(),
                        state: task.state,
                        path,
                    })
                })
                .collect::<Vec<_>>();
            logs.sort_by(|left, right| {
                left.task
                    .cmp(&right.task)
                    .then_with(|| left.path.cmp(&right.path))
            });
            logs.dedup_by(|left, right| left.path == right.path);
            match logs.len() {
                0 => {
                    app.notification = Some(format!(
                        "No retained task log path is available for {recipe_name}; BitBake may not have reported one or it may have been evicted."
                    ));
                }
                1 => return Some(Effect::OpenInEditor(logs.remove(0).path)),
                _ => open_dialog(
                    app,
                    Dialog::RecipeTaskLogPicker(RecipeTaskLogPicker {
                        recipe: recipe_name,
                        logs,
                        selection: 0,
                    }),
                ),
            }
        }
        Action::SelectRecipeTaskLog { delta } => {
            if let Some(Dialog::RecipeTaskLogPicker(picker)) = app.active_dialog_mut() {
                picker.selection = if delta.is_negative() {
                    picker.selection.saturating_sub(delta.unsigned_abs())
                } else {
                    picker
                        .selection
                        .saturating_add(delta as usize)
                        .min(picker.logs.len().saturating_sub(1))
                };
            }
        }
        Action::OpenSelectedRecipeTaskLog => {
            if let Some(Dialog::RecipeTaskLogPicker(picker)) = app.active_dialog()
                && let Some(path) = picker
                    .logs
                    .get(picker.selection)
                    .map(|choice| choice.path.clone())
            {
                close_dialog(app);
                return Some(Effect::OpenInEditor(path));
            }
        }
        Action::CancelRecipeTaskLogPicker => {
            if matches!(app.active_dialog(), Some(Dialog::RecipeTaskLogPicker(_))) {
                close_dialog(app);
            }
        }
        Action::BeginSelectedRecipePatchReview => {
            let Some(recipe) = app.workspace.recipes.get(app.recipe_selection) else {
                app.notification = Some("No recipe is selected for patch review.".into());
                return None;
            };
            let recipe_name = recipe.name.clone();
            let Some(patches) = app
                .recipe_metadata
                .get(&recipe_name)
                .and_then(|metadata| metadata.patches.as_ref())
            else {
                app.notification = Some(format!(
                    "Load authoritative metadata for {recipe_name} with Enter before reviewing patches."
                ));
                return None;
            };
            let mut local_patches = patches
                .iter()
                .map(PathBuf::from)
                .filter(|path| path.is_absolute())
                .collect::<Vec<_>>();
            local_patches.sort();
            local_patches.dedup();
            if local_patches.is_empty() {
                app.notification = Some(if patches.is_empty() {
                    format!("BitBake reported no patches for {recipe_name}.")
                } else {
                    format!(
                        "The patches for {recipe_name} are remote or unresolved; no authoritative local path is available."
                    )
                });
            } else if local_patches.len() == 1 {
                return Some(Effect::OpenInEditor(local_patches.remove(0)));
            } else {
                open_dialog(
                    app,
                    Dialog::RecipePatchPicker(RecipePatchPicker {
                        recipe: recipe_name,
                        patches: local_patches,
                        selection: 0,
                    }),
                );
            }
        }
        Action::SelectRecipePatch { delta } => {
            if let Some(Dialog::RecipePatchPicker(picker)) = app.active_dialog_mut() {
                picker.selection = if delta.is_negative() {
                    picker.selection.saturating_sub(delta.unsigned_abs())
                } else {
                    picker
                        .selection
                        .saturating_add(delta as usize)
                        .min(picker.patches.len().saturating_sub(1))
                };
            }
        }
        Action::OpenSelectedRecipePatch => {
            if let Some(Dialog::RecipePatchPicker(picker)) = app.active_dialog()
                && let Some(path) = picker.patches.get(picker.selection).cloned()
            {
                close_dialog(app);
                return Some(Effect::OpenInEditor(path));
            }
        }
        Action::CancelRecipePatchPicker => {
            if matches!(app.active_dialog(), Some(Dialog::RecipePatchPicker(_))) {
                close_dialog(app);
            }
        }
        Action::BeginSelectedRecipeDevtoolModify => {
            let identity = match selected_recipe_identity(app) {
                Ok(identity) => identity,
                Err(message) => {
                    app.notification = Some(message.into());
                    return None;
                }
            };
            let Some(status) = app.devtool_statuses.get(&identity) else {
                app.notification =
                    Some("Refresh authoritative Devtool status with t before modifying.".into());
                return None;
            };
            if let Some(reason) = status.disabled_reason(DevtoolAction::ModifyOrEdit) {
                app.notification = Some(reason);
                return None;
            }
            if let DevtoolWorkspace::Present { source_path, .. } = &status.workspace {
                return Some(Effect::OpenWorkspaceEditor {
                    label: identity.name,
                    root: source_path.clone(),
                });
            }
            open_dialog(app, Dialog::DevtoolModifyConfirmation(identity));
        }
        Action::BeginSelectedRecipeDevtoolStatus => match selected_recipe_identity(app) {
            Ok(identity) => {
                app.devtool_status_loading.insert(identity.clone());
                return Some(Effect::InspectDevtoolStatus(identity));
            }
            Err(message) => app.notification = Some(message.into()),
        },
        Action::DevtoolStatusLoaded(status) => {
            app.devtool_status_loading.remove(&status.identity);
            app.devtool_statuses.insert(status.identity.clone(), status);
        }
        Action::BeginSelectedRecipeDevtoolReset => {
            let identity = match selected_recipe_identity(app) {
                Ok(identity) => identity,
                Err(message) => {
                    app.notification = Some(message.into());
                    return None;
                }
            };
            let Some(status) = app.devtool_statuses.get(&identity) else {
                app.notification =
                    Some("Refresh authoritative Devtool status with t before reset.".into());
                return None;
            };
            if let Some(reason) = status.disabled_reason(DevtoolAction::Reset) {
                app.notification = Some(reason);
                return None;
            }
            let source_path = match &status.workspace {
                DevtoolWorkspace::Present { source_path, .. }
                | DevtoolWorkspace::MissingDirectory { source_path } => source_path.clone(),
                DevtoolWorkspace::NotMember => return None,
            };
            if !source_path.is_absolute() {
                app.notification =
                    Some("The authoritative Devtool reset source path is not absolute.".into());
                return None;
            }
            open_dialog(
                app,
                Dialog::DevtoolResetConfirmation(DevtoolResetPlan {
                    identity,
                    source_path,
                }),
            );
        }
        _ => unreachable!("action routed to the wrong reducer"),
    }
    synchronize_focus(app);
    None
}
