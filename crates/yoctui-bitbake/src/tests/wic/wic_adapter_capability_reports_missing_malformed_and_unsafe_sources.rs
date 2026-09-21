use super::*;

#[tokio::test]
async fn wic_adapter_capability_reports_missing_malformed_and_unsafe_sources() {
    assert_eq!(
        WicCapabilityInspector::with_executable("/missing/wic".into())
            .inspect(vec!["core-image-minimal".into()])
            .await,
        WicCapability::MissingTool
    );
    let directory = fixture("unsafe");
    let program = directory.join("wic");
    executable(&program, "printf 'bad/name malformed\\n'");
    assert!(matches!(
        WicCapabilityInspector::with_executable(program.clone())
            .inspect(vec!["core-image-minimal".into()])
            .await,
        WicCapability::Failed { .. }
    ));
    let target = directory.join("target.wks");
    fs::write(&target, "part /\n").unwrap();
    let link = directory.join("linked.wks");
    std::os::unix::fs::symlink(&target, &link).unwrap();
    executable(&program, "exit 0");
    assert!(matches!(
        WicCapabilityInspector::with_executable(program)
            .with_sources(vec![link], Vec::new())
            .inspect(vec!["core-image-minimal".into()])
            .await,
        WicCapability::Failed { .. }
    ));
    fs::remove_dir_all(directory).unwrap();
}
