//! Image updates.
use super::*;

pub(crate) fn normalize_image_artifact_limitations(mut limitations: Vec<String>) -> Vec<String> {
    limitations = limitations
        .into_iter()
        .filter(|limitation| !limitation.is_empty() && !limitation.chars().any(char::is_control))
        .map(|limitation| limitation.chars().take(2_048).collect())
        .collect();
    limitations.sort();
    limitations.dedup();
    limitations.truncate(MAX_IMAGE_ARTIFACT_LIMITATIONS);
    limitations
}

pub(crate) fn append_image_artifact_normalization_limitations(
    limitations: &mut Vec<String>,
    report: &ImageArtifactNormalizationReport,
) {
    if report.invalid_records > 0 || report.invalid_fields > 0 {
        limitations.push(format!(
            "Model validation dropped {} image artifact record(s) and {} field value(s).",
            report.invalid_records, report.invalid_fields
        ));
    }
    if report.truncated_records > 0
        || report.truncated_associated_files > 0
        || report.truncated_checksums > 0
    {
        limitations.push(format!(
            "Model bounds truncated {} image artifact(s), {} associated file(s), and {} checksum record(s).",
            report.truncated_records,
            report.truncated_associated_files,
            report.truncated_checksums
        ));
    }
}

pub(crate) fn set_image_artifact_selection_to_current_or_first(
    app: &mut App,
    previous: Option<ImageArtifactIdentity>,
) {
    let visible = app
        .filtered_image_artifacts()
        .into_iter()
        .map(|artifact| artifact.identity.clone())
        .collect::<Vec<_>>();
    app.image_artifact_selection = previous
        .filter(|identity| visible.contains(identity))
        .or_else(|| visible.first().cloned());
}

pub(crate) fn rootfs_image_identity(app: &App) -> Option<ImageArtifactIdentity> {
    let artifacts = app.image_artifacts.artifacts()?;
    let is_recipe = |name: &str| {
        app.workspace
            .recipes
            .iter()
            .any(|recipe| recipe.name == name)
    };
    if let Some(selected) = app
        .image_artifact_selection
        .as_ref()
        .filter(|identity| is_recipe(&identity.image))
    {
        return Some(selected.clone());
    }
    let rootfs_rank = |artifact: &&ImageArtifact| match artifact.kind {
        ImageArtifactKind::RootFilesystem => 0,
        ImageArtifactKind::Wic => 1,
        ImageArtifactKind::Manifest => 2,
        _ => 3,
    };
    let candidate = |target: &str| {
        artifacts
            .iter()
            .filter(|artifact| artifact.identity.image == target)
            .filter(|artifact| rootfs_rank(artifact) < 3)
            .min_by_key(rootfs_rank)
            .map(|artifact| artifact.identity.clone())
    };
    app.build
        .target
        .as_deref()
        .filter(|target| is_recipe(target))
        .and_then(candidate)
        .or_else(|| {
            artifacts
                .iter()
                .filter(|artifact| is_recipe(&artifact.identity.image))
                .filter(|artifact| rootfs_rank(artifact) < 3)
                .min_by_key(rootfs_rank)
                .map(|artifact| artifact.identity.clone())
        })
}

pub(crate) fn set_image_artifact_inventory(
    app: &mut App,
    request: ImageArtifactRequest,
    inventory: ImageArtifactInventory,
    limitations: Option<Vec<String>>,
) {
    let previous = app.image_artifact_selection.take();
    let (inventory, report) =
        normalize_image_artifact_inventory(&request, inventory, MAX_IMAGE_ARTIFACT_RECORDS);
    let Some(inventory) = inventory else {
        app.image_artifacts = ImageArtifactInventoryState::Failed {
            request,
            message: "backend returned image artifacts for a different or invalid machine".into(),
        };
        app.notification =
            Some("Image artifact inventory failed model identity validation.".into());
        return;
    };
    let mut limitations = limitations.unwrap_or_default();
    append_image_artifact_normalization_limitations(&mut limitations, &report);
    let limitations = normalize_image_artifact_limitations(limitations);
    app.image_artifacts = if inventory.artifacts.is_empty() && limitations.is_empty() {
        ImageArtifactInventoryState::AvailableEmpty { request, inventory }
    } else if limitations.is_empty() {
        ImageArtifactInventoryState::Available { request, inventory }
    } else {
        ImageArtifactInventoryState::Partial {
            request,
            inventory,
            limitations,
        }
    };
    set_image_artifact_selection_to_current_or_first(app, previous);
}

pub(crate) fn next_package_generation(app: &mut App) -> u64 {
    app.package_request_generation = app.package_request_generation.wrapping_add(1);
    if app.package_request_generation == 0 {
        app.package_request_generation = 1;
    }
    app.package_request_generation
}

pub(crate) fn normalize_package_limitations(mut limitations: Vec<String>) -> Vec<String> {
    limitations.retain(|limitation| !limitation.is_empty());
    limitations.sort();
    limitations.dedup();
    limitations.truncate(MAX_PACKAGE_LIMITATIONS);
    limitations
}

pub(crate) fn append_package_normalization_limitations(
    limitations: &mut Vec<String>,
    report: &PackageNormalizationReport,
) {
    if report.invalid_records > 0 || report.invalid_fields > 0 {
        limitations.push(format!(
            "Model validation dropped {} package record(s) and {} field value(s).",
            report.invalid_records, report.invalid_fields
        ));
    }
    if report.truncated_records > 0
        || report.truncated_files > 0
        || report.truncated_dependencies > 0
        || report.truncated_image_memberships > 0
    {
        limitations.push(format!(
            "Model bounds truncated {} package(s), {} file(s), {} dependency value(s), and {} image membership value(s).",
            report.truncated_records,
            report.truncated_files,
            report.truncated_dependencies,
            report.truncated_image_memberships
        ));
    }
}

pub(crate) fn set_package_selection_to_current_or_first(
    app: &mut App,
    previous: Option<PackageIdentity>,
) {
    let visible = app
        .filtered_packages()
        .into_iter()
        .map(|package| package.identity.clone())
        .collect::<Vec<_>>();
    app.package_selection = previous
        .filter(|identity| visible.contains(identity))
        .or_else(|| visible.first().cloned());
}

pub(crate) fn set_package_inventory(
    app: &mut App,
    request: PackageInventoryRequest,
    packages: Vec<PackageSummary>,
    limitations: Option<Vec<String>>,
) {
    let previous = app.package_selection.take();
    let (packages, report) = normalize_package_summaries(packages, MAX_PACKAGE_RECORDS);
    let identities = packages
        .iter()
        .map(|package| package.identity.clone())
        .collect::<BTreeSet<_>>();
    app.package_details
        .retain(|identity, _| identities.contains(identity));
    let mut limitations = limitations.unwrap_or_default();
    append_package_normalization_limitations(&mut limitations, &report);
    let limitations = normalize_package_limitations(limitations);
    app.package_inventory = if packages.is_empty() && limitations.is_empty() {
        PackageInventoryState::AvailableEmpty { request }
    } else if limitations.is_empty() {
        PackageInventoryState::Available { request, packages }
    } else {
        PackageInventoryState::Partial {
            request,
            packages,
            limitations,
        }
    };
    set_package_selection_to_current_or_first(app, previous);
}

pub(crate) fn package_detail_is_empty(detail: &PackageDetail) -> bool {
    matches!(&detail.files, PackageField::Available(files) if files.is_empty())
        && matches!(
            &detail.runtime_dependencies,
            PackageField::Available(dependencies) if dependencies.is_empty()
        )
        && matches!(
            &detail.reverse_dependencies,
            PackageField::Available(dependencies) if dependencies.is_empty()
        )
}

pub(crate) fn begin_package_inventory(app: &mut App) -> Effect {
    let request = PackageInventoryRequest {
        generation: next_package_generation(app),
    };
    app.package_inventory = PackageInventoryState::Loading { request };
    Effect::GetPackageInventory(request)
}

pub(crate) fn package_operation_is_loading(app: &App) -> bool {
    matches!(app.package_inventory, PackageInventoryState::Loading { .. })
        || app
            .package_details
            .values()
            .any(|state| matches!(state, PackageDetailState::Loading { .. }))
}

pub(crate) fn begin_package_detail(app: &mut App, identity: PackageIdentity) -> Effect {
    let request = PackageDetailRequest {
        identity: identity.clone(),
        generation: next_package_generation(app),
    };
    app.package_details.insert(
        identity,
        PackageDetailState::Loading {
            request: request.clone(),
        },
    );
    Effect::GetPackageDetail(request)
}

pub(crate) fn select_package_identity(
    app: &mut App,
    identity: PackageIdentity,
    load_detail: bool,
) -> Option<Effect> {
    if !app
        .package_inventory
        .packages()
        .is_some_and(|packages| packages.iter().any(|package| package.identity == identity))
    {
        app.notification =
            Some("The dependency is not present in the current package inventory.".into());
        return None;
    }
    if let Some(current) = app.package_selection.replace(identity.clone())
        && current != identity
    {
        if app.package_navigation.len() == 64 {
            app.package_navigation.remove(0);
        }
        app.package_navigation.push(current);
    }
    app.package_dependency_selection = 0;
    if !load_detail || app.package_details.contains_key(&identity) {
        None
    } else {
        Some(begin_package_detail(app, identity))
    }
}

pub(crate) fn current_collection_edge_action(app: &App, to_end: bool) -> Option<Action> {
    let delta = if to_end { isize::MAX } else { isize::MIN };
    Some(match app.screen {
        Screen::Dashboard | Screen::Insights | Screen::Tasks => Action::ScrollBuildTasks { delta },
        Screen::BuildHistory => Action::SelectBuildHistory { delta },
        Screen::Dependencies => Action::SelectDependencyGraphNode { delta },
        Screen::Signatures => Action::SelectSignatureRecord { delta },
        Screen::Recipes | Screen::Devtool => Action::SelectRecipe { delta },
        Screen::Packages => Action::SelectPackage { delta },
        Screen::Images => Action::SelectImageArtifact { delta },
        Screen::Hardware => Action::Hardware(HardwareAction::SelectDocument { delta }),
        Screen::Kernel => Action::SelectKernelFile { delta },
        Screen::Firmware => Action::SelectFirmwareFile { delta },
        Screen::Sdk => Action::SelectSdkArtifact { delta },
        Screen::Testing => match app.test_view {
            TestWorkspaceView::Launches => Action::SelectTestFamily { delta },
            TestWorkspaceView::Results if app.test_result_drilled => {
                Action::SelectTestCase { delta }
            }
            TestWorkspaceView::Results => Action::SelectTestResult { delta },
            TestWorkspaceView::Comparison => Action::SelectTestComparisonTransition { delta },
        },
        Screen::Security => Action::Security(if app.security.view == SecurityView::Cves {
            SecurityAction::SelectFinding(delta)
        } else if app.security.drilled {
            SecurityAction::SelectComponent(delta)
        } else {
            SecurityAction::SelectReport(delta)
        }),
        Screen::Qa => Action::Qa(if app.qa.drilled {
            QaAction::SelectFinding(delta)
        } else if app.qa.view == QaView::LayerQa {
            QaAction::SelectLayer(delta)
        } else {
            QaAction::SelectCheck(delta)
        }),
        Screen::Layers if app.layer_browser.is_some() => Action::SelectLayerBrowserEntry { delta },
        Screen::Layers => Action::SelectLayer { delta },
        Screen::Configuration => Action::SelectConfigVariable { delta },
        Screen::RawMode => Action::RawMode(match app.raw_mode.view {
            RawModeView::Browser if app.raw_mode.browser_column == RawBrowserColumn::Categories => {
                RawModeAction::SelectCategory { delta }
            }
            RawModeView::Browser => RawModeAction::SelectCommand { delta },
            RawModeView::History => RawModeAction::SelectHistory { delta },
            RawModeView::Favorites => RawModeAction::SelectFavorite { delta },
            RawModeView::Execution => RawModeAction::ScrollOutput {
                vertical: if to_end { isize::MIN } else { isize::MAX },
                horizontal: 0,
            },
            RawModeView::Form | RawModeView::Preview => return None,
        }),
        Screen::TerminalSessions => Action::TerminalScroll {
            delta: if to_end { isize::MIN + 1 } else { isize::MAX },
        },
        Screen::Maintenance => Action::Maintenance(MaintenanceAction::Select {
            delta,
            row_count: match app.maintenance.view {
                MaintenanceView::Sstate => 2,
                MaintenanceView::Services => 1,
                MaintenanceView::Release | MaintenanceView::Integrations => 4,
            },
        }),
        Screen::Logs => Action::ScrollLogs {
            delta: delta.saturating_neg(),
        },
        Screen::Errors => Action::SelectError { delta },
        Screen::BuildEnvironment => Action::SelectBuildEnvironmentField { delta },
        Screen::Compatibility => Action::SelectCompatibilityCapability { delta },
        Screen::Settings => Action::SelectSetting { delta },
        Screen::LayerRelationships | Screen::Bbmask | Screen::Help => return None,
    })
}
