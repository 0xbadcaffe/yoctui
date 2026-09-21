use super::*;

fn kickstart() -> WicKickstart {
    WicKickstart {
        identity: WicKickstartIdentity {
            name: "directdisk".into(),
            path: Some("/layers/meta/wic/directdisk.wks".into()),
        },
        source: "part / --source rootfs --fstype=ext4 --size=64".into(),
        partitions: vec![WicPartitionSummary {
            mount_point: Some("/".into()),
            filesystem: Some("ext4".into()),
            source_plugin: Some("rootfs".into()),
            size_mib: Some(64),
            alignment_kib: None,
        }],
        limitations: Vec::new(),
    }
}

fn capability() -> WicCapability {
    WicCapability::Available {
        executable: "/opt/poky/scripts/wic".into(),
        kickstarts: vec![kickstart()],
        image_targets: vec!["core-image-minimal".into()],
    }
}

mod wic_model_creation_preview_is_exact_and_rejects_stale_or_unsafe_identity;

mod wic_device_write_phrase_and_inventory_bounds_are_enforced;
