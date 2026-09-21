use super::*;

#[test]
fn signature_model_typed_events_map_dump_comparison_partial_and_failure() {
    let target = SignatureTarget {
        recipe: "busybox".into(),
        task: "do_compile".into(),
    };
    let identity = SignatureIdentity {
        target: target.clone(),
        hash: Some("abc".into()),
        path: Some("/tmp/busybox.sigdata".into()),
    };
    let record = SignatureRecord {
        identity: identity.clone(),
        base_hash: Some("base".into()),
        task_hash: Some("task".into()),
        variables: Vec::new(),
        dependencies: Vec::new(),
    };
    assert_eq!(
        model_action_from_backend_event(BackendEvent::SignatureDump {
            target: target.clone(),
            records: vec![record.clone()],
            limitations: Vec::new(),
        }),
        Some(Action::SignatureDumpLoaded {
            target: target.clone(),
            records: vec![record.clone()],
        })
    );
    assert!(matches!(
        model_action_from_backend_event(BackendEvent::SignatureDump {
            target: target.clone(),
            records: vec![record],
            limitations: vec!["partial".into()],
        }),
        Some(Action::SignatureDumpPartial { .. })
    ));
    let request = SignatureComparisonRequest {
        left: identity.clone(),
        right: SignatureIdentity {
            hash: Some("def".into()),
            path: Some("/tmp/busybox-old.sigdata".into()),
            ..identity
        },
    };
    let difference = SignatureDifference {
        category: SignatureDifferenceCategory::ChangedValue,
        key: "CC".into(),
        left: Some("gcc".into()),
        right: Some("clang".into()),
    };
    assert_eq!(
        model_action_from_backend_event(BackendEvent::SignatureComparison {
            request: request.clone(),
            differences: vec![difference.clone()],
            limitations: Vec::new(),
        }),
        Some(Action::SignatureComparisonLoaded {
            request: request.clone(),
            differences: vec![difference],
        })
    );
    assert_eq!(
        model_action_from_backend_event(BackendEvent::SignatureComparisonFailed {
            request: request.clone(),
            message: "tool failed".into(),
        }),
        Some(Action::SignatureComparisonFailed {
            request,
            message: "tool failed".into(),
        })
    );
}
