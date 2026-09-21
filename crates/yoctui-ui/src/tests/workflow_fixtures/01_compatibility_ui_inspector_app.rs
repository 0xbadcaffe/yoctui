use super::*;

pub(crate) fn compatibility_ui_inspector_app() -> App {
    let reason = |code: &str, message: &str, requirement: Option<&str>| {
        yoctui_model::CapabilityReason::new(code, message, requirement.map(str::to_owned)).unwrap()
    };
    let evidence = |outcome, subject: &str| yoctui_model::CapabilityEvidence {
        kind: yoctui_model::CapabilityEvidenceKind::DirectProbe,
        outcome,
        subject: subject.into(),
        detail: format!("Authoritative {subject} compatibility evidence."),
        argv: vec![subject.into(), "--help".into()],
    };
    let authority = yoctui_model::DaemonCompatibilitySnapshot {
        snapshot: yoctui_model::CapabilitySnapshot {
            generation: 7,
            environment: yoctui_model::YoctoEnvironmentIdentity {
                build_directory: yoctui_model::AuthoritativeValue::detected(
                    "/work/poky/build".into(),
                    yoctui_model::IdentityAuthority::InitializedEnvironment,
                ),
                source_roots: yoctui_model::AuthoritativeValue::detected(
                    vec![yoctui_model::SourceRootIdentity {
                        kind: yoctui_model::SourceRootKind::Poky,
                        path: "/work/poky".into(),
                    }],
                    yoctui_model::IdentityAuthority::ConfiguredLayerMetadata,
                ),
                bitbake_version: yoctui_model::AuthoritativeValue::detected(
                    "2.18.0".into(),
                    yoctui_model::IdentityAuthority::BitBakeVersionProbe,
                ),
                oe_core: yoctui_model::AuthoritativeValue::detected(
                    yoctui_model::ReleaseIdentity {
                        name: Some("OE-Core".into()),
                        version: Some("5.3".into()),
                    },
                    yoctui_model::IdentityAuthority::ReleaseMetadata,
                ),
                poky: yoctui_model::AuthoritativeValue::detected(
                    yoctui_model::ReleaseIdentity {
                        name: Some("wrynose".into()),
                        version: Some("6.0".into()),
                    },
                    yoctui_model::IdentityAuthority::ReleaseMetadata,
                ),
                distro: yoctui_model::AuthoritativeValue::detected(
                    yoctui_model::DistroIdentity {
                        name: "poky".into(),
                        version: Some("6.0".into()),
                    },
                    yoctui_model::IdentityAuthority::BitBakeDatastore,
                ),
                machine: yoctui_model::AuthoritativeValue::detected(
                    "qemux86-64".into(),
                    yoctui_model::IdentityAuthority::BitBakeDatastore,
                ),
                layer_series: yoctui_model::AuthoritativeValue::detected(
                    vec![yoctui_model::LayerSeriesIdentity {
                        layer: "core".into(),
                        root: "/work/poky/meta".into(),
                        compatible_series: vec!["wrynose".into()],
                    }],
                    yoctui_model::IdentityAuthority::ConfiguredLayerMetadata,
                ),
                backend: yoctui_model::AuthoritativeValue::detected(
                    yoctui_model::BackendIdentity {
                        name: "tinfoil".into(),
                        version: Some("2.18".into()),
                    },
                    yoctui_model::IdentityAuthority::BackendHandshake,
                ),
                protocol: yoctui_model::AuthoritativeValue::detected(
                    yoctui_model::ProtocolIdentity {
                        name: "yoctui-daemon".into(),
                        version: "1".into(),
                    },
                    yoctui_model::IdentityAuthority::ProtocolNegotiation,
                ),
                ..yoctui_model::YoctoEnvironmentIdentity::default()
            },
            capabilities: vec![
                yoctui_model::CapabilityRecord {
                    id: yoctui_model::CapabilityId::BitBakeBuild,
                    state: yoctui_model::CapabilityState::Available,
                    evidence: vec![evidence(
                        yoctui_model::CapabilityEvidenceOutcome::Positive,
                        "bitbake",
                    )],
                },
                yoctui_model::CapabilityRecord {
                    id: yoctui_model::CapabilityId::BitBakeGetVar,
                    state: yoctui_model::CapabilityState::AvailableWithLimitations {
                        reason: reason(
                            "compatibility.fallback",
                            "Native getvar is absent; environment dump fallback selected.",
                            Some("bitbake -e"),
                        ),
                        limitations: vec!["Complete environment dump is parsed.".into()],
                    },
                    evidence: vec![evidence(
                        yoctui_model::CapabilityEvidenceOutcome::Positive,
                        "bitbake -e",
                    )],
                },
                yoctui_model::CapabilityRecord {
                    id: yoctui_model::CapabilityId::DevtoolUpgrade,
                    state: yoctui_model::CapabilityState::Unavailable {
                        reason: reason(
                            "probe.subcommand_absent",
                            "Current Devtool does not expose the upgrade subcommand.",
                            Some("devtool upgrade"),
                        ),
                    },
                    evidence: vec![evidence(
                        yoctui_model::CapabilityEvidenceOutcome::Negative,
                        "devtool",
                    )],
                },
                yoctui_model::CapabilityRecord {
                    id: yoctui_model::CapabilityId::ResultTool,
                    state: yoctui_model::CapabilityState::Unknown {
                        reason: reason(
                            "probe.timed_out",
                            "The resulttool probe timed out.",
                            Some("resulttool --help"),
                        ),
                    },
                    evidence: vec![evidence(
                        yoctui_model::CapabilityEvidenceOutcome::Inconclusive,
                        "resulttool",
                    )],
                },
                yoctui_model::CapabilityRecord {
                    id: yoctui_model::CapabilityId::GitArchive,
                    state: yoctui_model::CapabilityState::Unsupported {
                        reason: reason(
                            "yoctui.not_implemented",
                            "Yoctui does not maintain this environment adapter.",
                            None,
                        ),
                    },
                    evidence: Vec::new(),
                },
            ],
        },
        implementations: std::collections::BTreeMap::from([
            (
                yoctui_model::CapabilityId::BitBakeBuild,
                yoctui_model::CapabilityImplementation {
                    id: "bitbake.build.command".into(),
                    kind: yoctui_model::CapabilityImplementationKind::Command,
                },
            ),
            (
                yoctui_model::CapabilityId::BitBakeGetVar,
                yoctui_model::CapabilityImplementation {
                    id: "bitbake.getvar.environment-fallback".into(),
                    kind: yoctui_model::CapabilityImplementationKind::Command,
                },
            ),
        ]),
    }
    .normalize()
    .unwrap();
    let mut app = App::new(32, 8192);
    app.screen = Screen::Compatibility;
    app.focus = FocusTarget::Workspace;
    app.daemon.status = yoctui_model::ClientReplicaStatus::Current;
    yoctui_model::install_workspace_compatibility(&mut app, authority).unwrap();
    app
}

pub(crate) fn security_report_identity(
    path: &str,
    fingerprint: &str,
) -> yoctui_model::SecurityReportIdentity {
    yoctui_model::SecurityReportIdentity::new(
        PathBuf::from(path),
        512,
        SystemTime::UNIX_EPOCH,
        fingerprint.into(),
    )
    .unwrap()
}

pub(crate) fn security_workflow_ui_app() -> App {
    let scope = SecurityScope::Recipe(RecipeIdentity {
        name: "busybox".into(),
        file: "/layers/meta/recipes-core/busybox/busybox_1.36.bb".into(),
    });
    let cve_identity =
        security_report_identity("/build/tmp/log/cve/busybox.cve.json", "cvefingerprint");
    let finding_identity = yoctui_model::CveFindingIdentity::new(
        "CVE-2026-0001".into(),
        "busybox".into(),
        Some("busybox".into()),
    )
    .unwrap();
    let cve = yoctui_model::CveReport {
        identity: cve_identity.clone(),
        scope: Some(scope.clone()),
        findings: vec![yoctui_model::CveFinding {
            identity: finding_identity.clone(),
            status: yoctui_model::CveStatus::Vulnerable,
            product: Some("busybox".into()),
            version: Some("1.36".into()),
            severity: Some("HIGH".into()),
            score: Some("8.1".into()),
            vector: Some("CVSS:3.1/AV:N".into()),
            advisory_url: Some("https://example.invalid/CVE-2026-0001".into()),
            summary: Some("A bounded vulnerability summary".into()),
            mapping: vec![
                yoctui_model::SecurityMetadata::new("upstream-product".into(), "busybox".into())
                    .unwrap(),
            ],
        }],
        metadata: vec![
            yoctui_model::SecurityMetadata::new("source".into(), "cve-check".into()).unwrap(),
        ],
        limitations: vec!["one unknown status was preserved".into()],
    };
    let spdx_identity =
        security_report_identity("/build/tmp/deploy/spdx/image.spdx.json", "spdxfingerprint");
    let spdx = yoctui_model::SpdxDocument {
        identity: spdx_identity.clone(),
        scope: Some(SecurityScope::Image {
            target: "core-image-minimal".into(),
            machine: "qemux86-64".into(),
            distro: "poky".into(),
        }),
        kind: SpdxArtifactKind::Json,
        spdx_version: Some("SPDX-2.3".into()),
        name: Some("core-image-minimal".into()),
        namespace: Some("https://example.invalid/spdx/image".into()),
        data_license: Some("CC0-1.0".into()),
        creators: vec!["Tool: bitbake".into()],
        components: vec![yoctui_model::SpdxComponent {
            identity: "SPDXRef-Package-busybox".into(),
            name: "busybox".into(),
            version: Some("1.36".into()),
            supplier: Some("Organization: Yocto".into()),
            license: Some("GPL-2.0-only".into()),
        }],
        file_count: Some(42),
        relationship_count: Some(7),
        checksums: vec![
            yoctui_model::SecurityMetadata::new("SHA256".into(), "abcd1234".into()).unwrap(),
        ],
        limitations: vec!["external references unavailable".into()],
    };
    let request = yoctui_model::SecurityReportRequest::new(
        1,
        vec![cve_identity.path.clone(), spdx_identity.path.clone()],
    )
    .unwrap();
    let capability = yoctui_model::SecurityCapabilitySnapshot::new(
        Some("6.0".into()),
        "/build".into(),
        scope.clone(),
        vec![scope.clone()],
        Some("cve_check".into()),
        Some("create_recipe_sbom".into()),
        None,
        false,
        Some(yoctui_model::SecurityMapperCapability {
            executable: "/workspace/scripts/cve-check-map-pkgs".into(),
            arguments: vec!["/build/tmp/log/cve".into()],
        }),
        vec!["/build/tmp/log/cve".into()],
        vec!["/build/tmp/deploy/spdx".into()],
        vec!["image SBOM task unavailable for recipe scope".into()],
    )
    .unwrap();
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Security;
    app.focus = FocusTarget::Workspace;
    app.security.scope = Some(scope);
    app.security.capability = SecurityCapability::Available(Box::new(capability));
    app.security.inventory = SecurityInventoryState::Partial {
        request,
        reports: vec![SecurityReport::Cve(cve), SecurityReport::Spdx(spdx)],
        limitations: vec!["one malformed report was skipped".into()],
    };
    app.security.report_selection = Some(cve_identity);
    app.security.finding_selection = Some(finding_identity);
    app
}

pub(crate) fn security_session(status: SecuritySessionStatus) -> yoctui_model::SecuritySession {
    let preview = yoctui_model::SecurityOperationPreview {
        id: yoctui_model::SecuritySessionId(9),
        scope: SecurityScope::Image {
            target: "core-image-minimal".into(),
            machine: "qemux86-64".into(),
            distro: "poky".into(),
        },
        operation: SecurityOperation::PackageMap {
            executable: "/workspace/scripts/cve-check-map-pkgs".into(),
            arguments: vec!["/build/tmp/log/cve".into()],
        },
        indexed_arguments: vec![
            "0: /workspace/scripts/cve-check-map-pkgs".into(),
            "1: /build/tmp/log/cve".into(),
        ],
        report_roots: vec!["/build/tmp/log/cve".into()],
    };
    yoctui_model::SecuritySession {
        preview,
        status,
        background_job_id: None,
        started_at: SystemTime::UNIX_EPOCH,
        finished_at: status.is_terminal().then_some(SystemTime::UNIX_EPOCH),
        message: matches!(
            status,
            SecuritySessionStatus::Failed
                | SecuritySessionStatus::Lost
                | SecuritySessionStatus::TimedOut
        )
        .then(|| format!("{} detail", security_session_status_label(status))),
        result_paths: (status == SecuritySessionStatus::Succeeded)
            .then(|| PathBuf::from("/build/tmp/log/cve/busybox.cve.json"))
            .into_iter()
            .collect(),
        output: vec![
            yoctui_model::SecurityOutputLine {
                stream: SecurityOutputStream::Stdout,
                line: "mapped busybox -> busybox".into(),
                truncated: false,
            },
            yoctui_model::SecurityOutputLine {
                stream: SecurityOutputStream::Stderr,
                line: "bounded mapper warning".into(),
                truncated: true,
            },
        ],
    }
}

pub(crate) fn ux_rootfs_ui_app() -> App {
    let mut app = App::new(20, 20_000);
    app.screen = Screen::Images;
    app.focus = FocusTarget::Workspace;
    let image = yoctui_model::ImageArtifactIdentity {
        machine: "qemux86-64".into(),
        image: "core-image-minimal".into(),
        path: "/build/tmp/deploy/images/qemux86-64/core-image-minimal.ext4".into(),
    };
    let request = yoctui_model::RootfsCompositionRequest {
        generation: 7,
        image: image.clone(),
    };
    let packages = (0_u64..10)
        .map(|index| yoctui_model::RootfsInstalledPackage {
            identity: PackageIdentity::new(format!("pkg{index}")),
            recipe: Some(format!("recipe{index}")),
            category: format!("category{index}"),
            installed_size_bytes: (index + 1) * 1_024,
            file_count: index + 1,
        })
        .collect();
    let entries = vec![
        yoctui_model::RootfsEntry {
            identity: yoctui_model::RootfsPathIdentity("/".into()),
            kind: RootfsEntryKind::Directory,
            size_bytes: 0,
            package: None,
        },
        yoctui_model::RootfsEntry {
            identity: yoctui_model::RootfsPathIdentity("/usr".into()),
            kind: RootfsEntryKind::Directory,
            size_bytes: 0,
            package: None,
        },
        yoctui_model::RootfsEntry {
            identity: yoctui_model::RootfsPathIdentity("/usr/bin".into()),
            kind: RootfsEntryKind::Directory,
            size_bytes: 0,
            package: None,
        },
        yoctui_model::RootfsEntry {
            identity: yoctui_model::RootfsPathIdentity("/usr/bin/tool".into()),
            kind: RootfsEntryKind::RegularFile,
            size_bytes: 4_096,
            package: Some(PackageIdentity::new("pkg9")),
        },
        yoctui_model::RootfsEntry {
            identity: yoctui_model::RootfsPathIdentity("/bin/sh".into()),
            kind: RootfsEntryKind::Symlink,
            size_bytes: 4,
            package: Some(PackageIdentity::new("pkg0")),
        },
        yoctui_model::RootfsEntry {
            identity: yoctui_model::RootfsPathIdentity("/dev/console".into()),
            kind: RootfsEntryKind::Other,
            size_bytes: 0,
            package: None,
        },
    ];
    app.rootfs_composition = RootfsCompositionState::Partial {
        request,
        composition: yoctui_model::RootfsComposition {
            image,
            installed_packages: yoctui_model::RootfsAuthority::Available(
                yoctui_model::RootfsPackageInventory { packages },
            ),
            filesystem_tree: yoctui_model::RootfsAuthority::Partial {
                value: yoctui_model::RootfsFilesystemTree { entries },
                limitations: vec!["package ownership is partial".into()],
            },
            system_inventory: yoctui_model::RootfsAuthority::Available(
                yoctui_model::RootfsSystemInventory::default(),
            ),
            root_directory: None,
        },
        limitations: vec!["package ownership is partial".into()],
    };
    app.rootfs_group_selection = Some(RootfsGroupIdentity::Other);
    app.rootfs_package_selection = Some(PackageIdentity::new("pkg2"));
    app.rootfs_entry_selection = Some(yoctui_model::RootfsPathIdentity("/usr/bin/tool".into()));
    app
}
