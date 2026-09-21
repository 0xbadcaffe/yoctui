use super::*;

#[test]
fn archive_rejects_future_schema_and_unbounded_records() {
    assert!(
        BuildArchive {
            schema_version: 2,
            builds: Vec::new()
        }
        .validate()
        .is_err()
    );
    let record = SavedBuild {
        id: "one".into(),
        target: "image".into(),
        machine: None,
        source: None,
        build_dir: None,
        outcome: yoctui_model::SavedBuildOutcome::Incomplete,
        saved_unix_ms: 0,
        started_unix_ms: None,
        finished_unix_ms: None,
        logs: Vec::new(),
        tasks: Vec::new(),
        limitations: Vec::new(),
    };
    let mut archive = BuildArchive {
        schema_version: 1,
        builds: vec![record.clone(); 33],
    };
    assert!(archive.validate().is_err());
    archive.builds = vec![record];
    archive.builds[0].target = "x".repeat(4097);
    assert!(archive.validate().is_err());
    archive.builds[0].target = "unsafe\x1b".into();
    assert!(archive.validate().is_err());
}
