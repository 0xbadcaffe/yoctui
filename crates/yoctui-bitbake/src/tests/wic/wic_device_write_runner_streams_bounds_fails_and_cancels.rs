use super::*;

#[tokio::test]
async fn wic_device_write_runner_streams_bounds_fails_and_cancels() {
    let inventory = device_inventory_json(vec![lsblk_node(
        "/dev/sdz",
        "disk",
        "8:240",
        16_384,
        (true, false),
        Vec::new(),
        Vec::new(),
    )]);
    let (directory, wic, inspector, request, _) =
        device_write_fixture("device-runner", &inventory, "printf 'writing\\n'; exit 0");
    let response = inspector.discover(request.clone()).await.unwrap();
    let write_request = WicWriteRequest {
        executable: wic,
        image: request.image,
        device: response.devices[0].identity.clone(),
    };
    let mut runner = WicJobRunner::new(directory.clone());
    runner.start_write(&inspector, write_request).await.unwrap();
    assert_eq!(runner.next_event().await.unwrap(), WicRunnerEvent::Starting);
    assert_eq!(runner.next_event().await.unwrap(), WicRunnerEvent::Started);
    let mut saw_output = false;
    loop {
        match runner.next_event().await.unwrap() {
            WicRunnerEvent::Output { line, .. } => saw_output |= line == "writing",
            WicRunnerEvent::Completed { outputs, .. } => {
                assert!(outputs.is_empty());
                break;
            }
            _ => {}
        }
    }
    assert!(saw_output);
    fs::remove_dir_all(directory).unwrap();

    let (directory, wic, inspector, request, _) = device_write_fixture(
        "device-runner-failure",
        &inventory,
        "dd if=/dev/zero bs=70000 count=1 2>/dev/null | tr '\\000' x; printf '\\nfailed\\n' >&2; exit 9",
    );
    let response = inspector.discover(request.clone()).await.unwrap();
    let mut runner = WicJobRunner::new(directory.clone());
    runner
        .start_write(
            &inspector,
            WicWriteRequest {
                executable: wic,
                image: request.image,
                device: response.devices[0].identity.clone(),
            },
        )
        .await
        .unwrap();
    let _ = runner.next_event().await.unwrap();
    let _ = runner.next_event().await.unwrap();
    let mut saw_truncated = false;
    let terminal = loop {
        let event = runner.next_event().await.unwrap();
        match event {
            WicRunnerEvent::Output { truncated, .. } => saw_truncated |= truncated,
            WicRunnerEvent::Failed { .. } => break event,
            _ => {}
        }
    };
    assert!(saw_truncated);
    assert!(matches!(
        terminal,
        WicRunnerEvent::Failed {
            exit_code: Some(9),
            ..
        }
    ));
    fs::remove_dir_all(directory).unwrap();

    let (directory, wic, inspector, request, _) = device_write_fixture(
        "device-runner-cancel",
        &inventory,
        "trap '' TERM; printf 'ready\\n'; while :; do sleep 1; done",
    );
    let response = inspector.discover(request.clone()).await.unwrap();
    let mut runner =
        WicJobRunner::new(directory.clone()).with_cancellation_timeout(Duration::from_millis(50));
    runner
        .start_write(
            &inspector,
            WicWriteRequest {
                executable: wic,
                image: request.image,
                device: response.devices[0].identity.clone(),
            },
        )
        .await
        .unwrap();
    let _ = runner.next_event().await.unwrap();
    let _ = runner.next_event().await.unwrap();
    loop {
        if matches!(
            runner.next_event().await.unwrap(),
            WicRunnerEvent::Output { ref line, .. } if line == "ready"
        ) {
            break;
        }
    }
    assert!(runner.cancel().await.unwrap());
    assert!(matches!(
        runner.next_event().await.unwrap(),
        WicRunnerEvent::Cancelled { forced: true, .. }
    ));
    fs::remove_dir_all(directory).unwrap();
}
