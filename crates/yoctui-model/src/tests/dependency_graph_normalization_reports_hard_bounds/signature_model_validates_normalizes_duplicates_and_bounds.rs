use super::*;

#[test]
fn signature_model_validates_normalizes_duplicates_and_bounds() {
    let target = SignatureTarget {
        recipe: "busybox".into(),
        task: "do_compile".into(),
    };
    let mut preferred = signature_record("busybox", "do_compile", "aaa", "/tmp/aaa.sigdata");
    preferred.variables = vec![
        SignatureValue {
            name: "Z".into(),
            value: Some("last".into()),
        },
        SignatureValue {
            name: "A".into(),
            value: Some("first".into()),
        },
        SignatureValue {
            name: "A".into(),
            value: Some("second".into()),
        },
    ];
    preferred.dependencies = vec!["z".into(), "a".into(), "a".into()];
    let mut duplicate = preferred.clone();
    duplicate.base_hash = Some("zzz".into());
    let invalid = signature_record("other", "do_compile", "bad", "/tmp/bad.sigdata");
    let overflow = signature_record("busybox", "do_compile", "ccc", "/tmp/ccc.sigdata");

    let (records, report) =
        normalize_signature_records(&target, vec![duplicate, invalid, overflow, preferred], 1);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].base_hash.as_deref(), Some("base-aaa"));
    assert_eq!(
        records[0].variables,
        [
            SignatureValue {
                name: "A".into(),
                value: Some("first".into())
            },
            SignatureValue {
                name: "Z".into(),
                value: Some("last".into())
            }
        ]
    );
    assert_eq!(records[0].dependencies, ["a", "z"]);
    assert_eq!(report.duplicate_records, 1);
    assert_eq!(report.invalid_records, 1);
    assert_eq!(report.truncated_records, 1);
    assert!(report.is_partial());

    let relative = SignatureIdentity {
        target,
        hash: Some("abc".into()),
        path: Some(PathBuf::from("relative.sigdata")),
    };
    assert_eq!(relative.validate(), Err("signature paths must be absolute"));
}
