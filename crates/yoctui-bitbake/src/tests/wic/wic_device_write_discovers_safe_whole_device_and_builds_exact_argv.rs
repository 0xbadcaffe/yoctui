use super::*;

#[tokio::test]
async fn wic_device_write_discovers_safe_whole_device_and_builds_exact_argv() {
    let inventory = device_inventory_json(vec![
        lsblk_node(
            "/dev/sdz",
            "disk",
            "8:240",
            16_384,
            (true, false),
            Vec::new(),
            Vec::new(),
        ),
        lsblk_node(
            "/dev/loop0",
            "loop",
            "7:0",
            16_384,
            (false, false),
            Vec::new(),
            Vec::new(),
        ),
    ]);
    let (directory, wic, inspector, request, _) =
        device_write_fixture("device-exact", &inventory, "printf '%s\\n' \"$@\"; exit 0");
    let response = inspector.discover(request.clone()).await.unwrap();
    assert_eq!(response.request, request);
    assert_eq!(response.devices.len(), 1);
    let device = &response.devices[0];
    assert_eq!(device.identity.path, Path::new("/dev/sdz"));
    assert_eq!(device.identity.major_minor, "8:240");
    assert_eq!(device.identity.serial.as_deref(), Some("serial-8:240"));
    assert!(response.limitations.iter().any(
        |limitation| limitation.contains("/dev/sda") && limitation.contains("root filesystem")
    ));
    assert!(
        response
            .limitations
            .iter()
            .any(|limitation| limitation.contains("device type loop"))
    );
    let write_request = WicWriteRequest {
        executable: wic,
        image: request.image,
        device: device.identity.clone(),
    };
    let command = inspector.command_for(&write_request).await.unwrap();
    assert_eq!(command.executable(), write_request.executable);
    assert_eq!(
        command.arguments(),
        &[
            OsString::from("write"),
            write_request.image.path.as_os_str().to_owned(),
            OsString::from("/dev/sdz"),
        ]
    );
    fs::remove_dir_all(directory).unwrap();
}
