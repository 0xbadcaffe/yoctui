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

#[test]
fn bridge_preserves_critical_and_fatal_diagnostics_as_errors() {
    for level in ["critical", "fatal"] {
        let event = crate::BridgeBackend::event(yoctui_protocol::Event::Log {
            level: level.into(),
            message: "build failed".into(),
            recipe: Some("obmc-phosphor-image".into()),
            task: Some("do_image_complete".into()),
            path: Some("/tmp/log.do_image_complete.42".into()),
        })
        .unwrap();
        let crate::BackendEvent::Log(entry) = event else {
            panic!("diagnostic was not retained as a log entry");
        };
        assert_eq!(entry.severity, yoctui_model::Severity::Error);
        assert_eq!(
            entry.path.as_deref(),
            Some(std::path::Path::new("/tmp/log.do_image_complete.42"))
        );
    }
}
