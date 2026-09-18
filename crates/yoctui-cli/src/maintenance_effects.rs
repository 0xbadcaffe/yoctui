//! Maintenance effects.
use super::*;

pub(crate) async fn route_independent_maintenance_effect(
    guard: &TerminalGuard,
    app: &mut App,
    coordinator: &mut MaintenanceCliCoordinator,
    effect: Effect,
    editor: Option<&str>,
) -> bool {
    match effect {
        Effect::Maintenance(yoctui_model::MaintenanceEffect::OpenEvidence(identity)) => {
            match coordinator.revalidate_evidence(&identity) {
                Ok(path) => open_in_editor(guard, app, path, editor).await,
                Err(message) => app.notification = Some(message),
            }
            true
        }
        Effect::Maintenance(yoctui_model::MaintenanceEffect::Navigate(screen)) => {
            if let Some(next) = compatibility_workspace_action(app, Action::Open(screen)) {
                let _ = coordinator.handle_effect(app, next).await;
            }
            true
        }
        Effect::Maintenance(_) => coordinator.handle_effect(app, effect).await,
        _ => false,
    }
}

pub(crate) fn maintenance_row_count(app: &App) -> usize {
    match app.maintenance.view {
        yoctui_model::MaintenanceView::Sstate => 2,
        yoctui_model::MaintenanceView::Services => 1,
        yoctui_model::MaintenanceView::Release => 4,
        yoctui_model::MaintenanceView::Integrations => 4,
    }
}
