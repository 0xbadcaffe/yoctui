#[test]
fn task_identity_statistics_decode_without_recipe_or_task_inference() {
    let event = serde_json::from_str::<super::Event>(
        r#"{"type":"task_stats","stats":{"completed":3,"total":10,"active":1,"failed":0}}"#,
    )
    .unwrap();
    assert!(matches!(
        super::BridgeBackend::event(event).unwrap(),
        super::BackendEvent::TaskStats(super::TaskStats {
            completed: 3,
            total: 10,
            active: 1,
            failed: 0
        })
    ));
}
