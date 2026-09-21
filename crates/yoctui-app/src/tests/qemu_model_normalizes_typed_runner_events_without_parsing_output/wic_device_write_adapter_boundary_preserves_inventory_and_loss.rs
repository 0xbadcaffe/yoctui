use super::*;

#[test]
fn wic_device_write_adapter_boundary_preserves_inventory_and_loss() {
    let request = yoctui_model::WicDeviceInventoryRequest {
        generation: 3,
        image: yoctui_model::WicOutputIdentity {
            path: "/build/output/image.wic".into(),
            size_bytes: 4_096,
            modified_unix_seconds: 7,
        },
    };
    let device = yoctui_model::WicDevice {
        identity: yoctui_model::WicDeviceIdentity {
            path: "/dev/sdz".into(),
            major_minor: "8:240".into(),
            size_bytes: 8_192,
            model: Some("fixture".into()),
            serial: Some("serial".into()),
            transport: Some("usb".into()),
        },
        removable: true,
        writable: true,
        read_only: false,
        descendant_mounts: Vec::new(),
        unavailable_reason: None,
    };
    assert_eq!(
        wic_device_inventory_action(WicDeviceInventoryResponse {
            request: request.clone(),
            devices: vec![device.clone()],
            limitations: vec!["excluded /dev/sda".into()],
        }),
        Action::WicDeviceInventoryLoaded {
            request,
            devices: vec![device],
            limitations: vec!["excluded /dev/sda".into()],
        }
    );
    let id = WicSessionId(11);
    assert_eq!(
        wic_actions_for_runner_event(
            id,
            WicRunnerEvent::Lost {
                message: "write output channel lost".into(),
            },
            SystemTime::UNIX_EPOCH,
        ),
        vec![Action::LoseWicSession {
            id,
            message: "write output channel lost".into(),
            finished_at: SystemTime::UNIX_EPOCH,
        }]
    );
}
