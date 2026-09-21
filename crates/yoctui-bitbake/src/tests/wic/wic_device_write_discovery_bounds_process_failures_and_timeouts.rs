use super::*;

#[tokio::test]
async fn wic_device_write_discovery_bounds_process_failures_and_timeouts() {
    let inventory = device_inventory_json(vec![lsblk_node(
        "/dev/sdz",
        "disk",
        "8:240",
        16_384,
        (true, false),
        Vec::new(),
        Vec::new(),
    )]);
    let (directory, _, inspector, request, lsblk) =
        device_write_fixture("device-discovery-failure", &inventory, "exit 0");
    executable(&lsblk, "printf 'permission denied' >&2; exit 7");
    assert!(matches!(
        inspector.discover(request.clone()).await,
        Err(WicAdapterError::DeviceDiscovery(message))
            if message == "permission denied"
    ));
    executable(&lsblk, "exec sleep 30");
    assert!(matches!(
        inspector
            .with_inspection_timeout(Duration::from_millis(20))
            .discover(request)
            .await,
        Err(WicAdapterError::DeviceDiscovery(message))
            if message == "lsblk timed out"
    ));
    fs::remove_dir_all(directory).unwrap();
}
