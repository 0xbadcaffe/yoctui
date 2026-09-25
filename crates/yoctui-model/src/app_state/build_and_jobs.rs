impl App {
    pub fn waiting_task_count(&self) -> usize {
        if matches!(
            self.build.status,
            BuildStatus::Completed
                | BuildStatus::Cancelled
                | BuildStatus::Failed
                | BuildStatus::Lost
        ) {
            return 0;
        }
        self.build.total.map_or(0, |total| {
            total.saturating_sub(self.build.completed.saturating_add(self.tasks.len()))
        })
    }
    /// Count observed active workers only when one complete identity namespace is available.
    pub fn active_worker_count(&self) -> Option<usize> {
        if self.daemon.status != ClientReplicaStatus::Current {
            return None;
        }
        match self.build.status {
            BuildStatus::Idle
            | BuildStatus::Completed
            | BuildStatus::Cancelled
            | BuildStatus::Failed => return Some(0),
            BuildStatus::LoadingWorkspace | BuildStatus::Parsing | BuildStatus::Lost => {
                return None;
            }
            BuildStatus::Running | BuildStatus::Cancelling => {}
        }
        let mut pids = std::collections::HashSet::new();
        let mut labels = std::collections::HashSet::new();
        let mut complete_pids = true;
        let mut complete_labels = true;
        let mut observed_active = false;
        for task in self
            .tasks
            .values()
            .filter(|task| task.state == TaskState::Active)
        {
            observed_active = true;
            if let Some(pid) = task.pid.filter(|pid| *pid > 0) {
                pids.insert(pid);
            } else {
                complete_pids = false;
            }
            if let Some(label) = task
                .worker
                .as_deref()
                .filter(|label| !label.trim().is_empty())
            {
                labels.insert(label);
            } else {
                complete_labels = false;
            }
        }
        if !observed_active {
            None
        } else if complete_pids {
            Some(pids.len())
        } else if complete_labels {
            Some(labels.len())
        } else {
            None
        }
    }

    pub fn build_summary_at(&self, now: SystemTime) -> BuildSummary {
        let elapsed = match self.build.status {
            BuildStatus::Completed | BuildStatus::Cancelled | BuildStatus::Failed => {
                self.build_history.back().and_then(|record| record.elapsed)
            }
            BuildStatus::Lost => None,
            BuildStatus::Idle
            | BuildStatus::LoadingWorkspace
            | BuildStatus::Parsing
            | BuildStatus::Running
            | BuildStatus::Cancelling => self
                .build
                .started
                .and_then(|started| now.duration_since(started).ok()),
        };
        BuildSummary {
            completed: self.build.completed,
            total: self.build.total,
            active: self
                .tasks
                .values()
                .filter(|task| task.state == TaskState::Active)
                .count(),
            waiting: self.waiting_task_count(),
            warnings: self.build.warnings,
            errors: self.build.errors,
            elapsed,
        }
    }
    pub fn job_history_rows(&self) -> Vec<JobHistoryRowRef<'_>> {
        let daemon_current = self.daemon.status == ClientReplicaStatus::Current;
        let mut rows = Vec::new();
        if daemon_current {
            rows.extend(
                self.daemon
                    .jobs
                    .iter()
                    .rev()
                    .filter(|job| !job.lifecycle.is_terminal())
                    .map(JobHistoryRowRef::Daemon),
            );
        }
        rows.extend(
            self.background_jobs
                .jobs
                .iter()
                .rev()
                .filter(|job| {
                    !job.status.is_terminal()
                        && (!daemon_current
                            || !self.daemon.jobs.iter().any(|daemon| daemon.id == job.id.0))
                })
                .map(JobHistoryRowRef::Background),
        );
        if daemon_current {
            rows.extend(
                self.daemon
                    .jobs
                    .iter()
                    .rev()
                    .filter(|job| job.lifecycle.is_terminal())
                    .map(JobHistoryRowRef::Daemon),
            );
        }
        rows.extend(
            self.background_jobs
                .jobs
                .iter()
                .rev()
                .filter(|job| {
                    job.status.is_terminal()
                        && (!daemon_current
                            || !self.daemon.jobs.iter().any(|daemon| daemon.id == job.id.0))
                })
                .map(JobHistoryRowRef::Background),
        );
        let mut unmatched_daemon_builds = if daemon_current {
            self.daemon
                .jobs
                .iter()
                .filter(|job| {
                    job.kind == ClientDaemonJobKind::BitBakeBuild && job.lifecycle.is_terminal()
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        for record in self.build_history.iter().rev() {
            let duplicate = unmatched_daemon_builds.iter().position(|job| {
                let Some(target) = record.target.as_deref() else {
                    return false;
                };
                let daemon_target = job
                    .label
                    .strip_prefix("BitBake build ")
                    .unwrap_or(job.label.as_str());
                let daemon_success = job.lifecycle == ClientDaemonLifecycle::Exited
                    && job.exit_code.is_none_or(|exit_code| exit_code == 0);
                daemon_target == target && daemon_success == record.success
            });
            if let Some(index) = duplicate {
                unmatched_daemon_builds.remove(index);
            } else {
                rows.push(JobHistoryRowRef::Build(record));
            }
        }
        rows
    }
    pub fn job_summary(&self) -> JobSummary {
        let mut summary = JobSummary {
            daemon_owned: (self.daemon.status == ClientReplicaStatus::Current)
                .then_some(self.daemon.jobs.len()),
            ..JobSummary::default()
        };
        if self.daemon.status == ClientReplicaStatus::Current {
            for job in &self.daemon.jobs {
                match job.lifecycle {
                    ClientDaemonLifecycle::Connecting => summary.queued += 1,
                    ClientDaemonLifecycle::Running | ClientDaemonLifecycle::Stopping => {
                        summary.active += 1
                    }
                    ClientDaemonLifecycle::Failed => {
                        summary.failed += 1;
                        summary.recent_completed += 1;
                    }
                    ClientDaemonLifecycle::Exited
                    | ClientDaemonLifecycle::Lost
                    | ClientDaemonLifecycle::Disconnected => summary.recent_completed += 1,
                }
            }
        }
        for job in self.background_jobs.jobs.iter().filter(|job| {
            self.daemon.status != ClientReplicaStatus::Current
                || !self.daemon.jobs.iter().any(|daemon| daemon.id == job.id.0)
        }) {
            match job.status {
                BackgroundJobStatus::Queued => summary.queued += 1,
                BackgroundJobStatus::Starting
                | BackgroundJobStatus::Running
                | BackgroundJobStatus::Cancelling => summary.active += 1,
                BackgroundJobStatus::Failed => {
                    summary.failed += 1;
                    summary.recent_completed += 1;
                }
                BackgroundJobStatus::Succeeded
                | BackgroundJobStatus::Cancelled
                | BackgroundJobStatus::Lost => summary.recent_completed += 1,
            }
        }
        summary
    }
    pub fn transient_status(&self) -> Option<TransientStatus> {
        if let Some(activity) = self.background_activities.first() {
            return Some(TransientStatus {
                kind: TransientStatusKind::Activity,
                text: activity.label().into(),
            });
        }
        if let Some(text) = self.platform_menuconfig_waiting_label() {
            return Some(TransientStatus {
                kind: TransientStatusKind::Activity,
                text,
            });
        }

        let notification = self
            .notification
            .as_deref()
            .map(str::trim)
            .filter(|message| !message.is_empty());

        let notification_kind = notification.map(|message| {
            let logged_severity = self
                .logs
                .entries
                .iter()
                .rev()
                .find(|entry| entry.message == message)
                .map(|entry| entry.severity);
            match logged_severity {
                Some(Severity::Error) => TransientStatusKind::Error,
                Some(Severity::Warning) => TransientStatusKind::Warning,
                _ if self.build.status == BuildStatus::Failed
                    && message.starts_with("Build failed with ") =>
                {
                    TransientStatusKind::Error
                }
                _ if self.build.status == BuildStatus::Completed
                    && message.starts_with("Build completed with ") =>
                {
                    TransientStatusKind::Warning
                }
                _ if self.build.status == BuildStatus::Completed
                    && message == "Build completed successfully with no errors." =>
                {
                    TransientStatusKind::Success
                }
                _ if self.build.status == BuildStatus::Cancelled
                    && message == "Build was cancelled; this is distinct from a build failure." =>
                {
                    TransientStatusKind::Warning
                }
                _ => TransientStatusKind::Notification,
            }
        });
        if notification_kind == Some(TransientStatusKind::Error) {
            return Some(TransientStatus {
                kind: TransientStatusKind::Error,
                text: notification
                    .expect("an error kind requires notification text")
                    .to_owned(),
            });
        }
        if self.active_dialog().is_some_and(Dialog::is_confirmation) {
            return Some(TransientStatus {
                kind: TransientStatusKind::Confirmation,
                text: "Confirmation pending".into(),
            });
        }
        if let Some(message) = notification {
            return Some(TransientStatus {
                kind: notification_kind
                    .expect("notification text always has a projected semantic kind"),
                text: message.to_owned(),
            });
        }

        match self.daemon.status {
            ClientReplicaStatus::Synchronizing => {
                return Some(TransientStatus {
                    kind: TransientStatusKind::Reconnecting,
                    text: "Daemon synchronizing".into(),
                });
            }
            ClientReplicaStatus::Stale => {
                return Some(TransientStatus {
                    kind: TransientStatusKind::Warning,
                    text: "Daemon state stale".into(),
                });
            }
            ClientReplicaStatus::Current
                if self.daemon.bitbake == ClientDaemonLifecycle::Connecting =>
            {
                return Some(TransientStatus {
                    kind: TransientStatusKind::Activity,
                    text: "BitBake connecting".into(),
                });
            }
            ClientReplicaStatus::Disconnected | ClientReplicaStatus::Current => {}
        }

        let text = match self.build.status {
            BuildStatus::LoadingWorkspace => Some("Workspace loading".to_owned()),
            BuildStatus::Parsing => Some("BitBake parsing".to_owned()),
            BuildStatus::Running => {
                let active = self
                    .tasks
                    .values()
                    .filter(|task| task.state == TaskState::Active)
                    .count();
                let queued = self.job_summary().queued;
                Some(if queued > 0 {
                    format!("Build running · {active} active · {queued} queued")
                } else {
                    format!("Build running · {active} active")
                })
            }
            BuildStatus::Cancelling => Some("Build cancellation pending".to_owned()),
            BuildStatus::Idle
            | BuildStatus::Completed
            | BuildStatus::Cancelled
            | BuildStatus::Failed
            | BuildStatus::Lost => None,
        };
        if let Some(text) = text {
            return Some(TransientStatus {
                kind: TransientStatusKind::Activity,
                text,
            });
        }
        let jobs = self.job_summary();
        (jobs.active > 0 || jobs.queued > 0).then(|| TransientStatus {
            kind: TransientStatusKind::Activity,
            text: format!("{} active jobs · {} queued", jobs.active, jobs.queued),
        })
    }
}
