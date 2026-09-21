use super::*;

#[test]
fn signature_model_reducer_correlates_states_selection_and_comparison() {
    let target = SignatureTarget {
        recipe: "busybox".into(),
        task: "do_compile".into(),
    };
    let left = signature_record("busybox", "do_compile", "aaa", "/tmp/aaa.sigdata");
    let right = signature_record("busybox", "do_compile", "bbb", "/tmp/bbb.sigdata");
    let mut app = App::new(10, 1_000);
    assert_eq!(
        update(&mut app, Action::BeginSignatureDump(target.clone())),
        Some(Effect::GetSignatureDump(target.clone()))
    );
    let stale_target = SignatureTarget {
        recipe: "other".into(),
        task: "do_compile".into(),
    };
    let _ = update(
        &mut app,
        Action::SignatureDumpLoaded {
            target: stale_target,
            records: vec![left.clone()],
        },
    );
    assert!(matches!(
        app.signature_dump,
        SignatureDumpState::Loading { .. }
    ));
    let _ = update(
        &mut app,
        Action::SignatureDumpLoaded {
            target: target.clone(),
            records: vec![right.clone(), left.clone()],
        },
    );
    assert_eq!(app.signature_selection, Some(left.identity.clone()));
    assert!(matches!(
        app.signature_dump,
        SignatureDumpState::Available { .. }
    ));

    let _ = update(
        &mut app,
        Action::SetSelectedSignatureComparisonSide(SignatureComparisonSide::Left),
    );
    let _ = update(&mut app, Action::SelectSignatureRecord { delta: 1 });
    assert_eq!(app.signature_selection, Some(right.identity.clone()));
    let _ = update(
        &mut app,
        Action::SetSelectedSignatureComparisonSide(SignatureComparisonSide::Right),
    );
    let request = SignatureComparisonRequest {
        left: left.identity.clone(),
        right: right.identity.clone(),
    };
    assert_eq!(
        update(&mut app, Action::BeginSignatureComparison),
        Some(Effect::CompareSignatures(request.clone()))
    );
    let stale_request = SignatureComparisonRequest {
        left: right.identity.clone(),
        right: left.identity.clone(),
    };
    let _ = update(
        &mut app,
        Action::SignatureComparisonLoaded {
            request: stale_request,
            differences: Vec::new(),
        },
    );
    assert!(matches!(
        app.signature_comparison,
        SignatureComparisonState::Loading { .. }
    ));
    let _ = update(
        &mut app,
        Action::SignatureComparisonLoaded {
            request: request.clone(),
            differences: vec![SignatureDifference {
                category: SignatureDifferenceCategory::ChangedValue,
                key: "CC".into(),
                left: Some("gcc".into()),
                right: Some("clang".into()),
            }],
        },
    );
    assert!(matches!(
        app.signature_comparison,
        SignatureComparisonState::Available { .. }
    ));
    let _ = update(
        &mut app,
        Action::SetSelectedSignatureComparisonSide(SignatureComparisonSide::Left),
    );
    assert!(matches!(
        app.signature_comparison,
        SignatureComparisonState::Ready { .. }
    ));

    let _ = update(&mut app, Action::BeginSignatureDump(target.clone()));
    let _ = update(
        &mut app,
        Action::SignatureDumpPartial {
            target: target.clone(),
            records: vec![right.clone()],
            limitations: vec!["one artifact unreadable".into()],
        },
    );
    assert_eq!(app.signature_selection, Some(right.identity));
    assert!(matches!(
        app.signature_dump,
        SignatureDumpState::Partial { .. }
    ));
    let _ = update(&mut app, Action::BeginSignatureDump(target.clone()));
    let _ = update(
        &mut app,
        Action::SignatureDumpLoaded {
            target: target.clone(),
            records: Vec::new(),
        },
    );
    assert_eq!(
        app.signature_dump,
        SignatureDumpState::AvailableEmpty {
            target: target.clone()
        }
    );
    let _ = update(&mut app, Action::BeginSignatureDump(target.clone()));
    let _ = update(
        &mut app,
        Action::SignatureDumpFailed {
            target: target.clone(),
            message: "tool unavailable".into(),
        },
    );
    assert_eq!(
        app.signature_dump,
        SignatureDumpState::Failed {
            target,
            message: "tool unavailable".into()
        }
    );
}
