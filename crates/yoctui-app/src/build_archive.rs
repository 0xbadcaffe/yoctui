//! Bounded read-only projection of daemon build evidence.
use std::collections::BTreeMap;
use yoctui_model::{SavedBuild, SavedBuildLog, SavedBuildOutcome, SavedBuildTask, Severity};
use yoctui_protocol::daemon::{
    DaemonBuildEvent as Event, DaemonSnapshot, JobKind, LifecycleState, LogSeverity,
};

fn text(value: &str) -> String {
    let mut result = String::new();
    for c in value
        .chars()
        .filter(|c| !c.is_control() || matches!(c, '\n' | '\t'))
    {
        if result.len() + c.len_utf8() > 4096 {
            break;
        }
        result.push(c);
    }
    result
}

pub fn capture_saved_build(snapshot: &DaemonSnapshot, saved_unix_ms: u64) -> Option<SavedBuild> {
    let job = snapshot
        .jobs
        .iter()
        .filter(|j| j.kind == JobKind::BitBakeBuild)
        .max_by_key(|j| j.id.0)?;
    let instance = snapshot
        .daemon_instance_id
        .0
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let mut result = SavedBuild {
        id: format!("{instance}-{}", job.id.0), target: text(&job.label), machine: None,
        source: snapshot.workspace.as_ref().map(|w| text(&w.canonical_source)),
        build_dir: snapshot.workspace.as_ref().map(|w| text(&w.canonical_build)),
        outcome: match job.lifecycle {
            LifecycleState::Exited if job.exit_code == Some(0) => SavedBuildOutcome::Succeeded,
            LifecycleState::Failed => SavedBuildOutcome::Failed,
            LifecycleState::Lost => SavedBuildOutcome::Lost,
            _ => SavedBuildOutcome::Incomplete,
        },
        saved_unix_ms, started_unix_ms: None, finished_unix_ms: None,
        logs: Vec::new(), tasks: Vec::new(),
        limitations: vec!["Bounded capture: at most 256 log lines and 256 task rows; not a complete build transcript.".into()],
    };
    let mut tasks = BTreeMap::new();
    for event in &snapshot.build_events {
        let task = match event {
            Event::Reset { targets } => {
                result.target = text(&targets.join(" "));
                None
            }
            Event::Workspace { data } => {
                result.machine = data.variables.get("MACHINE").map(|s| text(s));
                None
            }
            Event::Started { started_unix_ms } => {
                result.started_unix_ms = *started_unix_ms;
                None
            }
            Event::Completed {
                success,
                finished_unix_ms,
                ..
            } => {
                result.finished_unix_ms = *finished_unix_ms;
                if job.lifecycle == LifecycleState::Exited {
                    result.outcome = if *success {
                        SavedBuildOutcome::Succeeded
                    } else {
                        SavedBuildOutcome::Failed
                    };
                }
                None
            }
            Event::TaskQueued { recipe, task, .. } => {
                Some((recipe, task, "Queued (last observed)"))
            }
            Event::TaskStarted { recipe, task, .. } | Event::TaskProgress { recipe, task, .. } => {
                Some((recipe, task, "Running (last observed)"))
            }
            Event::TaskCompleted {
                recipe,
                task,
                success,
                ..
            } => Some((recipe, task, if *success { "Succeeded" } else { "Failed" })),
            _ => None,
        };
        if let Some((recipe, task, status)) = task {
            tasks.insert((text(recipe), text(task)), status);
            while tasks.len() > 256 {
                tasks.pop_first();
            }
        }
    }
    result.tasks = tasks
        .into_iter()
        .map(|((recipe, task), status)| SavedBuildTask {
            recipe,
            task,
            status: status.into(),
        })
        .collect();
    if let Some(start) = result.started_unix_ms {
        result.logs = snapshot
            .recent_logs
            .iter()
            .filter(|l| {
                l.source == "bitbake"
                    && l.unix_ms >= start
                    && l.unix_ms <= result.finished_unix_ms.unwrap_or(saved_unix_ms)
            })
            .rev()
            .take(256)
            .map(|l| SavedBuildLog {
                unix_ms: l.unix_ms,
                message: text(&l.message),
                severity: match l.severity {
                    LogSeverity::Trace => Severity::Trace,
                    LogSeverity::Warning => Severity::Warning,
                    LogSeverity::Error => Severity::Error,
                    _ => Severity::Info,
                },
            })
            .collect();
        result.logs.reverse();
    }
    if result.logs.is_empty() {
        result.limitations.push(
            "Saved logs unavailable: no correlated log lines were retained for this build.".into(),
        );
    }
    if result.outcome == SavedBuildOutcome::Failed {
        result.limitations.push("Backend failure may include cancellation; the retained protocol does not distinguish it.".into());
    }
    Some(result)
}

pub fn saved_build_workspace_action(
    app: &yoctui_model::App,
    input: crate::Input,
) -> Option<yoctui_model::Action> {
    use crate::Input;
    use yoctui_model::{Action, FocusTarget, SavedBuildAction as A, Screen};
    if app.screen != Screen::BuildHistory
        || app.focus != FocusTarget::Workspace
        || app.active_dialog().is_some()
        || app.command_palette_open
        || app.menu.is_open()
        || app.onboarding.open
        || app.keymap_preferences_ui.open
        || app
            .notification
            .as_deref()
            .is_some_and(yoctui_model::notification_requires_acknowledgement)
    {
        return None;
    }
    let browsing = app.is_offline() || app.saved_builds.browsing;
    let action = match input {
        Input::Char('l') => A::Toggle,
        Input::Char('r') => A::Refresh,
        Input::Enter if browsing => A::Open,
        Input::Esc if browsing && app.saved_builds.view.is_some() => A::Close,
        Input::Up | Input::Char('k') if browsing && app.saved_builds.view.is_none() => {
            A::Select(-1)
        }
        Input::Down | Input::Char('j') if browsing && app.saved_builds.view.is_none() => {
            A::Select(1)
        }
        Input::Up | Input::Char('k') if browsing => A::Scroll(-1),
        Input::Down | Input::Char('j') if browsing => A::Scroll(1),
        Input::Left if browsing && app.saved_builds.view.is_some() => A::ShiftView(-1),
        Input::Right if browsing && app.saved_builds.view.is_some() => A::ShiftView(1),
        Input::PageUp if browsing => A::Scroll(-10),
        Input::PageDown if browsing => A::Scroll(10),
        _ => return None,
    };
    Some(Action::SavedBuild(action))
}

pub fn legacy_saved_builds(
    state: &yoctui_protocol::daemon_persist::DaemonPersistedState,
) -> Vec<SavedBuild> {
    let instance = state
        .previous_daemon_instance_id
        .0
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    state.job_history.iter().filter(|j|j.kind==JobKind::BitBakeBuild).rev().take(32).map(|j|SavedBuild {
        id:format!("{instance}-{}",j.id.0), target:text(&j.label), machine:None,
        source:state.workspace.as_ref().map(|w|text(&w.canonical_source)),build_dir:state.workspace.as_ref().map(|w|text(&w.canonical_build)),
        outcome:match j.lifecycle { LifecycleState::Exited if j.exit_code==Some(0)=>SavedBuildOutcome::Succeeded, LifecycleState::Failed=>SavedBuildOutcome::Failed,LifecycleState::Lost=>SavedBuildOutcome::Lost,_=>SavedBuildOutcome::Incomplete },
        saved_unix_ms:state.saved_unix_ms,started_unix_ms:None,finished_unix_ms:None,logs:Vec::new(),tasks:Vec::new(),
        limitations:vec!["Legacy summary only: logs, machine, duration and task details were not saved per build.".into()],
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use yoctui_protocol::daemon::*;
    fn snapshot() -> DaemonSnapshot {
        DaemonSnapshot {
            daemon_instance_id: DaemonInstanceId([1; 16]),
            sequence: 1,
            generation: 1,
            workspace: None,
            project_profile: ProjectProfileSummary::Absent,
            bitbake: BitBakeState {
                lifecycle: LifecycleState::Running,
                version: None,
                capabilities: Vec::new(),
                diagnostic: None,
            },
            compatibility: None,
            jobs: vec![JobSummary {
                id: JobId(1),
                kind: JobKind::BitBakeBuild,
                label: "image".into(),
                lifecycle: LifecycleState::Exited,
                progress_current: None,
                progress_total: None,
                exit_code: Some(0),
            }],
            raw_executions: Vec::new(),
            raw_history: Vec::new(),
            pty_sessions: Vec::new(),
            pty_screens: Vec::new(),
            clients: Vec::new(),
            recent_logs: Vec::new(),
            build_progress: None,
            recovery_warnings: Vec::new(),
            build_events: vec![
                Event::Reset {
                    targets: vec!["image".into()],
                },
                Event::Started {
                    started_unix_ms: Some(100),
                },
                Event::Completed {
                    success: true,
                    exit_code: Some(0),
                    finished_unix_ms: Some(500),
                },
            ],
        }
    }
    #[test]
    fn archive_capture_excludes_other_builds_and_retains_bounded_text() {
        let mut s = snapshot();
        for n in 0..600 {
            s.recent_logs.push(LogRecord {
                source: "bitbake".into(),
                severity: LogSeverity::Info,
                message: format!("line {n} \x1b"),
                unix_ms: n,
                recipe: None,
                task: None,
                path: None,
                build: Some("image".into()),
            });
        }
        let r = capture_saved_build(&s, 600).unwrap();
        assert_eq!(r.logs.len(), 256);
        assert!(
            r.logs
                .iter()
                .all(|l| l.unix_ms >= 100 && l.unix_ms <= 500 && !l.message.contains('\x1b'))
        );
        assert_eq!(r.outcome, SavedBuildOutcome::Succeeded);
        assert!(r.limitations[0].contains("not a complete"));
        s.jobs[0].id = JobId(2);
        assert_ne!(capture_saved_build(&s, 600).unwrap().id, r.id);
        s.build_events.clear();
        assert!(capture_saved_build(&s, 600).unwrap().logs.is_empty());
    }
    #[test]
    fn archive_legacy_summaries_do_not_invent_logs_or_liveness() {
        let mut snapshot = snapshot();
        snapshot.jobs[0].lifecycle = LifecycleState::Running;
        let persisted = yoctui_protocol::daemon_persist::DaemonPersistedState::capture(
            &snapshot,
            1000,
            "boot".into(),
            Vec::new(),
            Default::default(),
        );
        let records = legacy_saved_builds(&persisted);
        assert_eq!(records[0].outcome, SavedBuildOutcome::Incomplete);
        assert!(records[0].logs.is_empty());
        assert!(records[0].started_unix_ms.is_none());
        assert!(records[0].limitations[0].contains("Legacy summary only"));
    }
    #[test]
    fn archive_input_respects_modal_focus_and_preserves_live_state() {
        use yoctui_model::{Action, FocusTarget, Screen};
        let mut app = yoctui_model::App::new(32, 4096);
        app.screen = Screen::BuildHistory;
        app.focus = FocusTarget::Workspace;
        app.saved_builds.browsing = true;
        assert!(matches!(
            saved_build_workspace_action(&app, crate::Input::Enter),
            Some(Action::SavedBuild(_))
        ));
        app.command_palette_open = true;
        assert!(saved_build_workspace_action(&app, crate::Input::Enter).is_none());
    }
}
