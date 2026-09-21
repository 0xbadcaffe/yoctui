use super::*;
use std::sync::atomic::{AtomicU64, Ordering};
use yoctui_model::{WicCompression, WicCreateDraft};

static FIXTURE_ID: AtomicU64 = AtomicU64::new(1);

fn fixture(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "yoctui-wic-capability-{}-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        FIXTURE_ID.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&path).unwrap();
    fs::canonicalize(path).unwrap()
}

#[cfg(unix)]
fn executable(path: &Path, body: &str) {
    crate::test_support::write_executable(path, &format!("#!/bin/sh\n{body}\n"));
}

#[cfg(unix)]
mod wic_async_spawn_retries_only_transient_text_file_busy;

#[cfg(unix)]
mod wic_adapter_capability_discovers_parses_and_constructs_exact_command;

#[cfg(unix)]
mod wic_adapter_capability_reports_missing_malformed_and_unsafe_sources;

#[cfg(unix)]
async fn runner_fixture(name: &str, body: &str) -> (PathBuf, PathBuf, WicCreateCommandSpec) {
    let directory = fixture(name);
    let program = directory.join("wic");
    executable(&program, body);
    let kickstart_path = directory.join("directdisk.wks");
    fs::write(&kickstart_path, "part / --source=rootfs\n").unwrap();
    let kickstart_path = fs::canonicalize(kickstart_path).unwrap();
    let output = directory.join("output");
    fs::create_dir(&output).unwrap();
    let output = fs::canonicalize(output).unwrap();
    let capability = WicCapability::Available {
        executable: fs::canonicalize(program).unwrap(),
        kickstarts: vec![read_kickstart(&kickstart_path, Some("directdisk".into())).unwrap()],
        image_targets: vec!["core-image-minimal".into()],
    };
    let preview = WicCreateDraft {
        machine: "qemux86-64".into(),
        image: "core-image-minimal".into(),
        kickstart: WicKickstartIdentity {
            name: "directdisk".into(),
            path: Some(kickstart_path),
        },
        output_directory: output.display().to_string(),
        generate_bmap: false,
        compression: WicCompression::None,
    }
    .preview(&capability)
    .unwrap();
    let command = WicCreateCommandSpec::from_preview(&preview, &capability).unwrap();
    (directory, output, command)
}

#[cfg(unix)]
mod wic_adapter_runner_reports_only_new_outputs_and_nonzero_failure;

#[cfg(unix)]
mod wic_adapter_runner_rejects_duplicate_and_forces_cancellation;

#[cfg(unix)]
mod wic_adapter_runner_times_out_without_blocking_forever;

fn lsblk_node(
    path: &str,
    kind: &str,
    major_minor: &str,
    size: u64,
    access: (bool, bool),
    mounts: Vec<&str>,
    children: Vec<serde_json::Value>,
) -> serde_json::Value {
    let (removable, read_only) = access;
    serde_json::json!({
        "path": path,
        "type": kind,
        "maj:min": major_minor,
        "size": size,
        "model": if kind == "disk" { serde_json::Value::String("fixture".into()) } else { serde_json::Value::Null },
        "serial": if kind == "disk" { serde_json::Value::String(format!("serial-{major_minor}")) } else { serde_json::Value::Null },
        "tran": if removable { serde_json::Value::String("usb".into()) } else { serde_json::Value::Null },
        "rm": removable,
        "ro": read_only,
        "mountpoints": mounts,
        "children": children,
    })
}

fn device_inventory_json(extra: Vec<serde_json::Value>) -> String {
    let root_partition = lsblk_node(
        "/dev/sda1",
        "part",
        "8:1",
        4_096,
        (false, false),
        vec!["/"],
        Vec::new(),
    );
    let mut devices = vec![lsblk_node(
        "/dev/sda",
        "disk",
        "8:0",
        8_192,
        (false, false),
        Vec::new(),
        vec![root_partition],
    )];
    devices.extend(extra);
    serde_json::json!({ "blockdevices": devices }).to_string()
}

#[cfg(unix)]
fn device_write_fixture(
    name: &str,
    inventory: &str,
    wic_body: &str,
) -> (
    PathBuf,
    PathBuf,
    WicDeviceInspector,
    WicDeviceInventoryRequest,
    PathBuf,
) {
    let directory = fixture(name);
    let image = directory.join("image.wic");
    fs::write(&image, b"image").unwrap();
    let image = fs::canonicalize(image).unwrap();
    let lsblk = directory.join("lsblk");
    executable(
        &lsblk,
        &format!(
            "test \"$#\" -eq 5 && test \"$1\" = --json && test \"$2\" = --bytes && test \"$3\" = --paths && test \"$4\" = --output && test \"$5\" = 'PATH,TYPE,MAJ:MIN,SIZE,MODEL,SERIAL,TRAN,RM,RO,MOUNTPOINTS' || exit 64\nprintf '%s' '{}'",
            inventory.replace('\'', "'\\''")
        ),
    );
    let wic = directory.join("wic");
    executable(&wic, wic_body);
    let request = WicDeviceInventoryRequest {
        generation: 1,
        image: WicOutputIdentity {
            path: image,
            size_bytes: 5,
            modified_unix_seconds: 1,
        },
    };
    (
        directory,
        fs::canonicalize(wic).unwrap(),
        WicDeviceInspector::with_program(fs::canonicalize(&lsblk).unwrap())
            .without_device_node_validation_for_tests(),
        request,
        lsblk,
    )
}

#[cfg(unix)]
mod wic_device_write_discovers_safe_whole_device_and_builds_exact_argv;

#[cfg(unix)]
mod wic_device_write_fails_closed_for_exclusions_duplicates_and_stale_identity;

#[cfg(unix)]
mod wic_device_write_discovery_bounds_process_failures_and_timeouts;

#[cfg(unix)]
mod wic_device_write_runner_streams_bounds_fails_and_cancels;
