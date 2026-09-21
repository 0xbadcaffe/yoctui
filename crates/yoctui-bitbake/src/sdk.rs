include!("sdk/types_and_adapter.rs");

include!("sdk/artifact_scan.rs");

include!("sdk/association_and_limits.rs");

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs::File,
        io::Write,
        sync::atomic::{AtomicU64, Ordering},
    };

    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "yoctui-sdk-artifact-{}-{}",
                std::process::id(),
                NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn fixture() -> TestDirectory {
        TestDirectory::new()
    }

    fn request(root: PathBuf) -> SdkArtifactInventoryRequest {
        SdkArtifactInventoryRequest {
            generation: 1,
            root,
            machine: "qemux86-64".into(),
        }
    }

    #[tokio::test]
    async fn sdk_artifact_scan_sorts_classifies_and_associates_records() {
        let fixture = fixture();
        let root = fixture.path().join("sdk");
        fs::create_dir(&root).unwrap();
        fs::write(root.join("poky-toolchain.sh.target.manifest"), b"target").unwrap();
        fs::write(root.join("poky-toolchain.sh.sha256"), b"digest").unwrap();
        fs::write(root.join("poky-toolchain.host.manifest"), b"host").unwrap();
        fs::write(root.join("poky-toolchain.sh"), b"installer").unwrap();
        fs::write(root.join("README.txt"), b"other").unwrap();

        let response = SdkArtifactAdapter::new(root.clone())
            .scan(request(root))
            .await
            .unwrap();
        let SdkArtifactScanOutcome::Complete(artifacts) = response.outcome else {
            panic!("expected complete SDK inventory");
        };
        assert!(
            artifacts
                .windows(2)
                .all(|pair| pair[0].identity < pair[1].identity)
        );
        let installer = artifacts
            .iter()
            .find(|artifact| artifact.kind == SdkArtifactKind::Installer)
            .unwrap();
        assert_eq!(installer.checksums.len(), 1);
        assert_eq!(installer.manifests.len(), 2);
        assert!(installer.identity.size_bytes > 0);
    }

    #[tokio::test]
    async fn sdk_artifact_scan_distinguishes_empty_and_unavailable_metadata() {
        let fixture = fixture();
        let root = fixture.path().join("sdk");
        fs::create_dir(&root).unwrap();
        let empty = SdkArtifactAdapter::new(root.clone())
            .scan(request(root.clone()))
            .await
            .unwrap();
        assert_eq!(empty.outcome, SdkArtifactScanOutcome::Empty);

        fs::write(root.join("poky-toolchain.sh"), b"installer").unwrap();
        let response = SdkArtifactAdapter::new(root.clone())
            .scan(request(root))
            .await
            .unwrap();
        let artifact = response.outcome.artifacts().first().unwrap();
        assert_eq!(artifact.sdk_kind, None);
        assert_eq!(artifact.machine, None);
        assert_eq!(artifact.host_tuple, None);
        assert_eq!(artifact.target_tuple, None);
        assert_eq!(artifact.published, None);
    }

    #[tokio::test]
    async fn sdk_artifact_scan_reports_partial_malformed_and_oversized_records() {
        let fixture = fixture();
        let root = fixture.path().join("sdk");
        fs::create_dir(&root).unwrap();
        fs::write(root.join("valid.sh"), b"installer").unwrap();
        fs::write(root.join(".manifest"), b"malformed").unwrap();
        let long_name = format!("{}.txt", "x".repeat(MAX_SDK_NAME_BYTES));
        File::create(root.join(long_name))
            .unwrap()
            .write_all(b"x")
            .unwrap();

        let response = SdkArtifactAdapter::new(root.clone())
            .scan(request(root))
            .await
            .unwrap();
        assert!(matches!(
            response.outcome,
            SdkArtifactScanOutcome::Partial { .. }
        ));
        assert!(
            response
                .outcome
                .limitations()
                .iter()
                .any(|message| { message.contains("malformed") || message.contains("oversized") })
        );
        assert_eq!(response.outcome.artifacts().len(), 1);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn sdk_artifact_scan_rejects_root_symlink_mismatch_and_entry_escape() {
        use std::os::unix::fs::symlink;

        let fixture = fixture();
        let root = fixture.path().join("sdk");
        fs::create_dir(&root).unwrap();
        let missing = fixture.path().join("missing");
        assert!(matches!(
            SdkArtifactAdapter::new(missing.clone())
                .scan(request(missing))
                .await,
            Err(SdkArtifactAdapterError::MissingRoot(_))
        ));

        let linked = fixture.path().join("linked");
        symlink(&root, &linked).unwrap();
        assert!(matches!(
            SdkArtifactAdapter::new(linked.clone())
                .scan(request(linked))
                .await,
            Err(SdkArtifactAdapterError::SymlinkRoot(_))
        ));

        let other = fixture.path().join("other");
        fs::create_dir(&other).unwrap();
        assert!(matches!(
            SdkArtifactAdapter::new(root.clone())
                .scan(request(other))
                .await,
            Err(SdkArtifactAdapterError::RootMismatch { .. })
        ));

        let outside = fixture.path().join("outside.sh");
        fs::write(&outside, b"outside").unwrap();
        symlink(&outside, root.join("escaped.sh")).unwrap();
        let response = SdkArtifactAdapter::new(root.clone())
            .scan(request(root))
            .await
            .unwrap();
        assert!(matches!(
            response.outcome,
            SdkArtifactScanOutcome::Partial { .. }
        ));
        assert!(
            response
                .outcome
                .limitations()
                .iter()
                .any(|message| message.contains("symlink"))
        );
    }

    #[tokio::test]
    async fn sdk_artifact_scan_has_distinct_timeout_cancellation_permission_and_worker_loss() {
        let fixture = fixture();
        let root = fixture.path().join("sdk");
        fs::create_dir(&root).unwrap();
        let cancellation = SdkArtifactCancellation::default();
        cancellation.cancel();
        assert_eq!(
            SdkArtifactAdapter::new(root.clone())
                .scan_with_cancellation(request(root.clone()), cancellation)
                .await,
            Err(SdkArtifactAdapterError::Cancelled)
        );
        assert!(matches!(
            SdkArtifactAdapter::new(root.clone())
                .with_timeout(Duration::ZERO)
                .scan(request(root.clone()))
                .await,
            Err(SdkArtifactAdapterError::Timeout(_))
        ));
        assert!(matches!(
            SdkArtifactAdapter::new(root.clone())
                .with_worker_panic()
                .scan(request(root.clone()))
                .await,
            Err(SdkArtifactAdapterError::WorkerLost(_))
        ));
        assert_eq!(
            root_io_error(
                &root,
                io::Error::new(io::ErrorKind::PermissionDenied, "denied")
            ),
            SdkArtifactAdapterError::PermissionDenied(root)
        );
    }

    #[tokio::test]
    async fn sdk_artifact_scan_bounds_directory_entries_deterministically() {
        let fixture = fixture();
        let root = fixture.path().join("sdk");
        fs::create_dir(&root).unwrap();
        for index in (0..=MAX_DIRECTORY_ENTRIES).rev() {
            fs::write(root.join(format!("{index:05}.txt")), b"record").unwrap();
        }

        let first = SdkArtifactAdapter::new(root.clone())
            .scan(request(root.clone()))
            .await
            .unwrap();
        let second = SdkArtifactAdapter::new(root.clone())
            .scan(request(root))
            .await
            .unwrap();
        assert!(matches!(
            first.outcome,
            SdkArtifactScanOutcome::Partial { .. }
        ));
        assert_eq!(first.outcome.artifacts(), second.outcome.artifacts());
        assert_eq!(first.outcome.limitations(), second.outcome.limitations());
        assert_eq!(first.outcome.artifacts().len(), MAX_DIRECTORY_ENTRIES);
        assert!(
            first
                .outcome
                .artifacts()
                .iter()
                .all(|artifact| !artifact.identity.path.ends_with("04096.txt"))
        );
    }

    #[tokio::test]
    async fn sdk_artifact_scan_bounds_traversed_directories() {
        let fixture = fixture();
        let root = fixture.path().join("sdk");
        fs::create_dir(&root).unwrap();
        let mut directory = root.clone();
        for index in 0..=MAX_SDK_DIRECTORIES {
            directory = directory.join(format!("{index:03}"));
            fs::create_dir(&directory).unwrap();
            fs::write(directory.join("record.txt"), b"record").unwrap();
        }

        let response = SdkArtifactAdapter::new(root.clone())
            .scan(request(root))
            .await
            .unwrap();
        assert!(matches!(
            response.outcome,
            SdkArtifactScanOutcome::Partial { .. }
        ));
        assert!(
            response
                .outcome
                .limitations()
                .iter()
                .any(|message| message.contains("directories were omitted"))
        );
        assert_eq!(response.outcome.artifacts().len(), MAX_SDK_DIRECTORIES - 1);
    }
}
