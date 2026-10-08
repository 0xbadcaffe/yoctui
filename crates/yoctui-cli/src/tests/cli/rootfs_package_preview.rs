use super::*;

#[tokio::test]
async fn rootfs_package_preview_installs_before_worker_finishes_and_rejects_cancelled_results() {
    use yoctui_model::*;
    for cancelled in [false, true] {
        let mut app = App::new(16, 4096);
        let request = RootfsCompositionRequest {
            generation: 1,
            image: ImageArtifactIdentity {
                machine: "machine".into(),
                image: "image".into(),
                path: "/build/image".into(),
            },
        };
        app.rootfs_composition = RootfsCompositionState::Loading {
            request: request.clone(),
        };
        let cancellation = RootfsCompositionCancellation::default();
        if cancelled {
            cancellation.cancel();
        }
        let (sender, receiver) = tokio::sync::oneshot::channel();
        sender
            .send(yoctui_bitbake::RootfsCompositionResponse {
                request: request.clone(),
                composition: RootfsComposition {
                    image: request.image.clone(),
                    installed_packages: RootfsAuthority::Available(RootfsPackageInventory {
                        packages: vec![RootfsInstalledPackage {
                            identity: PackageIdentity::new("busybox"),
                            recipe: Some("busybox".into()),
                            category: "base".into(),
                            installed_size_bytes: 1024,
                            file_count: 1,
                        }],
                    }),
                    filesystem_tree: RootfsAuthority::Unavailable {
                        reason: "Still loading".into(),
                    },
                    system_inventory: RootfsAuthority::Unavailable {
                        reason: "Still loading".into(),
                    },
                    root_directory: None,
                },
                limitations: vec!["Still loading".into()],
            })
            .unwrap();
        let mut operation = Some(RootfsCompositionBackgroundOperation {
            request,
            #[cfg(unix)]
            authority: None,
            _cancellation: cancellation,
            package_preview: Some(receiver),
            handle: tokio::spawn(std::future::pending()),
        });
        poll_rootfs_composition_operation(&mut app, &mut operation).await;
        assert!(!operation.as_ref().unwrap().handle.is_finished());
        if cancelled {
            assert!(matches!(
                app.rootfs_composition,
                RootfsCompositionState::Loading { .. }
            ));
        } else {
            assert!(matches!(
                app.rootfs_composition,
                RootfsCompositionState::LoadingDetails { .. }
            ));
            assert_eq!(
                app.rootfs_composition
                    .composition()
                    .unwrap()
                    .totals()
                    .0
                    .installed_package_bytes,
                1024
            );
        }
        operation.as_ref().unwrap().handle.abort();
    }
}

#[tokio::test]
#[ignore = "read-only live build benchmark; requires explicit YOCTUI_ROOTFS_BENCH_* paths"]
async fn rootfs_live_package_loading_benchmark() {
    let path = |suffix: &str| {
        PathBuf::from(std::env::var(format!("YOCTUI_ROOTFS_BENCH_{suffix}")).unwrap())
    };
    let build = path("BUILD");
    let image = yoctui_model::ImageArtifactIdentity {
        machine: std::env::var("YOCTUI_ROOTFS_BENCH_MACHINE").unwrap(),
        image: std::env::var("YOCTUI_ROOTFS_BENCH_IMAGE").unwrap(),
        path: path("ARTIFACT"),
    };
    let request = RootfsCompositionRequest {
        generation: 1,
        image: image.clone(),
    };
    let mut sources = RootfsCompositionSources {
        image,
        manifest: Some(path("MANIFEST")),
        pkgdata_directory: Some(path("PKGDATA")),
        image_rootfs: None,
    };
    for stage in ["packages", "full"] {
        if stage == "full" {
            sources.image_rootfs = Some(path("ROOTFS"));
        }
        for run in 0..3 {
            let started = std::time::Instant::now();
            let response = RootfsCompositionAdapter::new(build.clone(), sources.clone(), 1)
                .scan(request.clone())
                .await
                .unwrap();
            let inventory = response.composition.package_inventory().unwrap();
            assert!(!inventory.packages.is_empty());
            println!(
                "stage={stage} run={run} elapsed_ms={:.3} packages={} bytes={} filesystem_entries={}",
                started.elapsed().as_secs_f64() * 1000.0,
                inventory.packages.len(),
                response.composition.totals().0.installed_package_bytes,
                response
                    .composition
                    .filesystem_tree()
                    .map_or(0, |tree| tree.entries.len())
            );
        }
    }
}
