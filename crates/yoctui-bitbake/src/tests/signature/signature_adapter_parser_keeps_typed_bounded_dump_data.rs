use super::*;

#[test]
fn signature_adapter_parser_keeps_typed_bounded_dump_data() {
    let identity = SignatureIdentity {
        target: target(),
        hash: Some("aaa".into()),
        path: Some("/build/tmp/stamps/qemux86_64/busybox/1.0.do_compile.sigdata.aaa".into()),
    };
    let (record, limitations) = parse_signature_dump(&identity, &fixture("aaa")).unwrap();
    assert!(limitations.is_empty());
    assert_eq!(record.base_hash.as_deref(), Some("base-aaa"));
    assert_eq!(record.task_hash.as_deref(), Some("aaa"));
    assert_eq!(record.variables.len(), 2);
    assert_eq!(
        record.variables[1].value.as_deref(),
        Some("line one\nline two")
    );
    assert_eq!(record.dependencies, vec!["busybox:do_configure=dep-aaa"]);

    assert!(matches!(
        parse_signature_dump(&identity, "nonsense"),
        Err(SignatureAdapterError::Malformed(_))
    ));
}
