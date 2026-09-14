//! Inventory updates.
use super::*;

pub(crate) fn set_dependency_graph(
    app: &mut App,
    graph: DependencyGraph,
    limitations: Option<Vec<String>>,
) {
    let previous = app.dependency_graph_selection.take();
    app.dependency_graph_collapsed
        .retain(|identity| graph.contains(identity));
    app.dependency_graph_anchor = app
        .dependency_graph_anchor
        .take()
        .filter(|identity| graph.contains(identity));
    app.dependency_graph_selection = previous
        .filter(|selected| graph.contains(selected))
        .or_else(|| graph.contains(&graph.root).then(|| graph.root.clone()))
        .or_else(|| graph.nodes.first().map(|node| node.id.clone()));
    app.dependencies = Some(RecipeDependencies {
        recipe: graph.root.recipe_name().to_owned(),
        build: graph
            .edges
            .iter()
            .filter_map(|edge| {
                if edge.from == graph.root && edge.kind == DependencyEdgeKind::Build {
                    match &edge.to {
                        DependencyNodeId::Recipe(recipe) => Some(recipe.clone()),
                        DependencyNodeId::Task { .. } => None,
                    }
                } else {
                    None
                }
            })
            .collect(),
        runtime: graph
            .edges
            .iter()
            .filter_map(|edge| {
                if edge.from == graph.root && edge.kind == DependencyEdgeKind::Runtime {
                    match &edge.to {
                        DependencyNodeId::Recipe(recipe) => Some(recipe.clone()),
                        DependencyNodeId::Task { .. } => None,
                    }
                } else {
                    None
                }
            })
            .collect(),
    });
    app.dependency_selection = 0;
    app.screen = Screen::Dependencies;
    app.dependency_graph = if let Some(limitations) = limitations {
        DependencyGraphState::Partial { graph, limitations }
    } else if graph.edges.is_empty() {
        DependencyGraphState::AvailableEmpty { root: graph.root }
    } else {
        DependencyGraphState::Available(graph)
    };
}

pub(crate) fn signature_comparison_inputs(
    state: &SignatureComparisonState,
) -> (Option<SignatureIdentity>, Option<SignatureIdentity>) {
    match state {
        SignatureComparisonState::NotSelected => (None, None),
        SignatureComparisonState::Ready { left, right } => (left.clone(), right.clone()),
        SignatureComparisonState::Loading { request }
        | SignatureComparisonState::AvailableEmpty { request }
        | SignatureComparisonState::Available { request, .. }
        | SignatureComparisonState::Partial { request, .. }
        | SignatureComparisonState::Failed { request, .. } => {
            (Some(request.left.clone()), Some(request.right.clone()))
        }
    }
}

pub(crate) fn set_signature_dump(
    app: &mut App,
    target: SignatureTarget,
    records: Vec<SignatureRecord>,
    limitations: Option<Vec<String>>,
) {
    let (records, report) = normalize_signature_records(&target, records, MAX_SIGNATURE_RECORDS);
    let identities = records
        .iter()
        .map(|record| record.identity.clone())
        .collect::<BTreeSet<_>>();
    app.signature_selection = app
        .signature_selection
        .take()
        .filter(|selected| identities.contains(selected))
        .or_else(|| records.first().map(|record| record.identity.clone()));

    let (left, right) = signature_comparison_inputs(&app.signature_comparison);
    app.signature_comparison = SignatureComparisonState::Ready {
        left: left.filter(|identity| identities.contains(identity)),
        right: right.filter(|identity| identities.contains(identity)),
    };

    let mut limitations = limitations.unwrap_or_default();
    if report.is_partial() {
        limitations.push(format!(
            "Model bounds rejected {} invalid and truncated {} signature records.",
            report.invalid_records, report.truncated_records
        ));
    }
    app.signature_dump = if records.is_empty() && limitations.is_empty() {
        SignatureDumpState::AvailableEmpty { target }
    } else if limitations.is_empty() {
        SignatureDumpState::Available { target, records }
    } else {
        SignatureDumpState::Partial {
            target,
            records,
            limitations,
        }
    };
}

pub(crate) fn begin_signature_dump(app: &mut App, target: SignatureTarget) -> Option<Effect> {
    if let Err(message) = target.validate() {
        app.notification = Some(message.into());
        return None;
    }
    app.signature_dump = SignatureDumpState::Loading {
        target: target.clone(),
    };
    Some(Effect::GetSignatureDump(target))
}

pub(crate) fn signature_operation_is_loading(app: &App) -> bool {
    matches!(app.signature_dump, SignatureDumpState::Loading { .. })
        || matches!(
            app.signature_comparison,
            SignatureComparisonState::Loading { .. }
        )
}

pub(crate) fn next_image_artifact_generation(app: &mut App) -> u64 {
    app.image_artifact_request_generation = app.image_artifact_request_generation.wrapping_add(1);
    if app.image_artifact_request_generation == 0 {
        app.image_artifact_request_generation = 1;
    }
    app.image_artifact_request_generation
}

pub(crate) fn begin_image_artifact_inventory(app: &mut App) -> Option<Effect> {
    let machine = app
        .workspace
        .variables
        .get("MACHINE")
        .cloned()
        .unwrap_or_default();
    let request = ImageArtifactRequest {
        generation: next_image_artifact_generation(app),
        machine,
    };
    if let Err(message) = request.validate() {
        app.notification = Some(format!("Image artifacts are unavailable: {message}."));
        return None;
    }
    app.image_artifacts = ImageArtifactInventoryState::Loading {
        request: request.clone(),
    };
    Some(Effect::GetImageArtifacts(request))
}

pub(crate) fn image_artifact_operation_is_loading(app: &App) -> bool {
    matches!(
        app.image_artifacts,
        ImageArtifactInventoryState::Loading { .. }
    )
}

pub(crate) fn next_rootfs_generation(app: &mut App) -> u64 {
    app.rootfs_request_generation = app.rootfs_request_generation.wrapping_add(1).max(1);
    app.rootfs_request_generation
}

pub(crate) fn begin_rootfs_composition(
    app: &mut App,
    image: ImageArtifactIdentity,
) -> Option<Effect> {
    let request = RootfsCompositionRequest {
        generation: next_rootfs_generation(app),
        image,
    };
    if let Err(message) = request.validate() {
        app.notification = Some(format!("Rootfs composition is unavailable: {message}."));
        return None;
    }
    app.rootfs_composition = RootfsCompositionState::Loading {
        request: request.clone(),
    };
    Some(Effect::GetRootfsComposition(request))
}

pub(crate) fn normalize_rootfs_limitations(mut limitations: Vec<String>) -> Vec<String> {
    limitations.retain(|value| {
        !value.is_empty()
            && value.len() <= MAX_ROOTFS_TEXT_BYTES
            && !value.chars().any(char::is_control)
    });
    limitations.sort();
    limitations.dedup();
    limitations.truncate(MAX_ROOTFS_LIMITATIONS);
    limitations
}

pub(crate) fn append_rootfs_report_limitations(
    limitations: &mut Vec<String>,
    report: &RootfsNormalizationReport,
) {
    if report.invalid_packages > 0 || report.duplicate_packages > 0 {
        limitations.push(format!(
            "Model validation dropped {} invalid and {} duplicate installed-package record(s).",
            report.invalid_packages, report.duplicate_packages
        ));
    }
    if report.invalid_entries > 0 || report.duplicate_entries > 0 || report.orphan_entries > 0 {
        limitations.push(format!(
            "Model validation found {} invalid, {} duplicate, and {} orphan filesystem entry record(s).",
            report.invalid_entries, report.duplicate_entries, report.orphan_entries
        ));
    }
    if report.truncated_packages > 0 || report.truncated_entries > 0 || report.truncated_depth > 0 {
        limitations.push(format!(
            "Model bounds truncated {} package(s), {} filesystem entry record(s), and {} over-depth path(s).",
            report.truncated_packages, report.truncated_entries, report.truncated_depth
        ));
    }
    if report.invalid_limitations > 0 || report.truncated_limitations > 0 {
        limitations.push(format!(
            "Model bounds rejected {} and truncated {} limitation message(s).",
            report.invalid_limitations, report.truncated_limitations
        ));
    }
    if report.arithmetic_overflow > 0 {
        limitations.push("Rootfs totals exceeded the u64 display range.".into());
    }
}

pub(crate) fn rootfs_group_rows(app: &App) -> Vec<RootfsGroupRow> {
    app.rootfs_composition
        .composition()
        .and_then(RootfsComposition::package_inventory)
        .map(|inventory| inventory.grouped(12))
        .unwrap_or_default()
}

pub(crate) fn reconcile_rootfs_selection(
    app: &mut App,
    previous_group: Option<RootfsGroupIdentity>,
    previous_package: Option<PackageIdentity>,
    previous_entry: Option<RootfsPathIdentity>,
) {
    let groups = rootfs_group_rows(app);
    app.rootfs_group_selection = previous_group
        .filter(|identity| groups.iter().any(|row| &row.identity == identity))
        .or_else(|| groups.first().map(|row| row.identity.clone()));
    let members = app
        .rootfs_group_selection
        .as_ref()
        .and_then(|selected| groups.iter().find(|row| &row.identity == selected))
        .map(|row| row.members.as_slice())
        .unwrap_or_default();
    app.rootfs_package_selection = previous_package
        .filter(|identity| members.contains(identity))
        .or_else(|| members.first().cloned());
    let entries = app
        .rootfs_composition
        .composition()
        .and_then(RootfsComposition::filesystem_tree)
        .map(|tree| tree.entries.as_slice())
        .unwrap_or_default();
    app.rootfs_entry_selection = previous_entry
        .filter(|identity| entries.iter().any(|entry| &entry.identity == identity))
        .or_else(|| entries.first().map(|entry| entry.identity.clone()));
}

pub(crate) fn set_rootfs_composition(
    app: &mut App,
    request: RootfsCompositionRequest,
    composition: RootfsComposition,
    limitations: Vec<String>,
) {
    let previous_group = app.rootfs_group_selection.take();
    let previous_package = app.rootfs_package_selection.take();
    let previous_entry = app.rootfs_entry_selection.take();
    let (composition, mut report) = normalize_rootfs_composition(&request, composition);
    let Some(composition) = composition else {
        app.rootfs_composition = RootfsCompositionState::Failed {
            request,
            message: "backend returned rootfs composition for a different or invalid image".into(),
        };
        app.notification = Some("Rootfs composition failed model identity validation.".into());
        return;
    };
    if composition.totals().1 {
        report.arithmetic_overflow = 1;
    }
    let mut limitations = limitations;
    append_rootfs_report_limitations(&mut limitations, &report);
    if composition.is_partial() && limitations.is_empty() {
        limitations.push("One rootfs authority is partial or unavailable.".into());
    }
    let limitations = normalize_rootfs_limitations(limitations);
    app.record_overview_image_size(&composition);
    app.rootfs_composition = if composition.is_unavailable() {
        let mut reasons = Vec::new();
        if let RootfsAuthority::Unavailable { reason } = &composition.installed_packages {
            reasons.push(reason.as_str());
        }
        if let RootfsAuthority::Unavailable { reason } = &composition.filesystem_tree {
            reasons.push(reason.as_str());
        }
        if let RootfsAuthority::Unavailable { reason } = &composition.system_inventory {
            reasons.push(reason.as_str());
        }
        RootfsCompositionState::Unavailable {
            request,
            reason: reasons.join("; "),
        }
    } else if composition.is_empty() && limitations.is_empty() {
        RootfsCompositionState::AvailableEmpty {
            request,
            composition,
        }
    } else if limitations.is_empty() {
        RootfsCompositionState::Available {
            request,
            composition,
        }
    } else {
        RootfsCompositionState::Partial {
            request,
            composition,
            limitations,
        }
    };
    reconcile_rootfs_selection(app, previous_group, previous_package, previous_entry);
    if let Some(inventory) = app
        .rootfs_composition
        .composition()
        .and_then(RootfsComposition::system_inventory)
    {
        app.rootfs_systemd_selection = app
            .rootfs_systemd_selection
            .min(inventory.systemd_services.len().saturating_sub(1));
        app.rootfs_dbus_selection = app
            .rootfs_dbus_selection
            .min(inventory.dbus_services.len().saturating_sub(1));
        app.rootfs_udev_selection = app
            .rootfs_udev_selection
            .min(inventory.udev_rules.len().saturating_sub(1));
        app.rootfs_udev_preview_offset = 0;
    }
}
