use super::*;

#[tokio::test]
async fn raw_capability_probe_environment_change_rejects_stale_result() {
    let first_build = temporary_build("first");
    let second_build = temporary_build("second");
    let first_environment = environment(first_build.clone(), "1.52.0");
    let second_environment = environment(second_build.clone(), "2.18.0");
    let mut coordinator = DaemonCompatibilityCoordinator::default();
    let DaemonCompatibilitySelection::Probe(stale) = coordinator
        .select_environment(key(first_environment, "poky-old"))
        .unwrap()
    else {
        panic!("first selection must probe");
    };
    let DaemonCompatibilitySelection::Probe(current) = coordinator
        .select_environment(key(second_environment.clone(), "poky-new"))
        .unwrap()
    else {
        panic!("changed environment must probe");
    };

    let current_snapshot = coordinator
        .probe(current, &context(second_environment))
        .await
        .unwrap();
    assert!(matches!(
        coordinator.accept(
            stale.clone(),
            current_snapshot.snapshot.clone(),
            current_snapshot.implementations.clone()
        ),
        Err(DaemonCompatibilityError::StaleProbe)
    ));
    assert!(matches!(
        coordinator.select_environment(stale.key).unwrap(),
        DaemonCompatibilitySelection::Probe(_)
    ));
    assert_eq!(coordinator.invalidate().unwrap(), 4);
    fs::remove_dir_all(first_build).unwrap();
    fs::remove_dir_all(second_build).unwrap();
}
