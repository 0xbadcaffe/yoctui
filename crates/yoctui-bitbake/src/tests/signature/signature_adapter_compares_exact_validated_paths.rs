use super::*;

#[tokio::test]
async fn signature_adapter_compares_exact_validated_paths() {
    let directory = TestDirectory::new("compare");
    let left_path = signature_path(directory.path(), "aaa");
    let right_path = signature_path(directory.path(), "bbb");
    let dump = directory.path().join("dump");
    let diff = directory.path().join("diff");
    write_executable(
        &dump,
        &format!(
            "#!/bin/sh\ncase \"$1\" in\n*aaa) printf '%s' '{}';;\n*bbb) printf '%s' '{}';;\n*) exit 9;;\nesac\n",
            fixture("aaa").replace('\'', "'\\''"),
            fixture("bbb")
                .replace("Variable CC value is gcc", "Variable CC value is clang")
                .replace('\'', "'\\''")
        ),
    );
    write_executable(
        &diff,
        &format!(
            "#!/bin/sh\n[ \"$1\" = '-c' ] || exit 8\n[ \"$2\" = 'never' ] || exit 9\n[ \"$3\" = '{}' ] || exit 10\n[ \"$4\" = '{}' ] || exit 11\nprintf '%s\\n' \"basehash changed from base-aaa to base-bbb\"\n",
            left_path.display(),
            right_path.display()
        ),
    );
    let request = SignatureComparisonRequest {
        left: identity_from_path(&target(), left_path).unwrap(),
        right: identity_from_path(&target(), right_path).unwrap(),
    };
    let response = test_adapter(directory.path(), dump, diff)
        .compare(request.clone())
        .await
        .unwrap();
    assert_eq!(response.request, request);
    assert!(response.limitations.is_empty());
    assert!(response.differences.iter().any(|difference| {
        difference.category == SignatureDifferenceCategory::ChangedValue && difference.key == "CC"
    }));
}
