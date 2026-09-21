use super::*;

#[test]
fn test_results_imports_official_shape_and_preserves_partial_failures() {
    let (_directory, adapter, _tool, results) = fixture("import");
    let valid = results.join("valid").join("testresults.json");
    let malformed = results.join("malformed").join("testresults.json");
    let empty = results.join("empty").join("testresults.json");
    for path in [&valid, &malformed, &empty] {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
    }
    fs::write(&valid, result_json("PASSED")).unwrap();
    fs::write(&malformed, b"{not json").unwrap();
    fs::write(&empty, b"{}").unwrap();

    let response = imported(&adapter, 7, vec![results]);
    assert_eq!(response.records.len(), 1);
    let record = &response.records[0];
    assert_eq!(record.family, Some(TestFamily::TestImage));
    assert_eq!(record.machine.as_deref(), Some("qemux86-64"));
    assert_eq!(record.image.as_deref(), Some("core-image-minimal"));
    assert_eq!(record.revision.as_deref(), Some("abc123"));
    assert_eq!(record.counts().passed, 1);
    assert_eq!(record.identity.fingerprint.len(), 64);
    assert!(record.is_valid());
    assert_eq!(response.limitations.len(), 2);
    assert!(
        response
            .limitations
            .iter()
            .any(|message| message.contains("malformed"))
    );
    assert!(
        response
            .limitations
            .iter()
            .any(|message| message.contains("no typed result runs"))
    );
    let state = if response.limitations.is_empty() {
        TestResultInventoryState::Available {
            request: response.request,
            records: response.records,
        }
    } else {
        TestResultInventoryState::Partial {
            request: response.request,
            records: response.records,
            limitations: response.limitations,
        }
    };
    assert!(matches!(state, TestResultInventoryState::Partial { .. }));
}
