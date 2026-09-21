#[test]
fn cache_bridge_only_promotes_native_unscoped_info() {
    for (level, recipe, expected) in [
        ("info", None, true),
        ("warning", None, false),
        ("info", Some("recipe"), false),
    ] {
        let event = crate::BridgeBackend::event(yoctui_protocol::Event::Log {
                level: level.into(),
                message: "Sstate summary: Wanted 10 Local 3 Mirrors 2 Missed 5 Current 8 (50% match, 72% complete)".into(),
                recipe: recipe.map(str::to_owned), task: None, path: None,
            }).unwrap();
        assert_eq!(
            matches!(event, crate::BackendEvent::SstateSummary(_)),
            expected
        );
    }
}
