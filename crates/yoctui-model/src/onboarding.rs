use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{
    App, BuildStatus, ClientReplicaStatus, ImageArtifactInventoryState, RootfsCompositionState,
    Screen,
};

pub const ONBOARDING_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum OnboardingStep {
    #[default]
    Environment,
    Target,
    FirstBuild,
    Diagnostics,
    ArtifactsRootfs,
    Terminal,
}

impl OnboardingStep {
    pub const ALL: [Self; 6] = [
        Self::Environment,
        Self::Target,
        Self::FirstBuild,
        Self::Diagnostics,
        Self::ArtifactsRootfs,
        Self::Terminal,
    ];

    pub const fn title(self) -> &'static str {
        match self {
            Self::Environment => "Verify environment",
            Self::Target => "Select a target",
            Self::FirstBuild => "Review the first build",
            Self::Diagnostics => "Inspect logs and errors",
            Self::ArtifactsRootfs => "Explore artifacts and rootfs",
            Self::Terminal => "Use Terminal Sessions",
        }
    }

    pub const fn destination(self) -> &'static str {
        match self {
            Self::Environment => "Build environment",
            Self::Target => "Image target picker",
            Self::FirstBuild => "Build options confirmation",
            Self::Diagnostics => "Logs / Errors",
            Self::ArtifactsRootfs => "Images / Rootfs",
            Self::Terminal => "Terminal Sessions",
        }
    }

    pub const fn instruction(self) -> &'static str {
        match self {
            Self::Environment => {
                "Configure source, build directory, and init script; verify before continuing."
            }
            Self::Target => {
                "Choose one image recipe from the connected workspace. Selection does not build it."
            }
            Self::FirstBuild => {
                "Open Build options, review the exact request, and confirm only when you are ready."
            }
            Self::Diagnostics => {
                "Use structured Logs and Errors to understand the completed build attempt."
            }
            Self::ArtifactsRootfs => {
                "Inspect correlated deploy artifacts, installed packages, and filesystem composition."
            }
            Self::Terminal => {
                "Open daemon-owned Terminal Sessions; creating a shell remains a separate command."
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OnboardingProgress {
    #[serde(default = "onboarding_schema_version")]
    pub schema_version: u16,
    #[serde(default)]
    pub current: OnboardingStep,
    #[serde(default)]
    pub completed: BTreeSet<OnboardingStep>,
    #[serde(default)]
    pub skipped: BTreeSet<OnboardingStep>,
    #[serde(default)]
    pub dismissed: bool,
}

const fn onboarding_schema_version() -> u16 {
    ONBOARDING_SCHEMA_VERSION
}

impl Default for OnboardingProgress {
    fn default() -> Self {
        Self {
            schema_version: ONBOARDING_SCHEMA_VERSION,
            current: OnboardingStep::Environment,
            completed: BTreeSet::new(),
            skipped: BTreeSet::new(),
            dismissed: false,
        }
    }
}

impl OnboardingProgress {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != ONBOARDING_SCHEMA_VERSION {
            return Err(format!(
                "unsupported onboarding schema {}; expected {}",
                self.schema_version, ONBOARDING_SCHEMA_VERSION
            ));
        }
        if let Some(step) = self.completed.intersection(&self.skipped).next() {
            return Err(format!(
                "onboarding step {:?} cannot be both completed and skipped",
                step
            ));
        }
        Ok(())
    }

    pub fn is_finished(&self) -> bool {
        OnboardingStep::ALL
            .iter()
            .all(|step| self.completed.contains(step) || self.skipped.contains(step))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnboardingState {
    pub open: bool,
    pub selected: OnboardingStep,
    pub progress: OnboardingProgress,
}

impl Default for OnboardingState {
    fn default() -> Self {
        let progress = OnboardingProgress::default();
        Self {
            open: false,
            selected: progress.current,
            progress,
        }
    }
}

impl OnboardingState {
    pub fn install(&mut self, progress: OnboardingProgress, open: bool) -> Result<(), String> {
        progress.validate()?;
        self.selected = progress.current;
        self.progress = progress;
        self.open = open;
        Ok(())
    }

    pub fn select(&mut self, delta: isize) {
        let current = OnboardingStep::ALL
            .iter()
            .position(|step| *step == self.selected)
            .unwrap_or_default();
        let next = if delta.is_negative() {
            current.saturating_sub(delta.unsigned_abs())
        } else {
            current
                .saturating_add(delta as usize)
                .min(OnboardingStep::ALL.len() - 1)
        };
        self.selected = OnboardingStep::ALL[next];
    }

    pub fn move_after_current(&mut self) {
        let current = OnboardingStep::ALL
            .iter()
            .position(|step| *step == self.progress.current)
            .unwrap_or_default();
        if let Some(next) = OnboardingStep::ALL.get(current + 1).copied() {
            self.progress.current = next;
            self.selected = next;
        } else {
            self.progress.dismissed = true;
            self.open = false;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnboardingStepStatus {
    Completed,
    Current,
    Blocked,
    Skipped,
    Stale,
    Unavailable,
}

impl OnboardingStepStatus {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Completed => "COMPLETED",
            Self::Current => "CURRENT",
            Self::Blocked => "BLOCKED",
            Self::Skipped => "SKIPPED",
            Self::Stale => "STALE",
            Self::Unavailable => "UNAVAILABLE",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnboardingStepProjection {
    pub step: OnboardingStep,
    pub status: OnboardingStepStatus,
    pub title: &'static str,
    pub destination: &'static str,
    pub instruction: &'static str,
    pub prerequisite: String,
    pub completion_satisfied: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnboardingProjection {
    pub rows: Vec<OnboardingStepProjection>,
    pub selected: OnboardingStep,
    pub current: OnboardingStep,
    pub finished: bool,
}

impl App {
    pub fn onboarding_projection(&self) -> OnboardingProjection {
        let mut prior_satisfied = true;
        let mut rows = Vec::with_capacity(OnboardingStep::ALL.len());
        for step in OnboardingStep::ALL {
            let evidence = onboarding_completion_evidence(self, step);
            let completed = self.onboarding.progress.completed.contains(&step);
            let skipped = self.onboarding.progress.skipped.contains(&step);
            let unavailable = onboarding_unavailable(self, step);
            let status = if completed {
                if prior_satisfied && evidence {
                    OnboardingStepStatus::Completed
                } else {
                    OnboardingStepStatus::Stale
                }
            } else if skipped {
                OnboardingStepStatus::Skipped
            } else if !prior_satisfied {
                OnboardingStepStatus::Blocked
            } else if step == self.onboarding.progress.current && unavailable {
                OnboardingStepStatus::Unavailable
            } else if step == self.onboarding.progress.current {
                OnboardingStepStatus::Current
            } else {
                OnboardingStepStatus::Blocked
            };
            rows.push(OnboardingStepProjection {
                step,
                status,
                title: step.title(),
                destination: step.destination(),
                instruction: step.instruction(),
                prerequisite: onboarding_prerequisite(self, step),
                completion_satisfied: evidence,
            });
            prior_satisfied &= (completed && evidence) || skipped;
        }
        let finished = rows.iter().all(|row| {
            matches!(
                row.status,
                OnboardingStepStatus::Completed | OnboardingStepStatus::Skipped
            )
        });
        OnboardingProjection {
            rows,
            selected: self.onboarding.selected,
            current: self.onboarding.progress.current,
            finished,
        }
    }
}

pub fn onboarding_route(app: &App, step: OnboardingStep) -> crate::Action {
    match step {
        OnboardingStep::Environment => crate::Action::Open(Screen::BuildEnvironment),
        OnboardingStep::Target => crate::Action::OpenImagePicker(app.available_images.clone()),
        OnboardingStep::FirstBuild => crate::Action::OpenBuildOptions,
        OnboardingStep::Diagnostics
            if app.build.errors > 0 || app.logs.diagnostics().next().is_some() =>
        {
            crate::Action::Open(Screen::Errors)
        }
        OnboardingStep::Diagnostics => crate::Action::Open(Screen::Logs),
        OnboardingStep::ArtifactsRootfs => crate::Action::Open(Screen::Images),
        OnboardingStep::Terminal => crate::Action::Open(Screen::TerminalSessions),
    }
}

pub fn onboarding_completion_evidence(app: &App, step: OnboardingStep) -> bool {
    match step {
        OnboardingStep::Environment => app.build_environment.connected(),
        OnboardingStep::Target => app.build.target.as_ref().is_some_and(|target| {
            app.available_images.iter().any(|image| image == target)
                || app
                    .workspace
                    .recipes
                    .iter()
                    .any(|recipe| &recipe.name == target)
        }),
        OnboardingStep::FirstBuild => {
            !app.build_history.is_empty()
                || matches!(
                    app.build.status,
                    BuildStatus::Completed | BuildStatus::Cancelled | BuildStatus::Failed
                )
        }
        OnboardingStep::Diagnostics => {
            !app.build_history.is_empty()
                || matches!(
                    app.build.status,
                    BuildStatus::Completed | BuildStatus::Cancelled | BuildStatus::Failed
                )
        }
        OnboardingStep::ArtifactsRootfs => {
            matches!(
                app.image_artifacts,
                ImageArtifactInventoryState::Available { .. }
                    | ImageArtifactInventoryState::Partial { .. }
            ) || matches!(
                app.rootfs_composition,
                RootfsCompositionState::Available { .. } | RootfsCompositionState::Partial { .. }
            )
        }
        OnboardingStep::Terminal => app.daemon.status == ClientReplicaStatus::Current,
    }
}

fn onboarding_unavailable(app: &App, step: OnboardingStep) -> bool {
    match step {
        OnboardingStep::Environment => false,
        OnboardingStep::Target => {
            !app.build_environment.connected()
                || (!onboarding_completion_evidence(app, step) && app.available_images.is_empty())
        }
        OnboardingStep::FirstBuild => {
            !app.build_environment.connected()
                || !onboarding_completion_evidence(app, OnboardingStep::Target)
        }
        OnboardingStep::Diagnostics => false,
        OnboardingStep::ArtifactsRootfs => !onboarding_completion_evidence(app, step),
        OnboardingStep::Terminal => app.daemon.status != ClientReplicaStatus::Current,
    }
}

fn onboarding_prerequisite(app: &App, step: OnboardingStep) -> String {
    match step {
        OnboardingStep::Environment if app.build_environment.connected() => {
            "Verified connected environment is current.".into()
        }
        OnboardingStep::Environment => "Requires environment verification.".into(),
        OnboardingStep::Target if app.available_images.is_empty() => {
            "Requires verified image-recipe inventory.".into()
        }
        OnboardingStep::Target => "Requires one current image target selection.".into(),
        OnboardingStep::FirstBuild => {
            "Requires verified environment and selected target; confirmation stays explicit.".into()
        }
        OnboardingStep::Diagnostics => "Requires one terminal build attempt.".into(),
        OnboardingStep::ArtifactsRootfs => {
            "Requires a current correlated deploy artifact or rootfs composition.".into()
        }
        OnboardingStep::Terminal => {
            "Requires a current daemon connection; opening creates no shell.".into()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Action, BuildEnvironmentState, ClientReplicaStatus, update};

    #[test]
    fn ux_onboarding_projection_distinguishes_current_blocked_skipped_stale_and_unavailable() {
        let mut app = App::new_unconfigured(32, 4096);
        app.onboarding.open = true;
        let projection = app.onboarding_projection();
        assert_eq!(projection.rows[0].status, OnboardingStepStatus::Current);
        assert_eq!(projection.rows[1].status, OnboardingStepStatus::Blocked);

        app.onboarding
            .progress
            .completed
            .insert(OnboardingStep::Environment);
        app.onboarding
            .progress
            .completed
            .insert(OnboardingStep::Target);
        let projection = app.onboarding_projection();
        assert_eq!(projection.rows[0].status, OnboardingStepStatus::Stale);
        assert_eq!(projection.rows[1].status, OnboardingStepStatus::Stale);
        assert!(!projection.finished);

        app.onboarding.progress.completed.clear();
        app.onboarding
            .progress
            .skipped
            .insert(OnboardingStep::Environment);
        app.onboarding.progress.current = OnboardingStep::Target;
        app.onboarding.selected = OnboardingStep::Target;
        let projection = app.onboarding_projection();
        assert_eq!(projection.rows[0].status, OnboardingStepStatus::Skipped);
        assert_eq!(projection.rows[1].status, OnboardingStepStatus::Unavailable);
    }

    #[test]
    fn ux_onboarding_open_and_resume_never_emit_execution_effects() {
        let mut app = App::new_unconfigured(32, 4096);
        assert_eq!(update(&mut app, Action::OpenOnboarding), None);
        assert!(app.onboarding.open);
        assert_eq!(app.build.status, BuildStatus::Idle);
        assert!(app.daemon.pty_sessions.is_empty());

        let effect = update(&mut app, Action::DismissOnboarding);
        assert_eq!(effect, Some(crate::Effect::PersistOnboarding));
        assert!(!app.onboarding.open);
        assert!(app.onboarding.progress.dismissed);
    }

    #[test]
    fn ux_onboarding_advances_only_with_exact_evidence_and_routes_through_typed_actions() {
        let mut app = App::new_unconfigured(32, 4096);
        app.onboarding.open = true;
        assert_eq!(update(&mut app, Action::AdvanceOnboarding), None);
        assert_eq!(app.onboarding.progress.current, OnboardingStep::Environment);

        app.build_environment = BuildEnvironmentState::Connected(crate::BuildEnvironmentProfile {
            source_dir: "/work/poky".into(),
            build_dir: "/work/build".into(),
            init_script: "/work/poky/oe-init-build-env".into(),
        });
        assert_eq!(
            update(&mut app, Action::AdvanceOnboarding),
            Some(crate::Effect::PersistOnboarding)
        );
        assert_eq!(app.onboarding.progress.current, OnboardingStep::Target);

        assert_eq!(
            onboarding_route(&app, OnboardingStep::FirstBuild),
            Action::OpenBuildOptions
        );
        assert_eq!(
            onboarding_route(&app, OnboardingStep::Terminal),
            Action::Open(Screen::TerminalSessions)
        );
        app.daemon.status = ClientReplicaStatus::Current;
        assert!(onboarding_completion_evidence(
            &app,
            OnboardingStep::Terminal
        ));
    }

    #[test]
    fn ux_onboarding_progress_validation_rejects_future_and_conflicting_state() {
        let mut future = OnboardingProgress::default();
        future.schema_version += 1;
        assert!(future.validate().is_err());

        let mut conflict = OnboardingProgress::default();
        conflict.completed.insert(OnboardingStep::Environment);
        conflict.skipped.insert(OnboardingStep::Environment);
        assert!(conflict.validate().is_err());
    }
}
