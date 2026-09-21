use super::*;

#[tokio::test]
async fn wic_device_write_fails_closed_for_exclusions_duplicates_and_stale_identity() {
    let candidates = vec![
        lsblk_node(
            "/dev/sdb",
            "disk",
            "8:16",
            16_384,
            (false, false),
            Vec::new(),
            Vec::new(),
        ),
        lsblk_node(
            "/dev/sdc",
            "disk",
            "8:32",
            16_384,
            (true, true),
            Vec::new(),
            Vec::new(),
        ),
        lsblk_node(
            "/dev/sdd",
            "disk",
            "8:48",
            16_384,
            (true, false),
            vec!["/media/card"],
            Vec::new(),
        ),
        lsblk_node(
            "/dev/sde",
            "disk",
            "8:64",
            1,
            (true, false),
            Vec::new(),
            Vec::new(),
        ),
        lsblk_node(
            "/dev/sdf",
            "disk",
            "8:80",
            16_384,
            (true, false),
            Vec::new(),
            Vec::new(),
        ),
        lsblk_node(
            "/dev/sdf1",
            "part",
            "8:81",
            16_384,
            (true, false),
            Vec::new(),
            Vec::new(),
        ),
        lsblk_node(
            "/dev/sr0",
            "rom",
            "11:0",
            16_384,
            (true, true),
            Vec::new(),
            Vec::new(),
        ),
        lsblk_node(
            "/dev/dm-0",
            "lvm",
            "253:0",
            16_384,
            (false, false),
            Vec::new(),
            Vec::new(),
        ),
        lsblk_node(
            "/dev/sdz",
            "disk",
            "8:240",
            16_384,
            (true, false),
            Vec::new(),
            Vec::new(),
        ),
    ];
    let inventory = device_inventory_json(candidates);
    let (directory, wic, inspector, request, lsblk) =
        device_write_fixture("device-rejections", &inventory, "exit 0");
    let inspector = inspector.with_unwritable_device("/dev/sdf".into());
    let response = inspector.discover(request.clone()).await.unwrap();
    assert_eq!(response.devices.len(), 1);
    for expected in [
        "not removable",
        "read-only",
        "mounted descendants",
        "smaller",
        "cannot be opened",
        "device type part",
        "device type rom",
        "device type lvm",
    ] {
        assert!(
            response
                .limitations
                .iter()
                .any(|limitation| limitation.contains(expected)),
            "{expected}: {:?}",
            response.limitations
        );
    }
    let write_request = WicWriteRequest {
        executable: wic,
        image: request.image.clone(),
        device: response.devices[0].identity.clone(),
    };
    let mut changed_identity_records = Vec::new();
    for (field, value) in [
        ("maj:min", serde_json::json!("8:241")),
        ("size", serde_json::json!(32_768)),
        ("model", serde_json::json!("replacement")),
        ("serial", serde_json::json!("replacement")),
        ("tran", serde_json::json!("mmc")),
        ("rm", serde_json::json!(false)),
        ("ro", serde_json::json!(true)),
        ("mountpoints", serde_json::json!(["/media/stale"])),
    ] {
        let mut node = lsblk_node(
            "/dev/sdz",
            "disk",
            "8:240",
            16_384,
            (true, false),
            Vec::new(),
            Vec::new(),
        );
        node[field] = value;
        changed_identity_records.push(device_inventory_json(vec![node]));
    }
    for changed in changed_identity_records {
        executable(
            &lsblk,
            &format!("printf '%s' '{}'", changed.replace('\'', "'\\''")),
        );
        assert_eq!(
            inspector.command_for(&write_request).await.unwrap_err(),
            WicAdapterError::StaleDevice
        );
    }

    let duplicate = device_inventory_json(vec![
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
            "/dev/sdz",
            "disk",
            "8:240",
            16_384,
            (true, false),
            Vec::new(),
            Vec::new(),
        ),
    ]);
    assert!(
        parse_lsblk_devices(
            duplicate.as_bytes(),
            &request.image,
            false,
            &BTreeSet::new()
        )
        .is_err()
    );
    assert!(
        parse_lsblk_devices(
            br#"{"blockdevices":[{"path":"/dev/sda","type":"disk"}]}"#,
            &request.image,
            false,
            &BTreeSet::new()
        )
        .is_err()
    );
    let mut malformed_non_disk = lsblk_node(
        "/dev/loop0",
        "loop",
        "7:0",
        16_384,
        (false, false),
        Vec::new(),
        Vec::new(),
    );
    malformed_non_disk["rm"] = serde_json::json!("maybe");
    assert!(
        parse_lsblk_devices(
            device_inventory_json(vec![malformed_non_disk]).as_bytes(),
            &request.image,
            false,
            &BTreeSet::new()
        )
        .is_err()
    );
    let duplicate_major_minor = device_inventory_json(vec![
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
            "/dev/sdy",
            "disk",
            "8:240",
            16_384,
            (true, false),
            Vec::new(),
            Vec::new(),
        ),
    ]);
    assert!(
        parse_lsblk_devices(
            duplicate_major_minor.as_bytes(),
            &request.image,
            false,
            &BTreeSet::new()
        )
        .is_err()
    );
    let no_root = serde_json::json!({
        "blockdevices": [lsblk_node(
            "/dev/sdz", "disk", "8:240", 16_384, (true, false), Vec::new(), Vec::new()
        )]
    })
    .to_string();
    assert!(
        parse_lsblk_devices(no_root.as_bytes(), &request.image, false, &BTreeSet::new()).is_err()
    );
    let ambiguous_root = serde_json::json!({
        "blockdevices": [
            lsblk_node(
                "/dev/sda", "disk", "8:0", 8_192, (false, false), vec!["/"], Vec::new()
            ),
            lsblk_node(
                "/dev/sdb", "disk", "8:16", 8_192, (false, false), vec!["/"], Vec::new()
            )
        ]
    })
    .to_string();
    assert!(
        parse_lsblk_devices(
            ambiguous_root.as_bytes(),
            &request.image,
            false,
            &BTreeSet::new()
        )
        .is_err()
    );
    assert!(
        parse_lsblk_devices(
            &vec![b' '; MAX_WIC_DEVICE_JSON_BYTES as usize + 1],
            &request.image,
            false,
            &BTreeSet::new()
        )
        .is_err()
    );
    assert!(matches!(
        WicDeviceInspector::with_program(directory.join("missing-lsblk"))
            .without_device_node_validation_for_tests()
            .discover(request.clone())
            .await,
        Err(WicAdapterError::MissingDeviceTool(_))
    ));
    let image_link = directory.join("linked-image.wic");
    std::os::unix::fs::symlink(&request.image.path, &image_link).unwrap();
    let mut linked_request = request.clone();
    linked_request.image.path = image_link;
    assert!(matches!(
        inspector.discover(linked_request).await,
        Err(WicAdapterError::UnsafeImage(_))
    ));
    assert!(!validate_device_node(&directory.join("linked-image.wic")));
    executable(
        &lsblk,
        &format!("printf '%s' '{}'", inventory.replace('\'', "'\\''")),
    );
    fs::remove_file(&write_request.executable).unwrap();
    assert!(matches!(
        inspector.command_for(&write_request).await,
        Err(WicAdapterError::UnsafeExecutable(_))
    ));
    fs::write(&request.image.path, b"changed-size").unwrap();
    assert!(matches!(
        inspector.discover(request).await,
        Err(WicAdapterError::UnsafeImage(_))
    ));
    fs::remove_dir_all(directory).unwrap();
}
