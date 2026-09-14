//! Image render.
use super::*;

pub(crate) fn images_workspace(frame: &mut Frame, app: &App, area: Rect) {
    match app.images_view {
        ImagesView::Artifacts => image_artifacts_workspace(frame, app, area),
        ImagesView::RootfsPackages => rootfs_packages_workspace(frame, app, area),
        ImagesView::RootfsFilesystem => rootfs_filesystem_workspace(frame, app, area),
        ImagesView::SystemdServices => rootfs_systemd_workspace(frame, app, area),
        ImagesView::SystemDbus => rootfs_dbus_workspace(frame, app, area),
        ImagesView::UdevRules => rootfs_udev_workspace(frame, app, area),
    }
}

pub(crate) fn images_tabs_line(app: &App, width: u16) -> Line<'static> {
    let tabs = ImagesView::ALL
        .into_iter()
        .enumerate()
        .flat_map(|(index, view)| {
            let label = match view {
                ImagesView::Artifacts => "Artifacts",
                ImagesView::RootfsPackages => "Rootfs packages",
                ImagesView::RootfsFilesystem => "Files",
                ImagesView::SystemdServices => "systemd",
                ImagesView::SystemDbus => "D-Bus",
                ImagesView::UdevRules => "udev",
            };
            let label = if width < 90 && view != app.images_view {
                ""
            } else {
                label
            };
            let style = if app.images_view == view {
                selected_style(app, true)
            } else {
                Style::default()
            };
            [
                Span::styled(format!(" {} {label} ", index + 1), style),
                Span::raw(if index + 1 == ImagesView::ALL.len() {
                    "  Tab switches"
                } else {
                    " │ "
                }),
            ]
        });
    Line::from(tabs.collect::<Vec<_>>())
}

pub(crate) fn image_artifacts_workspace(frame: &mut Frame, app: &App, area: Rect) {
    let recipe_count = app
        .workspace
        .recipes
        .iter()
        .filter(|recipe| recipe.name.contains("image"))
        .count();
    let machine = app
        .workspace
        .variables
        .get("MACHINE")
        .map_or("unavailable", String::as_str);
    let filtered = app.filtered_image_artifacts();
    let filtered_selection = filtered
        .iter()
        .position(|artifact| app.image_artifact_selection.as_ref() == Some(&artifact.identity));
    let mut lines = vec![
        images_tabs_line(app, area.width),
        Line::from(format!(
            "MACHINE {machine} | build target {} | {recipe_count} image recipe target(s)",
            app.build.target.as_deref().unwrap_or("not selected")
        )),
    ];
    let recipe_targets = app
        .workspace
        .recipes
        .iter()
        .filter(|recipe| recipe.name.contains("image"))
        .take(8)
        .map(|recipe| recipe.name.as_str())
        .collect::<Vec<_>>();
    lines.push(Line::from(format!(
        "Recipe targets: {}",
        if recipe_targets.is_empty() {
            "none discovered".into()
        } else {
            recipe_targets.join(", ")
        }
    )));
    lines.push(search_line(
        app,
        &app.image_artifact_query,
        app.image_artifact_searching,
        filtered_selection,
        filtered.len(),
        SearchNavigation::Results,
        SearchExit::Done,
        area.width.saturating_sub(2),
    ));
    lines.push(Line::from(
        "Image target                 Kind             Size       Timestamp    File",
    ));
    match &app.image_artifacts {
        ImageArtifactInventoryState::NotLoaded => {
            lines.push(Line::from("Artifacts not loaded. Press R to scan."));
        }
        ImageArtifactInventoryState::Loading { .. } => {
            lines.push(Line::from("Loading deployed image artifacts…"));
        }
        ImageArtifactInventoryState::AvailableEmpty { .. } => {
            lines.push(Line::from(
                "No deployed image artifacts were found in DEPLOY_DIR_IMAGE.",
            ));
        }
        ImageArtifactInventoryState::Failed { message, .. } => {
            lines.push(Line::from(format!("Artifact scan failed: {message}")));
        }
        ImageArtifactInventoryState::Available { .. }
        | ImageArtifactInventoryState::Partial { .. } => {
            if filtered.is_empty() {
                lines.push(Line::from("No artifacts match the active search."));
            }
            let limitation_rows = usize::from(matches!(
                &app.image_artifacts,
                ImageArtifactInventoryState::Partial { .. }
            ));
            let capacity = usize::from(area.height.saturating_sub(7))
                .saturating_sub(limitation_rows)
                .max(1);
            let viewport =
                yoctui_model::centered_viewport_range(filtered_selection, filtered.len(), capacity);
            for artifact in filtered[viewport].iter().copied() {
                let selected = app.image_artifact_selection.as_ref() == Some(&artifact.identity);
                let size = artifact
                    .size_bytes
                    .available()
                    .map_or_else(|| "unavailable".into(), |size| size.to_string());
                let timestamp = artifact
                    .modified_unix_seconds
                    .available()
                    .map_or_else(|| "unavailable".into(), |value| value.to_string());
                let file = artifact
                    .identity
                    .path
                    .file_name()
                    .map_or_else(|| "unavailable".into(), |name| name.to_string_lossy());
                lines.push(
                    Line::from(format!(
                        "{:<28} {:<16} {:<10} {:<12} {}",
                        artifact.identity.image,
                        artifact.kind.label(),
                        size,
                        timestamp,
                        file
                    ))
                    .style(selected_style(app, selected)),
                );
            }
            if let ImageArtifactInventoryState::Partial { limitations, .. } = &app.image_artifacts {
                lines.push(Line::from(format!(
                    "Partial artifact inventory: {} limitation(s); inspect the selected row.",
                    limitations.len()
                )));
            }
        }
    };
    frame.render_widget(
        Paragraph::new(lines)
            .block(pane_block(
                app,
                "Images",
                app.focus == FocusTarget::Workspace,
            ))
            .wrap(Wrap { trim: false }),
        area,
    );
}

pub(crate) fn rootfs_state_lines(app: &App) -> Option<Vec<Line<'static>>> {
    let lines = match &app.rootfs_composition {
        RootfsCompositionState::NotLoaded => vec![
            Line::from("Rootfs composition is not loaded."),
            Line::from("Select an artifact, then press p or Enter."),
        ],
        RootfsCompositionState::Loading { request } => vec![Line::from(format!(
            "Loading rootfs composition for {}…",
            request.image.image
        ))],
        RootfsCompositionState::Unavailable { reason, .. } => vec![
            Line::from("Rootfs composition unavailable."),
            Line::from(reason.clone()),
        ],
        RootfsCompositionState::Failed { message, .. } => vec![
            Line::from("Rootfs composition failed."),
            Line::from(message.clone()),
        ],
        RootfsCompositionState::AvailableEmpty { request, .. } => vec![
            Line::from(format!(
                "Rootfs composition for {} is empty.",
                request.image.image
            )),
            Line::from("No installed packages or filesystem entries were reported."),
        ],
        RootfsCompositionState::Available { .. } | RootfsCompositionState::Partial { .. } => {
            return None;
        }
    };
    Some(lines)
}
