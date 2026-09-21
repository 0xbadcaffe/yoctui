pub fn authorize_workspace_effect(
    app: &App,
    effect: &Effect,
) -> Result<WorkspaceAvailability, WorkspaceEffectDenied> {
    let availability = app
        .workspace_compatibility
        .availability(&workspace_effect_requirement(effect));
    if availability.is_enabled() {
        Ok(availability)
    } else {
        Err(WorkspaceEffectDenied { availability })
    }
}

/// Capability-aware reducer boundary. If an action attempts to emit an
/// unavailable environment effect, preparation mutations are rolled back and
/// only the exact denial notice is retained.
pub fn update_with_workspace_authority(app: &mut App, action: crate::Action) -> Option<Effect> {
    let before = app.clone();
    let effect = crate::update(app, action)?;
    if app.is_offline()
        && !matches!(
            workspace_effect_requirement(&effect),
            WorkspaceEffectRequirement::ClientLocal
        )
    {
        *app = before;
        app.notification = Some("Connect to a current daemon and verify the build environment first; saved history remains available with F3.".into());
        return None;
    }
    match authorize_workspace_effect(app, &effect) {
        Ok(_) => Some(effect),
        Err(error) => {
            *app = before;
            app.notification = Some(error.reason());
            None
        }
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[error("workspace effect is unavailable")]
pub struct WorkspaceEffectDenied {
    pub availability: WorkspaceAvailability,
}

impl WorkspaceEffectDenied {
    pub fn reason(&self) -> String {
        self.availability
            .issues
            .iter()
            .map(|issue| issue.reason.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

pub fn install_workspace_compatibility(
    app: &mut App,
    authority: DaemonCompatibilitySnapshot,
) -> Result<WorkspaceRevalidation, WorkspaceCompatibilityError> {
    let install = app.workspace_compatibility.install(authority)?;
    app.compatibility_ui
        .reconcile(app.workspace_compatibility.authority());
    if install == WorkspaceSnapshotInstall::Unchanged {
        return Ok(WorkspaceRevalidation {
            install,
            closed_dialog: false,
            reason: None,
        });
    }
    let revalidation = revalidate_workspace_dialog(app, install);
    reproject_raw_mode_authority(app);
    Ok(revalidation)
}

pub fn invalidate_workspace_compatibility(app: &mut App) -> WorkspaceRevalidation {
    app.workspace_compatibility.invalidate();
    app.compatibility_ui.reconcile(None);
    let revalidation = revalidate_workspace_dialog(app, WorkspaceSnapshotInstall::Invalidated);
    reproject_raw_mode_authority(app);
    revalidation
}

fn reproject_raw_mode_authority(app: &mut App) {
    let was_raw_modal = app.screen == crate::Screen::RawMode
        && matches!(
            app.raw_mode.view,
            crate::RawModeView::Form | crate::RawModeView::Preview
        );
    let authority = app.workspace_compatibility.authority().cloned();
    crate::reduce_raw_mode(
        &mut app.raw_mode,
        crate::builtin_raw_catalog(),
        authority.as_ref(),
        crate::RawModeAction::ReprojectAuthority,
    );
    let is_raw_modal = app.screen == crate::Screen::RawMode
        && matches!(
            app.raw_mode.view,
            crate::RawModeView::Form | crate::RawModeView::Preview
        );
    if was_raw_modal || is_raw_modal {
        crate::synchronize_focus(app);
    }
}

fn revalidate_workspace_dialog(
    app: &mut App,
    install: WorkspaceSnapshotInstall,
) -> WorkspaceRevalidation {
    let unavailable = app.active_dialog().and_then(|dialog| {
        let availability = app
            .workspace_compatibility
            .availability(&workspace_dialog_requirement(dialog));
        (!availability.is_enabled()).then_some(availability)
    });
    let reason = unavailable.as_ref().map(|availability| {
        availability
            .issues
            .iter()
            .map(|issue| issue.reason.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    });
    if unavailable.is_some() {
        crate::close_dialog(app);
        app.notification = reason
            .as_ref()
            .map(|reason| format!("Action closed after environment capability update: {reason}"));
        crate::synchronize_focus(app);
    }
    WorkspaceRevalidation {
        install,
        closed_dialog: unavailable.is_some(),
        reason,
    }
}
