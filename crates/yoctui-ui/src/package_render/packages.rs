pub(crate) fn packages_workspace(frame: &mut Frame, app: &App, area: Rect) {
    let palette = ThemePalette::for_app(app);
    let visible = app.filtered_packages();
    let selected = visible
        .iter()
        .position(|package| app.package_selection.as_ref() == Some(&package.identity));
    let workspace = Layout::vertical([Constraint::Length(1), Constraint::Min(3)]).split(area);
    frame.render_widget(
        Paragraph::new(search_line(
            app,
            &app.package_query,
            app.package_searching,
            selected,
            visible.len(),
            SearchNavigation::Results,
            SearchExit::Done,
            workspace[0].width,
        )),
        workspace[0],
    );
    let area = workspace[1];
    let block = pane_block(app, "Packages", app.focus == FocusTarget::Workspace);
    match &app.package_inventory {
        PackageInventoryState::NotLoaded => frame.render_widget(
            Paragraph::new(
                "Package data has not been loaded.\n\nEnter this workspace or press R to query generated pkgdata.",
            )
            .block(block)
            .wrap(Wrap { trim: false }),
            area,
        ),
        PackageInventoryState::Loading { .. } => frame.render_widget(
            Paragraph::new(
                "Loading authoritative package inventory…\n\nThe workspace remains responsive. Press c to cancel.",
            )
            .block(block)
            .wrap(Wrap { trim: false }),
            area,
        ),
        PackageInventoryState::AvailableEmpty { .. } => frame.render_widget(
            Paragraph::new("No built runtime packages were reported by oe-pkgdata-util.")
                .block(block)
                .wrap(Wrap { trim: false }),
            area,
        ),
        PackageInventoryState::Failed { message, .. } => frame.render_widget(
            Paragraph::new(format!(
                "Package inventory failed.\n\n{message}\n\nIf generated pkgdata is missing, build a target through do_package and press R."
            ))
            .block(block)
            .wrap(Wrap { trim: false }),
            area,
        ),
        PackageInventoryState::Available { .. } | PackageInventoryState::Partial { .. } => {
            let limitations = match &app.package_inventory {
                PackageInventoryState::Partial { limitations, .. } => limitations.as_slice(),
                _ => &[],
            };
            let rows_area = if limitations.is_empty() || area.height < 10 {
                area
            } else {
                Layout::vertical([Constraint::Min(4), Constraint::Length(4)]).split(area)[0]
            };
            let capacity = usize::from(rows_area.height.saturating_sub(3)).max(1);
            let viewport = yoctui_model::centered_viewport_range(
                selected,
                visible.len(),
                capacity,
            );
            let rows = visible[viewport]
                .iter()
                .copied()
                .map(|package| {
                    let selected = app.package_selection.as_ref() == Some(&package.identity);
                    let style = if selected {
                        palette.selected()
                    } else {
                        palette.base()
                    };
                    let name = package.identity.name.clone();
                    let recipe = package_field_text(&package.recipe);
                    let version = package_field_text(&package.version);
                    let size = package
                        .installed_size_bytes
                        .available()
                        .map_or_else(|| "unavailable".into(), |value| format_bytes(*value));
                    let license = package_field_text(&package.license);
                    Row::new(vec![name, recipe, version, size, license]).style(style)
                })
                .collect::<Vec<_>>();
            let widths = if area.width >= 68 {
                vec![
                    Constraint::Percentage(27),
                    Constraint::Percentage(22),
                    Constraint::Percentage(18),
                    Constraint::Percentage(14),
                    Constraint::Percentage(19),
                ]
            } else if area.width >= 44 {
                vec![
                    Constraint::Percentage(42),
                    Constraint::Percentage(33),
                    Constraint::Percentage(25),
                    Constraint::Length(0),
                    Constraint::Length(0),
                ]
            } else {
                vec![
                    Constraint::Percentage(100),
                    Constraint::Length(0),
                    Constraint::Length(0),
                    Constraint::Length(0),
                    Constraint::Length(0),
                ]
            };
            let header = Row::new(vec!["Package", "Recipe", "Version", "Size", "License"])
                .style(palette.role(palette.accent, Modifier::BOLD));
            frame.render_widget(
                Table::new(rows, widths)
                    .header(header)
                    .block(block)
                    .column_spacing(1),
                rows_area,
            );
            if !limitations.is_empty() && area.height >= 10 {
                let split =
                    Layout::vertical([Constraint::Min(4), Constraint::Length(4)]).split(area);
                frame.render_widget(
                    Paragraph::new(
                        limitations
                            .iter()
                            .take(2)
                            .map(|value| format!("! {value}"))
                            .collect::<Vec<_>>()
                            .join("\n"),
                    )
                    .style(palette.role(palette.warning, Modifier::BOLD))
                    .block(Block::default().title("Partial result").borders(Borders::ALL))
                    .wrap(Wrap { trim: true }),
                    split[1],
                );
            }
        }
    }
}

pub(crate) fn package_field_text(field: &PackageField<String>) -> String {
    match field {
        PackageField::Available(value) if value.is_empty() => "empty".into(),
        PackageField::Available(value) => value.clone(),
        PackageField::Unavailable => "unavailable".into(),
    }
}

pub(crate) fn package_path_field_text(field: &PackageField<std::path::PathBuf>) -> String {
    match field {
        PackageField::Available(value) => value.display().to_string(),
        PackageField::Unavailable => "unavailable".into(),
    }
}

pub(crate) fn package_identity_list(
    field: &PackageField<Vec<PackageIdentity>>,
    selected: Option<&PackageIdentity>,
) -> Vec<String> {
    match field {
        PackageField::Unavailable => vec!["  unavailable".into()],
        PackageField::Available(values) if values.is_empty() => vec!["  empty".into()],
        PackageField::Available(values) => values
            .iter()
            .map(|identity| {
                format!(
                    "{} {}",
                    if selected == Some(identity) {
                        "▶"
                    } else {
                        " "
                    },
                    identity.name
                )
            })
            .collect(),
    }
}

pub(crate) fn package_path_list(field: &PackageField<Vec<std::path::PathBuf>>) -> Vec<String> {
    match field {
        PackageField::Unavailable => vec!["  unavailable".into()],
        PackageField::Available(values) if values.is_empty() => vec!["  empty".into()],
        PackageField::Available(values) => values
            .iter()
            .take(64)
            .map(|path| format!("  {}", path.display()))
            .collect(),
    }
}

pub(crate) fn package_membership_text(field: &PackageField<Vec<String>>) -> String {
    match field {
        PackageField::Unavailable => "unavailable".into(),
        PackageField::Available(values) if values.is_empty() => "empty".into(),
        PackageField::Available(values) => values.join(", "),
    }
}

pub(crate) fn package_inspector_text(app: &App) -> String {
    let Some(package) = app.selected_package() else {
        return match &app.package_inventory {
            PackageInventoryState::Loading { .. } => {
                "Package inventory is loading.\n\nNo package is selected yet.".into()
            }
            PackageInventoryState::Failed { message, .. } => {
                format!("Package inventory failed.\n\n{message}")
            }
            PackageInventoryState::AvailableEmpty { .. } => {
                "The authoritative package inventory is empty.".into()
            }
            _ => "Select a package to inspect its typed metadata.".into(),
        };
    };
    let mut lines = vec![
        format!("Package: {}", package.identity.name),
        format!("Recipe: {}", package_field_text(&package.recipe)),
        format!("Provider: {}", package_path_field_text(&package.provider)),
        format!("Version: {}", package_field_text(&package.version)),
        format!(
            "Installed size: {}",
            package
                .installed_size_bytes
                .available()
                .map_or_else(|| "unavailable".into(), |value| format_bytes(*value))
        ),
        format!("License: {}", package_field_text(&package.license)),
        format!(
            "Image membership: {}",
            package_membership_text(&package.image_membership)
        ),
        String::new(),
    ];
    match app.selected_package_detail() {
        None | Some(PackageDetailState::NotLoaded) => {
            lines.push("Detail: not loaded (press Enter)".into());
        }
        Some(PackageDetailState::Loading { .. }) => {
            lines.push("Detail: loading… (press c to cancel)".into());
        }
        Some(PackageDetailState::Failed { message, .. }) => {
            lines.push(format!("Detail: failed\n{message}"));
        }
        Some(PackageDetailState::AvailableEmpty { .. }) => {
            lines.push("Detail: available-empty".into());
            lines.push("Files: empty".into());
            lines.push("Runtime dependencies: empty".into());
            lines.push("Reverse dependencies: empty".into());
        }
        Some(
            PackageDetailState::Available { detail, .. }
            | PackageDetailState::Partial { detail, .. },
        ) => {
            lines.push("Files:".into());
            lines.extend(package_path_list(&detail.files));
            lines.push(String::new());
            lines.push(format!(
                "{} runtime dependencies:",
                if app.package_dependency_reverse {
                    " "
                } else {
                    "▶"
                }
            ));
            lines.extend(package_identity_list(
                &detail.runtime_dependencies,
                (!app.package_dependency_reverse)
                    .then(|| app.selected_package_dependency())
                    .flatten(),
            ));
            lines.push(String::new());
            lines.push(format!(
                "{} reverse dependencies:",
                if app.package_dependency_reverse {
                    "▶"
                } else {
                    " "
                }
            ));
            lines.extend(package_identity_list(
                &detail.reverse_dependencies,
                app.package_dependency_reverse
                    .then(|| app.selected_package_dependency())
                    .flatten(),
            ));
            if let Some(PackageDetailState::Partial { limitations, .. }) =
                app.selected_package_detail()
            {
                lines.push(String::new());
                lines.push("Partial detail:".into());
                lines.extend(limitations.iter().map(|value| format!("! {value}")));
            }
        }
    }
    if !app.package_navigation.is_empty() {
        lines.push(String::new());
        lines.push(format!(
            "Navigation history: {} item(s); press u to return",
            app.package_navigation.len()
        ));
    }
    lines.join("\n")
}
