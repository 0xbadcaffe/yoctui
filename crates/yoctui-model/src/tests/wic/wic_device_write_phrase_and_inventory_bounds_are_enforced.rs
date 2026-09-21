use super::*;

#[test]
fn wic_device_write_phrase_and_inventory_bounds_are_enforced() {
    let image = WicOutputIdentity {
        path: "/build/out/image.wic".into(),
        size_bytes: 1024,
        modified_unix_seconds: 1,
    };
    let device = WicDevice {
        identity: WicDeviceIdentity {
            path: "/dev/sdz".into(),
            major_minor: "8:240".into(),
            size_bytes: 2048,
            model: Some("test".into()),
            serial: None,
            transport: Some("usb".into()),
        },
        removable: true,
        writable: true,
        read_only: false,
        descendant_mounts: Vec::new(),
        unavailable_reason: None,
    };
    assert!(WicWritePreview::new(&capability(), image.clone(), &device, "WRITE /dev/sdy").is_err());
    let preview = WicWritePreview::new(&capability(), image, &device, "WRITE /dev/sdz").unwrap();
    assert_eq!(
        preview.argv,
        vec![
            PathBuf::from("/opt/poky/scripts/wic"),
            "write".into(),
            "/build/out/image.wic".into(),
            "/dev/sdz".into(),
        ]
    );
    let mut mounted = device;
    mounted.descendant_mounts.push("/media/card".into());
    assert!(
        mounted
            .eligible_for(&preview.request.image)
            .unwrap_err()
            .contains("mounted")
    );
    let mut malformed = mounted.identity;
    malformed.major_minor = "8:".into();
    assert!(malformed.validate().is_err());
    let mut phrase = WicWritePhraseDialog {
        request: WicDeviceInventoryRequest {
            generation: 1,
            image: preview.request.image.clone(),
        },
        device: preview.request.device.clone(),
        input: String::new(),
        validation_error: Some("old".into()),
    };
    phrase.append('\n');
    for _ in 0..(MAX_WIC_WRITE_PHRASE_INPUT_BYTES + 10) {
        phrase.append('x');
    }
    assert_eq!(phrase.input.len(), MAX_WIC_WRITE_PHRASE_INPUT_BYTES);
    assert!(phrase.validation_error.is_none());
    phrase.backspace();
    assert_eq!(phrase.input.len(), MAX_WIC_WRITE_PHRASE_INPUT_BYTES - 1);

    let outputs = (0..(MAX_WIC_OUTPUTS + 10))
        .map(|index| WicOutput {
            identity: WicOutputIdentity {
                path: PathBuf::from(format!("/build/out/{index}.wic")),
                size_bytes: index as u64,
                modified_unix_seconds: 1,
            },
            kind: WicOutputKind::Wic,
        })
        .collect();
    assert_eq!(
        normalize_wic_outputs(Path::new("/build/out"), outputs)
            .unwrap()
            .len(),
        MAX_WIC_OUTPUTS
    );
}
