use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    ffi::OsString,
    fs,
    path::{Component, Path, PathBuf},
    process::Stdio,
    time::{Duration, Instant},
};

use crate::{WicRunnerEvent, WicRunnerOutputStream, output_text};
use serde::Deserialize;
use thiserror::Error;
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, BufReader},
    process::{Child, Command},
};
use yoctui_model::{
    MAX_WIC_DEVICE_MOUNTS, MAX_WIC_DEVICES, MAX_WIC_KICKSTARTS, MAX_WIC_LIMITATIONS,
    MAX_WIC_SOURCE_BYTES, WicCapability, WicCreatePreview, WicCreateRequest, WicDevice,
    WicDeviceIdentity, WicDeviceInventoryRequest, WicKickstart, WicKickstartIdentity, WicOutput,
    WicOutputIdentity, WicOutputKind, WicPartitionSummary, WicWriteRequest,
    normalize_wic_capability, normalize_wic_devices, normalize_wic_limitations,
};
use yoctui_utils::is_transient_spawn_error;

const MAX_WIC_LIST_BYTES: u64 = 256 * 1024;
const WIC_INSPECTION_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_WIC_LINE_BYTES: usize = 64 * 1024;
const MAX_WIC_OUTPUT_ENTRIES: usize = 4_096;
const WIC_EVENT_CHANNEL_CAPACITY: usize = 256;
const MAX_WIC_DEVICE_JSON_BYTES: u64 = 1024 * 1024;
const MAX_WIC_DEVICE_RECORDS: usize = 512;
const MAX_WIC_DEVICE_PATH_BYTES: usize = 4_096;
const WIC_DEVICE_INSPECTION_TIMEOUT: Duration = Duration::from_secs(10);
const WIC_SPAWN_ATTEMPTS: usize = 4;
const WIC_SPAWN_RETRY_DELAY: Duration = Duration::from_millis(5);
type WicOutputSnapshot = BTreeMap<PathBuf, (u64, u128)>;
type WicOutputScan = (WicOutputSnapshot, Vec<String>);

include!("wic/capability_and_kickstart.rs");
include!("wic/commands_and_device_types.rs");
include!("wic/device_discovery.rs");
include!("wic/job_runner.rs");
include!("wic/output_scan.rs");

#[cfg(test)]
mod tests {
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
    #[tokio::test]
    async fn wic_async_spawn_retries_only_transient_text_file_busy() {
        let directory = fixture("spawn-retry");
        let program = directory.join("wic");
        executable(&program, "exit 0");
        let writer = fs::OpenOptions::new().write(true).open(&program).unwrap();
        let release = tokio::spawn(async move {
            tokio::time::sleep(WIC_SPAWN_RETRY_DELAY + WIC_SPAWN_RETRY_DELAY).await;
            drop(writer);
        });
        let mut command = Command::new(&program);
        let mut child = spawn_async_command(&mut command).await.unwrap();
        assert!(child.wait().await.unwrap().success());
        release.await.unwrap();

        let writer = fs::OpenOptions::new().write(true).open(&program).unwrap();
        let mut command = Command::new(&program);
        let error = match spawn_async_command(&mut command).await {
            Ok(_) => panic!("write-held executable unexpectedly spawned"),
            Err(error) => error,
        };
        assert!(is_transient_spawn_error(&error));
        drop(writer);

        assert!(!is_transient_spawn_error(
            &std::io::Error::from_raw_os_error(libc::EACCES)
        ));
        fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn wic_adapter_capability_discovers_parses_and_constructs_exact_command() {
        let directory = fixture("exact");
        let program = directory.join("wic");
        executable(
            &program,
            "test \"$1 $2\" = 'list images' && printf 'directdisk  Direct disk\\ncustom Custom\\n'",
        );
        let canned = directory.join("canned");
        fs::create_dir(&canned).unwrap();
        let canned = fs::canonicalize(canned).unwrap();
        fs::write(
            canned.join("directdisk.wks"),
            "part / --source=rootfs --fstype=ext4 --size=64 --align=4\nbootloader --ptable gpt\n",
        )
        .unwrap();
        fs::write(
            canned.join("custom.wks.in"),
            "part /boot --source=bootimg --size=${BOOT_SIZE}\nunsupported value\n",
        )
        .unwrap();
        let capability = WicCapabilityInspector::with_executable(program)
            .with_sources(Vec::new(), vec![canned])
            .inspect(vec!["core-image-minimal".into()])
            .await;
        let WicCapability::Available { kickstarts, .. } = &capability else {
            panic!("available capability: {capability:?}");
        };
        assert_eq!(kickstarts.len(), 2);
        assert_eq!(
            kickstarts[1].partitions[0].mount_point.as_deref(),
            Some("/")
        );
        assert_eq!(kickstarts[1].partitions[0].size_mib, Some(64));
        assert!(
            kickstarts[0]
                .limitations
                .iter()
                .any(|limitation| limitation.contains("dynamic"))
        );

        let output = directory.join("output");
        fs::create_dir(&output).unwrap();
        let output = fs::canonicalize(output).unwrap();
        let draft = WicCreateDraft {
            machine: "qemux86-64".into(),
            image: "core-image-minimal".into(),
            kickstart: kickstarts[1].identity.clone(),
            output_directory: output.display().to_string(),
            generate_bmap: true,
            compression: WicCompression::Gzip,
        };
        let preview = draft.preview(&capability).unwrap();
        let command = WicCreateCommandSpec::from_preview(&preview, &capability).unwrap();
        assert_eq!(
            command.arguments(),
            &[
                OsString::from("create"),
                kickstarts[1]
                    .identity
                    .path
                    .as_ref()
                    .unwrap()
                    .as_os_str()
                    .to_owned(),
                "-e".into(),
                "core-image-minimal".into(),
                "-o".into(),
                output.as_os_str().to_owned(),
                "--bmap".into(),
                "--compress-with".into(),
                "gzip".into(),
            ]
        );
        let alternate = directory.join("alternate-wic");
        executable(&alternate, "exit 0");
        let alternate = fs::canonicalize(alternate).unwrap();
        let mut changed_capability = capability.clone();
        if let WicCapability::Available { executable, .. } = &mut changed_capability {
            *executable = alternate;
        }
        assert_eq!(
            WicCreateCommandSpec::from_preview(&preview, &changed_capability).unwrap_err(),
            WicAdapterError::PreviewMismatch
        );
        let mut tampered = preview;
        tampered.argv.push("--debug".into());
        assert_eq!(
            WicCreateCommandSpec::from_preview(&tampered, &capability).unwrap_err(),
            WicAdapterError::PreviewMismatch
        );
        fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn wic_adapter_capability_reports_missing_malformed_and_unsafe_sources() {
        assert_eq!(
            WicCapabilityInspector::with_executable("/missing/wic".into())
                .inspect(vec!["core-image-minimal".into()])
                .await,
            WicCapability::MissingTool
        );
        let directory = fixture("unsafe");
        let program = directory.join("wic");
        executable(&program, "printf 'bad/name malformed\\n'");
        assert!(matches!(
            WicCapabilityInspector::with_executable(program.clone())
                .inspect(vec!["core-image-minimal".into()])
                .await,
            WicCapability::Failed { .. }
        ));
        let target = directory.join("target.wks");
        fs::write(&target, "part /\n").unwrap();
        let link = directory.join("linked.wks");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        executable(&program, "exit 0");
        assert!(matches!(
            WicCapabilityInspector::with_executable(program)
                .with_sources(vec![link], Vec::new())
                .inspect(vec!["core-image-minimal".into()])
                .await,
            WicCapability::Failed { .. }
        ));
        fs::remove_dir_all(directory).unwrap();
    }

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
    #[tokio::test]
    async fn wic_adapter_runner_reports_only_new_outputs_and_nonzero_failure() {
        let (directory, output, command) = runner_fixture(
            "runner-success",
            "printf 'before\\n'; printf 'warning\\n' >&2; printf image > \"$6/new.wic\"; exit 0",
        )
        .await;
        fs::write(output.join("existing.wic"), "old").unwrap();
        let mut runner = WicJobRunner::new(directory.clone());
        runner.start(command, output.clone()).await.unwrap();
        assert_eq!(runner.next_event().await.unwrap(), WicRunnerEvent::Starting);
        assert_eq!(runner.next_event().await.unwrap(), WicRunnerEvent::Started);
        let terminal = tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let event = runner.next_event().await.unwrap();
                if matches!(event, WicRunnerEvent::Completed { .. }) {
                    break event;
                }
            }
        })
        .await
        .unwrap();
        let WicRunnerEvent::Completed { outputs, .. } = terminal else {
            unreachable!()
        };
        assert_eq!(outputs.len(), 1);
        assert!(outputs[0].identity.path.ends_with("new.wic"));
        fs::remove_dir_all(directory).unwrap();

        let (directory, output, command) =
            runner_fixture("runner-failure", "printf failed >&2; exit 9").await;
        let mut runner = WicJobRunner::new(directory.clone());
        runner.start(command, output).await.unwrap();
        let _ = runner.next_event().await.unwrap();
        let _ = runner.next_event().await.unwrap();
        let terminal = tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let event = runner.next_event().await.unwrap();
                if matches!(event, WicRunnerEvent::Failed { .. }) {
                    break event;
                }
            }
        })
        .await
        .unwrap();
        assert!(matches!(
            terminal,
            WicRunnerEvent::Failed {
                exit_code: Some(9),
                ..
            }
        ));
        fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn wic_adapter_runner_rejects_duplicate_and_forces_cancellation() {
        let (directory, output, command) = runner_fixture(
            "runner-cancel",
            "trap '' TERM; printf 'ready\\n'; while :; do :; done",
        )
        .await;
        let mut runner = WicJobRunner::new(directory.clone())
            .with_cancellation_timeout(Duration::from_millis(50));
        runner.start(command.clone(), output.clone()).await.unwrap();
        assert_eq!(
            runner.start(command, output).await.unwrap_err(),
            WicAdapterError::Busy
        );
        assert!(matches!(
            runner.next_event().await.unwrap(),
            WicRunnerEvent::Starting
        ));
        assert!(matches!(
            runner.next_event().await.unwrap(),
            WicRunnerEvent::Started
        ));
        loop {
            if matches!(
                runner.next_event().await.unwrap(),
                WicRunnerEvent::Output { ref line, .. } if line == "ready"
            ) {
                break;
            }
        }
        assert!(runner.cancel().await.unwrap());
        let cancelled = tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let event = runner.next_event().await.unwrap();
                if matches!(event, WicRunnerEvent::Cancelled { .. }) {
                    break event;
                }
            }
        })
        .await
        .unwrap();
        assert!(matches!(
            cancelled,
            WicRunnerEvent::Cancelled { forced: true, .. }
        ));
        assert!(!runner.cancel().await.unwrap());
        assert!(matches!(
            runner.next_event().await.unwrap(),
            WicRunnerEvent::CancellationRejected { .. }
        ));
        fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn wic_adapter_runner_times_out_without_blocking_forever() {
        let (directory, output, command) = runner_fixture("runner-timeout", "sleep 30").await;
        let mut runner =
            WicJobRunner::new(directory.clone()).with_execution_timeout(Duration::from_millis(20));
        runner.start(command, output).await.unwrap();
        let _ = runner.next_event().await.unwrap();
        let _ = runner.next_event().await.unwrap();
        assert!(matches!(
            runner.next_event().await.unwrap(),
            WicRunnerEvent::Failed {
                ref message,
                exit_code: None
            } if message.contains("timed out")
        ));
        fs::remove_dir_all(directory).unwrap();
    }

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
        assert!(
            response
                .limitations
                .iter()
                .any(|limitation| limitation.contains("/dev/sda")
                    && limitation.contains("root filesystem"))
        );
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

    #[cfg(unix)]
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
            parse_lsblk_devices(no_root.as_bytes(), &request.image, false, &BTreeSet::new())
                .is_err()
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

    #[cfg(unix)]
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

    #[cfg(unix)]
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
        let mut runner = WicJobRunner::new(directory.clone())
            .with_cancellation_timeout(Duration::from_millis(50));
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
}
