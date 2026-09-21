impl App {
    pub fn inspector_mode(&self) -> InspectorMode {
        if self.focus == FocusTarget::Navigator {
            return InspectorMode::Navigator;
        }
        match self.screen {
            Screen::Dashboard | Screen::Insights => InspectorMode::DaemonSession,
            Screen::Tasks => InspectorMode::Task,
            Screen::BuildHistory => InspectorMode::Job,
            Screen::Dependencies | Screen::LayerRelationships => InspectorMode::Dependency,
            Screen::Signatures => InspectorMode::Signature,
            Screen::Recipes => InspectorMode::Recipe,
            Screen::Packages => InspectorMode::Package,
            Screen::Images | Screen::Kernel | Screen::Firmware | Screen::Sdk => {
                InspectorMode::Artifact
            }
            Screen::Testing => InspectorMode::Test,
            Screen::Security => InspectorMode::Security,
            Screen::Qa => InspectorMode::Qa,
            Screen::Layers => {
                if self
                    .layer_browser
                    .as_ref()
                    .and_then(LayerBrowser::selected_entry)
                    .is_some_and(|entry| !entry.is_dir)
                {
                    InspectorMode::File
                } else {
                    InspectorMode::Layer
                }
            }
            Screen::Configuration | Screen::Bbmask => InspectorMode::Configuration,
            Screen::RawMode => InspectorMode::RawCommand,
            Screen::TerminalSessions => InspectorMode::DaemonSession,
            Screen::Maintenance => InspectorMode::Utility,
            Screen::Logs => InspectorMode::Log,
            Screen::Errors => InspectorMode::Error,
            Screen::Help => InspectorMode::Help,
            Screen::BuildEnvironment => InspectorMode::BuildEnvironment,
            Screen::Compatibility => InspectorMode::CompatibilityCapability,
            Screen::Settings => InspectorMode::Settings,
        }
    }
    pub fn task_inspector<'a>(
        &'a self,
        row: Option<TaskRowRef<'a>>,
        recent_log_limit: usize,
    ) -> TaskInspectorRef<'a> {
        match row {
            None => TaskInspectorRef::None,
            Some(TaskRowRef::WaitingSummary(count)) => TaskInspectorRef::Waiting { count },
            Some(TaskRowRef::Task { task, state }) => {
                let version = self
                    .workspace
                    .recipes
                    .iter()
                    .find(|recipe| recipe.name == task.recipe)
                    .and_then(|recipe| recipe.version.as_deref());
                let mut recent_logs = self
                    .logs
                    .entries
                    .iter()
                    .rev()
                    .filter(|entry| {
                        entry.recipe.as_deref() == Some(task.recipe.as_str())
                            && entry.task.as_deref() == Some(task.task.as_str())
                    })
                    .take(recent_log_limit)
                    .collect::<Vec<_>>();
                recent_logs.reverse();
                TaskInspectorRef::Task {
                    task,
                    state,
                    version,
                    revision: None,
                    workdir: None,
                    recent_logs,
                }
            }
        }
    }
    pub fn invalidate_task_projection(&mut self) {
        self.task_projection_generation = self.task_projection_generation.wrapping_add(1);
    }
    pub fn task_projection_rebuilds(&self) -> u64 {
        self.task_projection_cache.0.borrow().rebuilds
    }
    pub fn visible_task_row_refs_at(&self, now: SystemTime) -> Vec<TaskRowRef<'_>> {
        let duration_second = self.task_filters.minimum_duration.map(|_| {
            now.duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        });
        let waiting = self.waiting_task_count();
        let cache_current = {
            let cache = self.task_projection_cache.0.borrow();
            cache.generation == self.task_projection_generation
                && cache.filters == self.task_filters
                && cache.duration_second == duration_second
                && cache.active_len == self.tasks.len()
                && cache.completed_len == self.completed_tasks.len()
                && cache.waiting == waiting
        };
        if !cache_current {
            self.rebuild_task_projection(now, duration_second, waiting);
        }
        let cache = self.task_projection_cache.0.borrow();
        cache
            .rows
            .iter()
            .filter_map(|key| match key {
                TaskProjectionKey::Active(id, state) => {
                    self.tasks.get(id).map(|task| TaskRowRef::Task {
                        task,
                        state: *state,
                    })
                }
                TaskProjectionKey::Completed(index, state) => {
                    self.completed_tasks
                        .get(*index)
                        .map(|completed| TaskRowRef::Task {
                            task: &completed.task,
                            state: *state,
                        })
                }
                TaskProjectionKey::WaitingSummary(count) => {
                    Some(TaskRowRef::WaitingSummary(*count))
                }
            })
            .collect()
    }
    pub(crate) fn rebuild_task_projection(
        &self,
        now: SystemTime,
        duration_second: Option<u64>,
        waiting: usize,
    ) {
        let state_matches = |state: TaskState| match self.task_filters.state {
            TaskStateFilter::All => true,
            TaskStateFilter::Active => state == TaskState::Active,
            TaskStateFilter::Waiting => matches!(state, TaskState::Queued | TaskState::Waiting),
            TaskStateFilter::Completed => state == TaskState::Completed,
            TaskStateFilter::Failed => {
                matches!(
                    state,
                    TaskState::Failed | TaskState::Cancelled | TaskState::Lost
                )
            }
        };
        let text_matches = |task: &TaskInfo| {
            contains_case_insensitive(&task.recipe, &self.task_filters.recipe)
                && contains_case_insensitive(&task.task, &self.task_filters.task)
                && contains_case_insensitive(
                    task.worker.as_deref().unwrap_or(""),
                    &self.task_filters.worker,
                )
                && self.task_filters.minimum_duration.is_none_or(|minimum| {
                    task.elapsed_at(now)
                        .is_some_and(|elapsed| elapsed >= minimum)
                })
        };
        let retained = self
            .tasks
            .values()
            .map(|task| {
                (
                    TaskProjectionKey::Active(task.id.clone(), task.state),
                    task,
                    task.state,
                )
            })
            .chain(
                self.completed_tasks
                    .iter()
                    .enumerate()
                    .map(|(index, completed)| {
                        let state = if matches!(
                            completed.task.state,
                            TaskState::Queued | TaskState::Active | TaskState::Waiting
                        ) {
                            if completed.success {
                                TaskState::Completed
                            } else {
                                TaskState::Failed
                            }
                        } else {
                            completed.task.state
                        };
                        (
                            TaskProjectionKey::Completed(index, state),
                            &completed.task,
                            state,
                        )
                    }),
            );
        let mut rows = retained
            .filter(|(_, task, state)| state_matches(*state) && text_matches(task))
            .collect::<Vec<_>>();
        rows.sort_unstable_by(|left, right| {
            let (_, left, left_state) = left;
            let (_, right, right_state) = right;
            (
                left.started.is_none(),
                left.started,
                task_state_order(*left_state),
                left.recipe.as_str(),
                left.task.as_str(),
                left.id.0.as_str(),
            )
                .cmp(&(
                    right.started.is_none(),
                    right.started,
                    task_state_order(*right_state),
                    right.recipe.as_str(),
                    right.task.as_str(),
                    right.id.0.as_str(),
                ))
        });
        let waiting_filter_matches = matches!(
            self.task_filters.state,
            TaskStateFilter::All | TaskStateFilter::Waiting
        ) && self.task_filters.recipe.is_empty()
            && self.task_filters.task.is_empty()
            && self.task_filters.worker.is_empty()
            && self.task_filters.minimum_duration.is_none();
        let mut projection = rows.into_iter().map(|(key, _, _)| key).collect::<Vec<_>>();
        if waiting > 0 && waiting_filter_matches {
            projection.push(TaskProjectionKey::WaitingSummary(waiting));
        }
        let mut cache = self.task_projection_cache.0.borrow_mut();
        cache.generation = self.task_projection_generation;
        cache.filters = self.task_filters.clone();
        cache.duration_second = duration_second;
        cache.active_len = self.tasks.len();
        cache.completed_len = self.completed_tasks.len();
        cache.waiting = waiting;
        cache.rows = projection;
        cache.rebuilds = cache.rebuilds.saturating_add(1);
    }
    pub fn visible_task_rows(&self) -> Vec<TaskRow> {
        self.visible_task_row_refs_at(SystemTime::now())
            .into_iter()
            .map(TaskRowRef::into_owned)
            .collect()
    }
    pub fn selected_task_row(&self) -> Option<TaskRow> {
        self.visible_task_row_refs_at(SystemTime::now())
            .get(self.task_progress_scroll)
            .copied()
            .map(TaskRowRef::into_owned)
    }
}
