use super::*;

#[test]
fn signature_model_derives_deterministic_typed_differences() {
    let mut left = signature_record("busybox", "do_compile", "left", "/tmp/left.sigdata");
    left.base_hash = Some("base-left".into());
    left.task_hash = Some("task-left".into());
    left.variables = vec![
        SignatureValue {
            name: "CC".into(),
            value: Some("gcc".into()),
        },
        SignatureValue {
            name: "ONLY_LEFT".into(),
            value: Some("yes".into()),
        },
    ];
    left.dependencies = vec!["dep-left".into(), "dep-shared".into()];
    let mut right = signature_record("busybox", "do_compile", "right", "/tmp/right.sigdata");
    right.base_hash = Some("base-right".into());
    right.task_hash = None;
    right.variables = vec![SignatureValue {
        name: "CC".into(),
        value: Some("clang".into()),
    }];
    right.dependencies = vec!["dep-right".into(), "dep-shared".into()];

    let (differences, report) = compare_signature_records(&left, &right, 20);
    assert!(!report.is_partial());
    assert!(differences.iter().any(|difference| {
        difference.category == SignatureDifferenceCategory::BaseHash
            && difference.key == "base_hash"
    }));
    assert!(differences.iter().any(|difference| {
        difference.category == SignatureDifferenceCategory::ChangedValue && difference.key == "CC"
    }));
    assert!(differences.iter().any(|difference| {
        difference.category == SignatureDifferenceCategory::Unavailable
            && difference.key == "ONLY_LEFT"
    }));
    assert_eq!(
        differences
            .iter()
            .filter(|difference| { difference.category == SignatureDifferenceCategory::Dependency })
            .count(),
        2
    );
    let (_, bounded) = compare_signature_records(&left, &right, 2);
    assert!(bounded.is_partial());
}
