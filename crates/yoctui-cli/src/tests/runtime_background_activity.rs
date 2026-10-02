use super::*;
use yoctui_model::{BackgroundActivity, ClientReplicaStatus};

#[test]
fn background_activity_polling_matches_guarded_reducer_without_environment_effects() {
    for status in [
        ClientReplicaStatus::Disconnected,
        ClientReplicaStatus::Current,
    ] {
        for reduced_motion in [false, true] {
            let mut app = App::new(10, 1024);
            app.daemon.status = status;
            app.reduced_motion = reduced_motion;
            app.notification = Some("Retained notice".into());
            app.workspace.recipes = (0..4096)
                .map(|index| yoctui_model::Recipe {
                    name: format!("recipe-{index}"),
                    ..Default::default()
                })
                .collect();
            let mut reference = app.clone();
            for active in [false, true, true, false, false] {
                for activity in [
                    BackgroundActivity::Initializing,
                    BackgroundActivity::Cancelling,
                    BackgroundActivity::Loading,
                ] {
                    assert!(
                        compatibility_workspace_action(
                            &mut reference,
                            Action::SetBackgroundActivity { activity, active },
                        )
                        .is_none(),
                        "local flags must never emit environment work"
                    );
                    set_local_background_activity(&mut app, activity, active);
                    assert_eq!(app.background_activities, reference.background_activities);
                    assert_eq!(app.notification, reference.notification);
                    assert_eq!(app.focus, reference.focus);
                    assert_eq!(app.active_dialog(), reference.active_dialog());
                    assert_eq!(app.daemon.status, status);
                    assert_eq!(app.workspace.recipes, reference.workspace.recipes);
                    assert_eq!(app.reduced_motion, reduced_motion);
                }
            }
            assert!(app.background_activities.is_empty());
        }
    }
}
