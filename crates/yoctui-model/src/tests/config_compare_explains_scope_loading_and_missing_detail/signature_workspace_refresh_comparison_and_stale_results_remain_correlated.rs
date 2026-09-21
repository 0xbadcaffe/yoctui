use super::*;

#[test]
fn signature_workspace_refresh_comparison_and_stale_results_remain_correlated() {
    let target = SignatureTarget {
        recipe: "busybox".into(),
        task: "do_compile".into(),
    };
    let left = signature_record(
        "busybox",
        "do_compile",
        "aaa",
        "/build/tmp/stamps/busybox/do_compile.sigdata.aaa",
    );
    let right = signature_record(
        "busybox",
        "do_compile",
        "bbb",
        "/build/tmp/stamps/busybox/do_compile.sigdata.bbb",
    );
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Signatures;
    let _ = update(&mut app, Action::BeginSignatureDump(target.clone()));
    let _ = update(
        &mut app,
        Action::SignatureDumpLoaded {
            target: target.clone(),
            records: vec![left.clone(), right.clone()],
        },
    );
    let _ = update(
        &mut app,
        Action::SetSelectedSignatureComparisonSide(SignatureComparisonSide::Left),
    );
    let _ = update(&mut app, Action::SelectSignatureRecord { delta: 1 });
    let _ = update(
        &mut app,
        Action::SetSelectedSignatureComparisonSide(SignatureComparisonSide::Right),
    );
    let request = SignatureComparisonRequest {
        left: left.identity,
        right: right.identity,
    };
    assert_eq!(
        update(&mut app, Action::BeginSignatureComparison),
        Some(Effect::CompareSignatures(request.clone()))
    );
    assert_eq!(
        update(&mut app, Action::RefreshSignatureDump),
        None,
        "refresh is inert while a comparison is loading"
    );
    let stale = SignatureComparisonRequest {
        left: request.right.clone(),
        right: request.left.clone(),
    };
    let _ = update(
        &mut app,
        Action::SignatureComparisonLoaded {
            request: stale,
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
            request,
            differences: Vec::new(),
        },
    );
    assert!(matches!(
        app.signature_comparison,
        SignatureComparisonState::AvailableEmpty { .. }
    ));
    assert_eq!(
        update(&mut app, Action::RefreshSignatureDump),
        Some(Effect::GetSignatureDump(target))
    );
}
