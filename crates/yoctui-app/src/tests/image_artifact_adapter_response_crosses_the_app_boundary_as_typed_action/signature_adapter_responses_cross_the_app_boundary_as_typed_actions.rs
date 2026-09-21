use super::*;

#[test]
fn signature_adapter_responses_cross_the_app_boundary_as_typed_actions() {
    let target = SignatureTarget {
        recipe: "busybox".into(),
        task: "do_compile".into(),
    };
    let identity = SignatureIdentity {
        target: target.clone(),
        hash: Some("aaa".into()),
        path: Some("/build/tmp/stamps/busybox/do_compile.sigdata.aaa".into()),
    };
    let record = SignatureRecord {
        identity: identity.clone(),
        base_hash: Some("base-aaa".into()),
        task_hash: Some("aaa".into()),
        variables: Vec::new(),
        dependencies: Vec::new(),
    };
    let event: BackendEvent = yoctui_bitbake::SignatureDumpResponse {
        target: target.clone(),
        records: vec![record.clone()],
        limitations: vec!["one malformed historical signature was omitted".into()],
    }
    .into();
    assert_eq!(
        model_action_from_backend_event(event),
        Some(Action::SignatureDumpPartial {
            target,
            records: vec![record],
            limitations: vec!["one malformed historical signature was omitted".into()],
        })
    );

    let request = SignatureComparisonRequest {
        left: identity.clone(),
        right: SignatureIdentity {
            hash: Some("bbb".into()),
            ..identity
        },
    };
    let difference = SignatureDifference {
        category: SignatureDifferenceCategory::BaseHash,
        key: "base_hash".into(),
        left: Some("base-aaa".into()),
        right: Some("base-bbb".into()),
    };
    let event: BackendEvent = yoctui_bitbake::SignatureComparisonResponse {
        request: request.clone(),
        differences: vec![difference.clone()],
        limitations: Vec::new(),
    }
    .into();
    assert_eq!(
        model_action_from_backend_event(event),
        Some(Action::SignatureComparisonLoaded {
            request,
            differences: vec![difference],
        })
    );
}
