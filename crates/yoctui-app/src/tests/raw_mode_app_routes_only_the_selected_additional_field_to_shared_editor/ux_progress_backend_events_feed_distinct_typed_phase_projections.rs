use super::*;

#[test]
fn ux_progress_backend_events_feed_distinct_typed_phase_projections() {
    let action = model_action_from_backend_event(BackendEvent::ParseProgress {
        current: Some(8),
        total: Some(20),
    })
    .unwrap();
    assert_eq!(
        action,
        Action::ParseProgress {
            current: Some(8),
            total: Some(20)
        }
    );

    let mut app = yoctui_model::App::new(8, 1_000);
    app.build.status = yoctui_model::BuildStatus::Parsing;
    let _ = yoctui_model::update(&mut app, action);
    let progress = app.progress_hierarchy_at(std::time::SystemTime::UNIX_EPOCH);
    assert_eq!(progress.parse.fraction.unwrap().exact_text(), "8/20 (40%)");
    assert_eq!(
        progress.runqueue.state,
        yoctui_model::WidgetState::Unavailable
    );
    assert_eq!(
        progress.sstate.state,
        yoctui_model::WidgetState::Unavailable
    );
}
