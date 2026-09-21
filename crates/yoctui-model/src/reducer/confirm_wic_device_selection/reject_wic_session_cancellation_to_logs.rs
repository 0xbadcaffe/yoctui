use super::*;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match action {
        Action::RejectWicSessionCancellation { id, message } => {
            let Some(job_id) = wic_job_id(app, id) else {
                note_stale_wic_event(app);
                return None;
            };
            app.background_jobs
                .update_if(job_id, &[BackgroundJobStatus::Cancelling], |job| {
                    job.status = BackgroundJobStatus::Running;
                });
            app.notification = Some(format!("Wic cancellation failed: {message}"));
        }
        Action::CancelWicSession {
            id,
            exit_code,
            finished_at,
        } => {
            if !matches!(
                wic_job_id(app, id).and_then(|job_id| app.background_jobs.get(job_id)),
                Some(BackgroundJob {
                    status: BackgroundJobStatus::Cancelling,
                    ..
                })
            ) {
                note_stale_wic_event(app);
                return None;
            }
            let is_device_write = matches!(
                app.wic_session(id).map(|session| &session.operation),
                Some(WicOperation::Write(_))
            );
            let Some(job_id) = mutate_wic_session(app, id, |session| session.exit_code = exit_code)
            else {
                note_stale_wic_event(app);
                return None;
            };
            app.background_jobs
                .update_if(job_id, &[BackgroundJobStatus::Cancelling], |job| {
                    job.status = BackgroundJobStatus::Cancelled;
                    job.finished_at = Some(finished_at);
                    if is_device_write {
                        job.error = Some(BackgroundJobError {
                            summary: "Wic device write cancelled".into(),
                            detail: Some("The target device may be incomplete.".into()),
                        });
                    }
                });
        }
        Action::BeginBuildTargetEdit => {
            let mut editor = PopupEditor::new(popup_toml_document(
                "target",
                app.build.target.as_deref().unwrap_or_default(),
                None,
            ));
            let _ = editor.select_toml_value("target");
            replace_dialog(app, Dialog::BuildTarget { editor, task: None });
        }
        Action::BeginBuildTargetTask(task) => {
            let mut editor = PopupEditor::new(popup_toml_document(
                "target",
                app.build.target.as_deref().unwrap_or_default(),
                None,
            ));
            let _ = editor.select_toml_value("target");
            replace_dialog(app, Dialog::BuildTarget { editor, task });
        }
        Action::ToggleBuildTargetEdit => {
            if let Some(Dialog::BuildTarget { editor, .. }) = app.active_dialog_mut() {
                editor.editing = !editor.editing;
            }
        }
        Action::AppendBuildTarget(character) => {
            if let Some(Dialog::BuildTarget { editor, .. }) = app.active_dialog_mut()
                && editor.editing
            {
                editor.insert(&character.to_string());
            }
        }
        Action::BackspaceBuildTarget => {
            if let Some(Dialog::BuildTarget { editor, .. }) = app.active_dialog_mut()
                && editor.editing
            {
                editor.backspace();
            }
        }
        Action::CancelBuildTargetEdit => {
            if matches!(app.active_dialog(), Some(Dialog::BuildTarget { .. })) {
                close_dialog(app);
            }
        }
        Action::ConfirmBuildTarget => {
            if let Some(Dialog::BuildTarget { editor, task }) = app.active_dialog() {
                let input = match popup_toml_value(&editor.text, "target") {
                    Ok(value) => value,
                    Err(reason) => {
                        app.notification = Some(reason);
                        return None;
                    }
                };
                let request = BuildRequest {
                    targets: vec![input],
                    task: task.clone(),
                    force: false,
                };
                if let Err(error) = request.validate() {
                    app.notification = Some(error.to_string());
                } else {
                    replace_dialog(app, Dialog::RecipeTaskConfirmation(request));
                }
            }
        }
        Action::Start(r) => {
            if !app.build_environment.connected() {
                app.notification = Some("Configure and verify a BitBake environment first".into());
            } else if let Err(e) = r.validate() {
                app.notification = Some(e.to_string())
            } else {
                prepare_build(app, r.targets.first().cloned());
                return Some(Effect::Start(r));
            }
        }
        Action::QueueBackgroundJob(spec) => app.background_jobs.queue(spec),
        Action::StartBackgroundJob { id, started_at } => {
            app.background_jobs
                .update_if(id, &[BackgroundJobStatus::Queued], |job| {
                    job.status = BackgroundJobStatus::Starting;
                    job.started_at = Some(started_at);
                })
        }
        Action::RunBackgroundJob { id } => {
            app.background_jobs
                .update_if(id, &[BackgroundJobStatus::Starting], |job| {
                    job.status = BackgroundJobStatus::Running
                })
        }
        Action::UpdateBackgroundJobProgress { id, progress } => {
            if progress.is_valid() {
                app.background_jobs
                    .update_if(id, &[BackgroundJobStatus::Running], |job| {
                        job.progress = progress
                    });
            } else {
                app.background_jobs.ignored_transitions += 1;
            }
        }
        Action::AppendBackgroundJobOutput { id, entry } => {
            app.background_jobs.append_output(id, entry);
        }
        Action::RequestBackgroundJobCancellation { id } => {
            app.background_jobs.request_cancellation(id);
        }
        Action::RejectBackgroundJobCancellation { id } => {
            app.background_jobs
                .update_if(id, &[BackgroundJobStatus::Cancelling], |job| {
                    job.status = BackgroundJobStatus::Running
                })
        }
        Action::SucceedBackgroundJob {
            id,
            result,
            finished_at,
        } => app
            .background_jobs
            .update_if(id, &[BackgroundJobStatus::Running], |job| {
                job.status = BackgroundJobStatus::Succeeded;
                job.finished_at = Some(finished_at);
                job.result = Some(result);
            }),
        Action::FailBackgroundJob {
            id,
            error,
            finished_at,
        } => app.background_jobs.update_if(
            id,
            &[
                BackgroundJobStatus::Starting,
                BackgroundJobStatus::Running,
                BackgroundJobStatus::Cancelling,
            ],
            |job| {
                job.status = BackgroundJobStatus::Failed;
                job.finished_at = Some(finished_at);
                job.error = Some(error);
            },
        ),
        Action::CancelBackgroundJob { id, finished_at } => {
            app.background_jobs
                .update_if(id, &[BackgroundJobStatus::Cancelling], |job| {
                    job.status = BackgroundJobStatus::Cancelled;
                    job.finished_at = Some(finished_at);
                })
        }
        Action::LoseBackgroundJob {
            id,
            error,
            finished_at,
        } => app.background_jobs.update_if(
            id,
            &[
                BackgroundJobStatus::Starting,
                BackgroundJobStatus::Running,
                BackgroundJobStatus::Cancelling,
            ],
            |job| {
                job.status = BackgroundJobStatus::Lost;
                job.finished_at = Some(finished_at);
                job.error = Some(error);
            },
        ),
        Action::BuildRequested { target } => prepare_build(app, target),
        Action::SstateSummary(summary) => {
            if summary.valid() {
                app.build.cache.summary = Some(summary);
            }
        }
        Action::BuildStarted => {
            app.build.status = BuildStatus::Running;
            app.build.started = Some(SystemTime::now());
            app.build.parse_current = None;
            app.build.parse_total = None;
        }
        Action::TaskStats(stats) => {
            mark_build_running_from_task_activity(app);
            app.build.completed = app.build.completed.max(stats.completed);
            app.build.total = (stats.total > 0).then_some(stats.total);
            app.invalidate_task_projection();
        }
        Action::ParseProgress { current, total } => {
            if matches!(
                app.build.status,
                BuildStatus::LoadingWorkspace | BuildStatus::Parsing | BuildStatus::Running
            ) && app.tasks.is_empty()
                && app.build.completed == 0
            {
                app.build.status = BuildStatus::Parsing;
                app.build.parse_current = current;
                app.build.parse_total = total;
            }
        }
        Action::TaskStarted(t) => {
            apply_task_event(app, TaskEvent::Started(t));
            clamp_task_selection(app);
        }
        Action::TaskQueued(t) => {
            apply_task_event(app, TaskEvent::Queued(t));
            clamp_task_selection(app);
        }
        Action::TaskProgress { id, progress } => {
            app.task_progress_events = app.task_progress_events.saturating_add(1);
            apply_task_event(app, TaskEvent::Progress { id, progress });
        }
        Action::TaskCompleted { id, success } => {
            apply_task_event(app, TaskEvent::Completed { id, success });
            clamp_task_selection(app);
        }
        Action::TaskEvents(events) => apply_task_batch(app, events),
        Action::ScrollBuildTasks { delta } => {
            let task_count = app.visible_task_rows().len();
            app.task_progress_scroll = shifted_index(app.task_progress_scroll, delta, task_count);
        }
        Action::CycleTaskStateFilter => {
            app.task_filters.state = app.task_filters.state.next();
            clamp_task_selection(app);
        }
        Action::CycleTaskFilterField => {
            app.task_filter_field = app.task_filter_field.next();
        }
        Action::BeginTaskFilterEdit => {
            app.task_filter_editing = true;
        }
        Action::AppendTaskFilter(character) => {
            if app.task_filter_editing {
                match app.task_filter_field {
                    TaskFilterField::Recipe => app.task_filters.recipe.push(character),
                    TaskFilterField::Task => app.task_filters.task.push(character),
                    TaskFilterField::Worker => app.task_filters.worker.push(character),
                }
                clamp_task_selection(app);
            }
        }
        Action::BackspaceTaskFilter => {
            if app.task_filter_editing {
                match app.task_filter_field {
                    TaskFilterField::Recipe => app.task_filters.recipe.pop(),
                    TaskFilterField::Task => app.task_filters.task.pop(),
                    TaskFilterField::Worker => app.task_filters.worker.pop(),
                };
                clamp_task_selection(app);
            }
        }
        Action::FinishTaskFilterEdit => {
            app.task_filter_editing = false;
        }
        Action::CycleTaskDurationFilter => {
            app.task_filters.minimum_duration = match app.task_filters.minimum_duration {
                None => Some(Duration::from_secs(1)),
                Some(duration) if duration == Duration::from_secs(1) => {
                    Some(Duration::from_secs(10))
                }
                Some(duration) if duration == Duration::from_secs(10) => {
                    Some(Duration::from_secs(60))
                }
                Some(_) => None,
            };
            clamp_task_selection(app);
        }
        Action::Log(entry) => insert_log_entry(app, entry),
        Action::Logs(entries) => insert_log_batch(app, entries),
        _ => unreachable!("action routed to the wrong reducer"),
    }
    synchronize_focus(app);
    None
}
