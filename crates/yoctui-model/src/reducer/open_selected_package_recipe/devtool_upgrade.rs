use super::*;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match action {
        Action::ConfirmDevtoolUpgrade => {
            if let Some(Dialog::DevtoolUpgradeConfirmation(plan)) = app.active_dialog().cloned() {
                let Some(status) = app.devtool_statuses.get(&plan.identity) else {
                    app.notification =
                        Some("Authoritative Devtool status expired; refresh with t.".into());
                    return None;
                };
                if let Some(reason) = status.disabled_reason(DevtoolAction::Upgrade) {
                    app.notification = Some(reason);
                    return None;
                }
                if let Err(error) = plan.operation().validate() {
                    app.notification = Some(error.to_string());
                    return None;
                }
                close_dialog(app);
                synchronize_focus(app);
                return Some(Effect::DevtoolUpgrade(plan));
            }
        }
        Action::CancelDevtoolUpgrade => {
            if matches!(
                app.active_dialog(),
                Some(Dialog::DevtoolUpgradeConfirmation(_))
            ) {
                close_dialog(app);
            }
        }
        _ => unreachable!("action routed to the wrong reducer"),
    }
    synchronize_focus(app);
    None
}
