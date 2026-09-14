//! Image inspector.
use super::*;

pub(crate) fn image_artifact_inspector_text(app: &App) -> String {
    if app.images_view != ImagesView::Artifacts {
        return rootfs_inspector_text(app);
    }
    let paths = |field: &ImageArtifactField<Vec<std::path::PathBuf>>| {
        field.available().map_or_else(
            || "unavailable".into(),
            |paths| {
                if paths.is_empty() {
                    "none".into()
                } else {
                    paths
                        .iter()
                        .map(|path| path.display().to_string())
                        .collect::<Vec<_>>()
                        .join("\n")
                }
            },
        )
    };
    let artifact_text = app.selected_image_artifact().map_or_else(
        || {
            "No deployed image artifact selected.\nUse i to select a buildable image recipe; press R to scan DEPLOY_DIR_IMAGE.".into()
        },
        |artifact| {
            let checksums = artifact.checksums.available().map_or_else(
                || "unavailable".into(),
                |checksums| {
                    if checksums.is_empty() {
                        "none".into()
                    } else {
                        checksums
                            .iter()
                            .map(|checksum| {
                                format!(
                                    "{} {} ({})",
                                    checksum.algorithm,
                                    checksum.digest,
                                    checksum.source.display()
                                )
                            })
                            .collect::<Vec<_>>()
                            .join("\n")
                    }
                },
            );
            let deploy = app
                .image_artifacts
                .inventory()
                .and_then(|inventory| inventory.deploy_directory.available())
                .map_or_else(|| "unavailable".into(), |path| path.display().to_string());
            let limitations = match &app.image_artifacts {
                ImageArtifactInventoryState::Partial { limitations, .. } => limitations
                    .iter()
                    .map(|limitation| format!("! {limitation}"))
                    .collect::<Vec<_>>()
                    .join("\n"),
                _ => "none".into(),
            };
            let preview = artifact.kind.image_preview_decision(
                yoctui_model::ImagePreviewTransport::DirectTerminal,
            );
            format!(
                "Machine: {}\nImage: {}\nKind: {}\nPath: {}\n\nTerminal image preview\nNative graphics: not offered\nprotocol probe skipped\nFallback: {}\nReason: {}\n\nDeploy directory: {}\nSize: {}\nTimestamp: {}\nLimitations:\n{}\n\nChecksums:\n{}\n\nManifests:\n{}\n\nLicenses:\n{}\n\nSPDX/SBOM:\n{}\n\nWic files:\n{}",
                artifact.identity.machine,
                artifact.identity.image,
                artifact.kind.label(),
                artifact.identity.path.display(),
                preview.fallback.label(),
                preview.reason,
                deploy,
                artifact
                    .size_bytes
                    .available()
                    .map_or_else(|| "unavailable".into(), |value| format!("{value} bytes")),
                artifact.modified_unix_seconds.available().map_or_else(
                    || "unavailable".into(),
                    |value| format!("{value}s since Unix epoch")
                ),
                limitations,
                checksums,
                paths(&artifact.manifests),
                paths(&artifact.licenses),
                paths(&artifact.spdx),
                paths(&artifact.wic_files),
            )
        },
    );
    let operational_details = format!(
        "runqemu capability\n{}\nLaunch: {}\n\n{}\n{}",
        qemu_capability_text(app),
        app.qemu_launch_unavailable_reason()
            .unwrap_or_else(|| "ready for selected artifact (Q)".into()),
        qemu_session_text(app),
        wic_inspector_text(app),
    );
    let operation_has_priority = !matches!(app.qemu_capability, QemuCapability::NotInspected)
        || app.latest_qemu_session().is_some()
        || !matches!(app.wic_capability, WicCapability::NotInspected)
        || app.latest_wic_session().is_some();
    if operation_has_priority {
        format!("{operational_details}\nSelected artifact\n{artifact_text}")
    } else {
        format!("Selected artifact\n{artifact_text}\n\n{operational_details}")
    }
}

pub(crate) fn rootfs_inspector_text(app: &App) -> String {
    let state = match &app.rootfs_composition {
        RootfsCompositionState::NotLoaded => "not loaded".into(),
        RootfsCompositionState::Loading { request } => {
            format!("loading generation {}", request.generation)
        }
        RootfsCompositionState::AvailableEmpty { .. } => "available empty".into(),
        RootfsCompositionState::Available { .. } => "available".into(),
        RootfsCompositionState::Partial { limitations, .. } => {
            format!("partial · {} limitation(s)", limitations.len())
        }
        RootfsCompositionState::Unavailable { reason, .. } => format!("unavailable · {reason}"),
        RootfsCompositionState::Failed { message, .. } => format!("failed · {message}"),
    };
    let Some(composition) = app.rootfs_composition.composition() else {
        return format!(
            "Rootfs composition\nView: {}\nState: {state}\n\nPackage and filesystem authority are reported separately.",
            app.images_view.label()
        );
    };
    let (totals, overflowed) = composition.totals();
    let authority = format!(
        "Installed packages: {}\nFilesystem tree: {}\nOffline system map: {}",
        rootfs_authority_label(&composition.installed_packages),
        rootfs_authority_label(&composition.filesystem_tree),
        rootfs_authority_label(&composition.system_inventory)
    );
    let selected = match app.images_view {
        ImagesView::Artifacts => String::new(),
        ImagesView::RootfsPackages => {
            let inventory = composition.package_inventory();
            let package = app.rootfs_package_selection.as_ref().and_then(|identity| {
                inventory.and_then(|inventory| {
                    inventory
                        .packages
                        .iter()
                        .find(|package| &package.identity == identity)
                })
            });
            let group = app
                .rootfs_group_selection
                .as_ref()
                .map_or_else(|| "none".into(), rootfs_group_label);
            package.map_or_else(
                || format!("Selected group: {group}\nSelected package: none"),
                |package| {
                    format!(
                        "Selected group: {group}\nSelected package: {}\nRecipe: {}\nCategory: {}\nInstalled bytes: {}\nReported files: {}",
                        package.identity.name,
                        package.recipe.as_deref().unwrap_or("unavailable"),
                        package.category,
                        package.installed_size_bytes,
                        package.file_count
                    )
                },
            )
        }
        ImagesView::RootfsFilesystem => {
            let entry = app.rootfs_entry_selection.as_ref().and_then(|identity| {
                composition.filesystem_tree().and_then(|tree| {
                    tree.entries
                        .iter()
                        .find(|entry| &entry.identity == identity)
                })
            });
            entry.map_or_else(
                || "Selected path: none".into(),
                |entry| {
                    format!(
                        "Selected path: {}\nKind: {:?}\nExact bytes: {}\nPackage: {}",
                        entry.identity.0.display(),
                        entry.kind,
                        entry.size_bytes,
                        entry
                            .package
                            .as_ref()
                            .map_or("unavailable", |package| package.name.as_str())
                    )
                },
            )
        }
        ImagesView::SystemdServices => composition
            .system_inventory()
            .and_then(|inventory| inventory.systemd_services.get(app.rootfs_systemd_selection))
            .map_or_else(|| "Selected service: none".into(), |service| format!(
                "Selected service: {}\nDescription: {}\nUnit file: {}\nBusName: {}\nEnabled by: {}\n\nUnit file preview{}\n{}\n\nEdits affect the generated IMAGE_ROOTFS and may be replaced by the next BitBake task.",
                service.name,
                service.description.as_deref().unwrap_or("unavailable"),
                service.logical_path.0.display(),
                service.bus_name.as_deref().unwrap_or("none"),
                if service.enabled_by.is_empty() { "none (disabled, static, indirect, or generated)".into() } else { service.enabled_by.join(", ") },
                if service.preview_truncated { " (truncated)" } else { "" },
                service.preview
            )),
        ImagesView::UdevRules => composition.system_inventory()
            .and_then(|inventory| inventory.udev_rules.get(app.rootfs_udev_selection))
            .map_or_else(|| "Selected rule: none".into(), |rule| format!(
                "Rule: {}\nImage path: {}\nFile selection: {}\nOverridden by: {}\nLimitation: {}\nOffline files only; rules are not executed and live device state is unknown.",
                rule.name, rule.logical_path.0.display(), rule.status(),
                rule.overridden_by.as_ref().map_or_else(|| "none".into(), |path| path.0.display().to_string()),
                rule.limitation.as_deref().unwrap_or("none")
            )),
        ImagesView::SystemDbus => composition
            .system_inventory()
            .and_then(|inventory| inventory.dbus_services.get(app.rootfs_dbus_selection))
            .map_or_else(|| "Selected system bus service: none".into(), |service| format!(
                "Bus name: {}\nActivation file: {}\nExec: {}\nUser: {}\nSystemdService: {}\nPolicy files:\n{}\n\nActivation/unit preview{}\n{}",
                service.name,
                service.logical_path.0.display(),
                service.exec.as_deref().unwrap_or("unavailable"),
                service.user.as_deref().unwrap_or("unavailable"),
                service.systemd_service.as_deref().unwrap_or("none"),
                if service.policy_files.is_empty() { "none matched".into() } else { service.policy_files.iter().map(|path| path.0.display().to_string()).collect::<Vec<_>>().join("\n") },
                if service.preview_truncated { " (truncated)" } else { "" },
                service.preview
            )),
    };
    let limitations = match &app.rootfs_composition {
        RootfsCompositionState::Partial { limitations, .. } => limitations
            .iter()
            .map(|value| format!("! {value}"))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => "none".into(),
    };
    format!(
        "Rootfs composition\nView: {}\nState: {state}\nImage: {}\nPath: {}\n{authority}\n\nExact totals\nPackage bytes: {}\nPackage files: {}\nFilesystem bytes: {}\nEntries: {} (files {} · dirs {} · symlinks {} · special {})\nOverflow: {}\n\n{selected}\n\nLimitations\n{limitations}",
        app.images_view.label(),
        composition.image.image,
        composition.image.path.display(),
        totals.installed_package_bytes,
        totals.package_reported_files,
        totals.filesystem_bytes,
        totals.entries,
        totals.files,
        totals.directories,
        totals.symlinks,
        totals.other,
        overflowed
    )
}

pub(crate) fn rootfs_authority_label<T>(authority: &yoctui_model::RootfsAuthority<T>) -> String {
    match authority {
        yoctui_model::RootfsAuthority::Available(_) => "available".into(),
        yoctui_model::RootfsAuthority::Partial { limitations, .. } => {
            format!("partial · {} limitation(s)", limitations.len())
        }
        yoctui_model::RootfsAuthority::Unavailable { reason } => {
            format!("unavailable · {reason}")
        }
    }
}
