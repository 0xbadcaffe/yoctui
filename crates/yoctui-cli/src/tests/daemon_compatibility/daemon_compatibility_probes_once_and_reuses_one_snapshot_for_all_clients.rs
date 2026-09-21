use super::*;

#[tokio::test]
async fn daemon_compatibility_probes_once_and_reuses_one_snapshot_for_all_clients() {
    let build = temporary_build("reuse");
    let environment = environment(build.clone(), "2.18.0");
    let key = key(environment.clone(), "poky-current");
    let mut coordinator = DaemonCompatibilityCoordinator::default();
    let DaemonCompatibilitySelection::Probe(ticket) =
        coordinator.select_environment(key.clone()).unwrap()
    else {
        panic!("first environment selection must probe");
    };
    let resolved = coordinator
        .probe(ticket, &context(environment))
        .await
        .unwrap();
    assert_eq!(
        resolved.snapshot.capabilities.len(),
        CapabilityId::ALL.len()
    );

    let DaemonCompatibilitySelection::Cached(first_client) =
        coordinator.select_environment(key.clone()).unwrap()
    else {
        panic!("exact reconnect must reuse the daemon snapshot");
    };
    let DaemonCompatibilitySelection::Cached(second_client) =
        coordinator.select_environment(key).unwrap()
    else {
        panic!("second client must see the same daemon snapshot");
    };
    assert_eq!(first_client, second_client);
    assert_eq!(first_client, resolved);
    fs::remove_dir_all(build).unwrap();
}
