use super::*;

#[test]
fn typed_event_preserves_unknown_progress_and_ignores_future_events() {
    assert!(matches!(
        BridgeBackend::event(Event::TaskProgress {
            recipe: "busybox".into(),
            task: "do_compile".into(),
            progress: None,
        })
        .unwrap(),
        BackendEvent::TaskProgress { progress: None, .. }
    ));
    assert!(matches!(
        BridgeBackend::event(Event::Unknown).unwrap(),
        BackendEvent::Ignored
    ));
    assert!(matches!(
        BridgeBackend::event(Event::BridgeShutdown).unwrap(),
        BackendEvent::Disconnected
    ));
}
