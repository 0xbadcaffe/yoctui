pub(crate) fn platform_workspace(
    frame: &mut Frame,
    app: &App,
    workbench: &PlatformWorkbench,
    area: Rect,
    fallback_title: &str,
) {
    let title = workbench
        .inventory()
        .map_or(fallback_title, |inventory| inventory.component.label());
    let block = pane_block(app, title, app.focus == FocusTarget::Workspace);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.is_empty() {
        return;
    }
    let selection = match workbench.view {
        yoctui_model::PlatformView::Configuration => workbench.config_selection,
        yoctui_model::PlatformView::DeviceTrees => workbench.device_tree_selection,
    };
    let tabs = Line::from(vec![
        Span::styled(
            " 1 Configuration ",
            if workbench.view == yoctui_model::PlatformView::Configuration {
                selected_style(app, true)
            } else {
                Style::default()
            },
        ),
        Span::raw(" │ "),
        Span::styled(
            " 2 Device trees ",
            if workbench.view == yoctui_model::PlatformView::DeviceTrees {
                selected_style(app, true)
            } else {
                Style::default()
            },
        ),
        Span::raw("  Tab switches"),
    ]);
    let mut lines = vec![tabs];
    match &workbench.inventory {
        PlatformInventoryState::NotLoaded => {
            lines.push(Line::from("Not inspected. Press r to scan."))
        }
        PlatformInventoryState::Loading => lines.push(Line::from(
            "Inspecting provider, configuration, and device trees…",
        )),
        PlatformInventoryState::Failed(message) => {
            lines.push(Line::from(format!("Inspection failed: {message}")))
        }
        PlatformInventoryState::Available(inventory) => {
            lines.push(Line::from(format!(
                "Target {} | provider {} | menuconfig {} | dtc {}",
                inventory.target,
                inventory
                    .provider
                    .as_ref()
                    .map_or_else(|| "unavailable".into(), |path| path.display().to_string()),
                if inventory
                    .tasks
                    .iter()
                    .any(|task| task == "menuconfig" || task == "do_menuconfig")
                {
                    "available"
                } else {
                    "unavailable"
                },
                inventory
                    .dtc
                    .as_ref()
                    .map_or("unavailable", |_| "available"),
            )));
            lines.push(Line::from("Kind         Size       File"));
            let files = workbench.visible_files().collect::<Vec<_>>();
            if files.is_empty() {
                lines.push(Line::from(match workbench.view {
                    yoctui_model::PlatformView::Configuration => {
                        "No .config file was found in the reported source/build roots."
                    }
                    yoctui_model::PlatformView::DeviceTrees => {
                        "No DTS, DTSI, DTB, or DTBO artifact was found."
                    }
                }));
            }
            let capacity = usize::from(inner.height.saturating_sub(4)).max(1);
            let viewport =
                yoctui_model::centered_viewport_range(Some(selection), files.len(), capacity);
            for (offset, file) in files[viewport.clone()].iter().enumerate() {
                let index = viewport.start + offset;
                lines.push(Line::styled(
                    format!(
                        "{:<12} {:>9}  {}",
                        file.kind.label(),
                        file.size_bytes,
                        file.path.display()
                    ),
                    selected_style(app, index == selection),
                ));
            }
            if let Some(limitation) = inventory.limitations.first() {
                lines.push(Line::styled(
                    format!("Limited: {limitation}"),
                    ThemePalette::for_app(app)
                        .role(ThemePalette::for_app(app).warning, Modifier::empty()),
                ));
            }
        }
    }
    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
}

pub(crate) fn platform_inspector_text(workbench: &PlatformWorkbench, title: &str) -> String {
    match &workbench.inventory {
        PlatformInventoryState::NotLoaded => format!("{title} has not been inspected."),
        PlatformInventoryState::Loading => format!("{title} inspection is loading."),
        PlatformInventoryState::Failed(message) => format!("{title} inspection failed: {message}"),
        PlatformInventoryState::Available(inventory) => {
            let selected = workbench.selected_file();
            format!(
                "Target: {}\nProvider: {}\nView: {}\nRoots: {}\nFiles: {}\nmenuconfig: {}\ndtc: {}\n\nSelected: {}\nKind: {}\nSize: {} bytes\n\nText sources open in the in-app explorer/editor. DTB and DTBO files are binary and can be decompiled to a new .yoctui.dts file.",
                inventory.target,
                inventory
                    .provider
                    .as_ref()
                    .map_or_else(|| "unavailable".into(), |path| path.display().to_string()),
                workbench.view.label(),
                inventory.roots.len(),
                inventory.files.len(),
                if inventory
                    .tasks
                    .iter()
                    .any(|task| task == "menuconfig" || task == "do_menuconfig")
                {
                    "available"
                } else {
                    "unavailable"
                },
                inventory
                    .dtc
                    .as_ref()
                    .map_or_else(|| "unavailable".into(), |path| path.display().to_string()),
                selected.map_or_else(|| "none".into(), |file| file.path.display().to_string()),
                selected.map_or("unavailable", |file| file.kind.label()),
                selected.map_or(0, |file| file.size_bytes),
            )
        }
    }
}
