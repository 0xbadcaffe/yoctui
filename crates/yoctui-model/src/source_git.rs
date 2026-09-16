//! Source repository facts supplied by the process adapter.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum SourceGitStatus {
    #[default]
    Unknown,
    Scanning,
    Unavailable(String),
    Ready(SourceGitSummary),
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SourceGitSummary {
    pub branch: String,
    pub upstream: Option<String>,
    pub ahead: u64,
    pub behind: u64,
    pub staged: usize,
    pub unstaged: usize,
    pub untracked: usize,
    pub conflicts: usize,
}
impl SourceGitStatus {
    pub fn label(&self) -> Option<String> {
        Some(match self {
            Self::Unknown => return None,
            Self::Scanning => "Git: scanning…".into(),
            Self::Unavailable(_) => "Git: unavailable".into(),
            Self::Ready(s) => {
                let mut parts = Vec::new();
                if s.conflicts > 0 {
                    parts.push(format!("!{}", s.conflicts));
                }
                if s.staged > 0 {
                    parts.push(format!("+{}", s.staged));
                }
                if s.unstaged > 0 {
                    parts.push(format!("~{}", s.unstaged));
                }
                if s.untracked > 0 {
                    parts.push(format!("?{}", s.untracked));
                }
                if s.ahead > 0 {
                    parts.push(format!("ahead {}", s.ahead));
                }
                if s.behind > 0 {
                    parts.push(format!("behind {}", s.behind));
                }
                if s.upstream.is_none() {
                    parts.push("no upstream".into());
                } else if s.ahead == 0 && s.behind == 0 {
                    parts.push("synced*".into());
                }
                format!("Git: {} {}", s.branch, parts.join(" "))
            }
        })
    }
}

impl crate::App {
    pub fn source_repository_path(&self) -> Option<&std::path::Path> {
        use crate::BuildEnvironmentState::*;
        match &self.build_environment {
            Configured(p)
            | Connected(p)
            | Verifying { profile: p, .. }
            | Failed { profile: p, .. }
                if p.init_script != std::path::Path::new("/") =>
            {
                Some(&p.source_dir)
            }
            _ => self.workspace.source_dir.as_deref(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn git_status_reducer_preserves_independent_dirty_and_sync_facts() {
        let mut app = crate::App::new(10, 1024);
        crate::update(
            &mut app,
            crate::Action::SourceGitStatusUpdated(SourceGitStatus::Ready(SourceGitSummary {
                upstream: Some("origin/master".into()),
                staged: 2,
                unstaged: 1,
                ahead: 3,
                ..Default::default()
            })),
        );
        let label = app.source_git_status.label().unwrap();
        assert!(label.contains("+2 ~1 ahead 3"));
        assert!(!label.contains("synced"));
    }
}

#[cfg(test)]
mod gitui_tests {
    use crate::*;
    #[test]
    fn gitui_launch_is_typed_scoped_and_reports_missing_tools() {
        let mut app = App::new(10, 1024);
        update(&mut app, Action::OpenGitUi);
        assert!(app.notification.as_ref().unwrap().contains("not installed"));
        update(
            &mut app,
            Action::GitUiDetected(Some("/usr/bin/gitui".into())),
        );
        app.workspace.source_dir = Some("/source with spaces".into());
        app.source_git_status = SourceGitStatus::Ready(SourceGitSummary::default());
        let command = app
            .application_menu_items(ApplicationMenuGroup::Tools)
            .into_iter()
            .find(|c| c.label == "Open GitUI")
            .unwrap();
        assert!(command.enabled());
        update(&mut app, Action::OpenGitUi);
        let Some(Dialog::TerminalLaunch(dialog)) = app.active_dialog() else {
            panic!("missing GitUI launch dialog");
        };
        assert_eq!(
            dialog.request.cwd,
            std::path::PathBuf::from("/source with spaces")
        );
        assert_eq!(dialog.request.kind, TerminalCreationKind::GitUi);
        assert!(dialog.request.arguments.is_empty());
        assert!(matches!(
            update(&mut app, Action::ConfirmTerminalLaunch),
            Some(Effect::Terminal(TerminalEffect::Create {
                kind: TerminalCreationKind::GitUi,
                ..
            }))
        ));
        assert_eq!(app.screen, Screen::TerminalSessions);
        assert_eq!(app.focus, FocusTarget::Workspace);
    }
}
