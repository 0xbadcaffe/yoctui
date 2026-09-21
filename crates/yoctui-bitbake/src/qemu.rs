include!("qemu/capability_and_command.rs");

include!("qemu/job_runner.rs");

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use yoctui_model::{ImageArtifactField, QemuLaunchDraft};

    static FIXTURE_ID: AtomicU64 = AtomicU64::new(1);

    fn fixture_dir(name: &str) -> PathBuf {
        let id = FIXTURE_ID.fetch_add(1, Ordering::Relaxed);
        let directory = std::env::temp_dir().join(format!(
            "yoctui-qemu-adapter-{}-{name}-{id}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        fs::canonicalize(directory).unwrap()
    }

    #[cfg(unix)]
    fn executable(path: &Path, body: &str) {
        crate::test_support::write_executable(path, &format!("#!/bin/sh\n{body}\n"));
    }

    fn artifact(path: PathBuf, kind: ImageArtifactKind) -> ImageArtifact {
        ImageArtifact {
            identity: ImageArtifactIdentity {
                machine: "qemux86-64".into(),
                image: "core-image-minimal".into(),
                path,
            },
            kind,
            size_bytes: ImageArtifactField::Unavailable,
            modified_unix_seconds: ImageArtifactField::Unavailable,
            checksums: ImageArtifactField::Unavailable,
            manifests: ImageArtifactField::Unavailable,
            licenses: ImageArtifactField::Unavailable,
            spdx: ImageArtifactField::Unavailable,
            wic_files: ImageArtifactField::Unavailable,
        }
    }

    #[cfg(unix)]
    fn fixture_preview(name: &str, body: &str) -> (PathBuf, QemuLaunchPreview, QemuCommandSpec) {
        let directory = fixture_dir(name);
        let program = directory.join("runqemu");
        executable(&program, body);
        let deploy = directory.join("qemux86-64");
        fs::create_dir(&deploy).unwrap();
        let image_path = deploy.join("core-image-minimal.wic");
        fs::write(&image_path, b"wic").unwrap();
        let image = artifact(image_path, ImageArtifactKind::Wic);
        let capability =
            QemuCapabilityInspector::with_executable(program).inspect(std::slice::from_ref(&image));
        let draft = QemuLaunchDraft::for_artifact(image.identity, image.kind);
        let preview = draft.preview(&capability).unwrap();
        let command = QemuCommandSpec::from_preview(&preview).unwrap();
        (directory, preview, command)
    }

    #[cfg(unix)]
    #[test]
    fn qemu_adapter_capability_distinguishes_available_missing_image_and_failure() {
        let directory = fixture_dir("capability");
        let program = directory.join("runqemu");
        executable(&program, "exit 0");
        let deploy = directory.join("qemux86-64");
        fs::create_dir(&deploy).unwrap();
        let image_path = deploy.join("core-image-minimal.wic");
        fs::write(&image_path, b"wic").unwrap();
        let image = artifact(image_path, ImageArtifactKind::Wic);
        assert!(matches!(
            QemuCapabilityInspector::with_executable(program.clone())
                .inspect(std::slice::from_ref(&image)),
            QemuCapability::Available {
                compatible_images,
                ..
            } if compatible_images == vec![image.identity.clone()]
        ));
        assert_eq!(
            QemuCapabilityInspector::with_executable(directory.join("missing")).inspect(&[]),
            QemuCapability::MissingTool
        );
        assert_eq!(
            QemuCapabilityInspector::with_executable("definitely-missing-runqemu".into())
                .inspect(&[]),
            QemuCapability::MissingTool
        );
        assert_eq!(
            QemuCapabilityInspector::with_executable(program.clone()).inspect(&[]),
            QemuCapability::MissingCompatibleImage
        );
        let stale = artifact(deploy.join("missing.wic"), ImageArtifactKind::Wic);
        assert!(matches!(
            QemuCapabilityInspector::with_executable(program).inspect(&[stale]),
            QemuCapability::Failed { message } if message.contains("does not exist")
        ));
        fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn qemu_adapter_builds_exact_shell_free_arguments_and_rejects_tampering() {
        let (directory, mut preview, command) = fixture_preview("command", "printf '%s\\n' \"$@\"");
        assert_eq!(command.executable(), directory.join("runqemu"));
        assert_eq!(
            command.arguments(),
            [
                OsString::from("qemux86-64"),
                directory
                    .join("qemux86-64/core-image-minimal.wic")
                    .as_os_str()
                    .to_owned(),
                OsString::from("qemumemory=1024"),
                OsString::from("slirp"),
                OsString::from("sdl"),
                OsString::from("serialstdio"),
            ]
        );
        preview.argv.push("--help".into());
        assert_eq!(
            QemuCommandSpec::from_preview(&preview),
            Err(QemuAdapterError::PreviewMismatch)
        );
        preview.argv.pop();
        preview.request.extra_arguments = vec!["--help".into()];
        assert!(matches!(
            QemuCommandSpec::from_preview(&preview),
            Err(QemuAdapterError::InvalidRequest(_))
        ));
        fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn qemu_adapter_rejects_symlinked_artifacts() {
        use std::os::unix::fs::symlink;
        let directory = fixture_dir("symlink");
        let program = directory.join("runqemu");
        executable(&program, "exit 0");
        let deploy = directory.join("qemux86-64");
        fs::create_dir(&deploy).unwrap();
        let target = deploy.join("target.wic");
        let link = deploy.join("core-image-minimal.wic");
        fs::write(&target, b"wic").unwrap();
        symlink(&target, &link).unwrap();
        let image = artifact(link, ImageArtifactKind::Wic);
        assert!(matches!(
            QemuCapabilityInspector::with_executable(program).inspect(&[image]),
            QemuCapability::Failed { message } if message.contains("non-symlink")
        ));
        fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn qemu_adapter_streams_bounded_output_and_reports_nonzero_exit() {
        let (directory, _, command) = fixture_preview(
            "output",
            "printf 'stdout\\n'; printf 'stderr\\n' >&2; printf '\\377bad\\n'; head -c 70000 /dev/zero | tr '\\000' x; printf '\\n'; exit 7",
        );
        let mut runner = QemuJobRunner::new(directory.clone());
        runner.start(command.clone()).await.unwrap();
        assert_eq!(runner.start(command).await, Err(QemuAdapterError::Busy));
        assert_eq!(
            runner.next_event().await.unwrap(),
            QemuRunnerEvent::Starting
        );
        assert_eq!(runner.next_event().await.unwrap(), QemuRunnerEvent::Started);
        let mut output = Vec::new();
        loop {
            match runner.next_event().await.unwrap() {
                QemuRunnerEvent::Output {
                    stream,
                    line,
                    truncated,
                } => output.push((stream, line, truncated)),
                QemuRunnerEvent::Failed { exit_code, .. } => {
                    assert_eq!(exit_code, Some(7));
                    break;
                }
                event => panic!("unexpected event: {event:?}"),
            }
        }
        assert!(output.iter().any(|(stream, line, _)| {
            *stream == QemuRunnerOutputStream::Stdout && line == "stdout"
        }));
        assert!(output.iter().any(|(stream, line, _)| {
            *stream == QemuRunnerOutputStream::Stderr && line == "stderr"
        }));
        assert!(output.iter().any(|(_, line, _)| line.contains('\u{fffd}')));
        assert!(
            output
                .iter()
                .any(|(_, line, truncated)| { *truncated && line.len() <= MAX_QEMU_LINE_BYTES })
        );
        fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn qemu_adapter_reports_successful_completion() {
        let (directory, _, command) = fixture_preview("success", "exit 0");
        let mut runner = QemuJobRunner::new(directory.clone());
        runner.start(command).await.unwrap();
        assert_eq!(
            runner.next_event().await.unwrap(),
            QemuRunnerEvent::Starting
        );
        assert_eq!(runner.next_event().await.unwrap(), QemuRunnerEvent::Started);
        assert_eq!(
            runner.next_event().await.unwrap(),
            QemuRunnerEvent::Completed { exit_code: 0 }
        );
        fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn qemu_adapter_cancels_gracefully_and_escalates_process_groups() {
        for (name, trap, expected_forced) in [
            ("cancel-graceful", "trap 'exit 0' TERM", false),
            ("cancel-forced", "trap '' TERM", true),
        ] {
            let (directory, _, command) = fixture_preview(
                name,
                &format!("{trap}; printf 'ready\\n'; while :; do :; done"),
            );
            let mut runner = QemuJobRunner::new(directory.clone())
                .with_cancellation_timeout(Duration::from_millis(250));
            runner.start(command).await.unwrap();
            let _ = runner.next_event().await.unwrap();
            let _ = runner.next_event().await.unwrap();
            loop {
                if matches!(
                    runner.next_event().await.unwrap(),
                    QemuRunnerEvent::Output { ref line, .. } if line == "ready"
                ) {
                    break;
                }
            }
            assert!(runner.cancel().await.unwrap());
            assert!(!runner.cancel().await.unwrap());
            loop {
                if let QemuRunnerEvent::Cancelled { forced, .. } =
                    runner.next_event().await.unwrap()
                {
                    assert_eq!(forced, expected_forced);
                    break;
                }
            }
            assert!(matches!(
                runner.next_event().await.unwrap(),
                QemuRunnerEvent::CancellationRejected { message }
                    if message.contains("no cancellable")
            ));
            fs::remove_dir_all(directory).unwrap();
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn qemu_adapter_reports_unexpected_output_channel_loss() {
        let (directory, _, command) =
            fixture_preview("channel-loss", "printf 'ready\\n'; sleep 30");
        let mut runner = QemuJobRunner::new(directory.clone());
        runner.start(command).await.unwrap();
        let _ = runner.next_event().await.unwrap();
        let _ = runner.next_event().await.unwrap();
        runner.output = None;
        assert!(matches!(
            runner.next_event().await.unwrap(),
            QemuRunnerEvent::Lost { message } if message.contains("channel")
        ));
        assert!(!runner.is_active());
        fs::remove_dir_all(directory).unwrap();
    }
}
