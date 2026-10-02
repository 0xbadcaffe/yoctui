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
                ImagesView::RootfsPackages => {
                    if (78..90).contains(&width) {
                        "Packages"
                    } else {
                        "Rootfs packages"
                    }
                }
                ImagesView::RootfsFilesystem => "Files",
                ImagesView::SystemdServices => "systemd",
                ImagesView::SystemDbus => "D-Bus",
                ImagesView::UdevRules => "udev",
            };
            let label = if width < 78 && view != app.images_view {
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
                    if !(78..100).contains(&width) {
                        "  Tab switches"
                    } else {
                        ""
                    }
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
    match &app.image_artifacts {
        ImageArtifactInventoryState::NotLoaded => {
            lines.push(Line::from("Artifacts not loaded. Press Alt+r to scan."));
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
            let block = pane_block(app, "Images", app.focus == FocusTarget::Workspace);
            let inner = block.inner(area);
            frame.render_widget(block, area);
            if let ImageArtifactInventoryState::Partial { limitations, .. } = &app.image_artifacts {
                lines.push(Line::from(format!(
                    "Partial inventory: {} limitation(s); inspect selected row.",
                    limitations.len()
                )));
            }
            let header_height = (lines.len() as u16).min(inner.height);
            frame.render_widget(
                Paragraph::new(lines),
                Rect::new(inner.x, inner.y, inner.width, header_height),
            );
            let table_area = Rect::new(
                inner.x,
                inner.y.saturating_add(header_height),
                inner.width,
                inner.height.saturating_sub(header_height),
            );
            if filtered.is_empty() {
                frame.render_widget(
                    Paragraph::new("No artifacts match the active search."),
                    table_area,
                );
                return;
            }
            let capacity = usize::from(table_area.height.saturating_sub(1));
            let viewport =
                yoctui_model::centered_viewport_range(filtered_selection, filtered.len(), capacity);
            let show_time = table_area.width >= 52;
            let show_kind = table_area.width >= 78;
            let show_image = table_area.width >= 112;
            let mut titles = vec!["File", "Size (B)"];
            let mut widths = vec![Constraint::Min(12), Constraint::Length(11)];
            if show_time {
                titles.push("Modified UTC");
                widths.push(Constraint::Length(20));
            }
            if show_kind {
                titles.push("Kind");
                widths.push(Constraint::Length(16));
            }
            if show_image {
                titles.push("Image target");
                widths.push(Constraint::Length(24));
            }
            let rows = filtered[viewport].iter().map(|artifact| {
                let selected = app.image_artifact_selection.as_ref() == Some(&artifact.identity);
                let size = artifact
                    .size_bytes
                    .available()
                    .map_or_else(|| "unavailable".into(), |size| size.to_string());
                let file = artifact.identity.path.file_name().map_or_else(
                    || "unavailable".into(),
                    |name| name.to_string_lossy().into_owned(),
                );
                let mut cells = vec![file, size];
                if show_time {
                    cells.push(super::image_inspector::image_artifact_timestamp(
                        artifact.modified_unix_seconds.available().copied(),
                    ));
                }
                if show_kind {
                    cells.push(artifact.kind.label().into());
                }
                if show_image {
                    cells.push(artifact.identity.image.clone());
                }
                Row::new(cells)
                    .height(1)
                    .style(selected_style(app, selected))
            });
            frame.render_widget(
                Table::new(rows, widths).header(Row::new(titles)),
                table_area,
            );
            return;
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
