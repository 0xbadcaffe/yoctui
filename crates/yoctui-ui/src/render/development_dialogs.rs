fn render_development_dialogs(frame: &mut Frame, app: &App, area: Rect) -> bool {
    if render_yocto_utility_dialog(frame, app, area) {
        return true;
    }
    if render_extended_devtool_dialogs(frame, app, area) {
        return true;
    }
    if let Some(Dialog::DtcCompile(dialog)) = app.active_dialog() {
        let popup = dialog_popup_rect(area, 84, 16);
        clear_popup(frame, app, popup);
        let mut lines = vec![
            Line::from(format!("Source: {}", dialog.source.display())),
            Line::from(format!("Output: {}", dialog.output.display())),
            Line::from(""),
        ];
        for index in 0..yoctui_model::DtcCompileOption::COUNT {
            let option = yoctui_model::DtcCompileOption::from_index(index);
            lines.push(Line::styled(
                format!(
                    "{} {:<28} {}",
                    if index == dialog.selection {
                        "▶"
                    } else {
                        " "
                    },
                    option.label(),
                    dialog.option_value(option),
                ),
                selected_style(app, index == dialog.selection),
            ));
        }
        lines.extend([
            Line::from(""),
            Line::from(format!("Arguments: {}", dialog.arguments().join(" "))),
            Line::from(""),
            Line::from("↑/↓ select · ←/→ or Space change · Enter review launch · Esc cancel"),
        ]);
        frame.render_widget(
            Paragraph::new(lines).block(dialog_block(
                app,
                "Compile device tree",
                DialogTone::Standard,
            )),
            popup,
        );
        return true;
    } else if let Some(Dialog::TerminalLaunch(dialog)) = app.active_dialog() {
        let width = area.width.saturating_sub(8).clamp(50, 96);
        let popup = Rect::new(
            (area.width.saturating_sub(width)) / 2,
            area.height.saturating_sub(12) / 2,
            width,
            12,
        );
        clear_popup(frame, app, popup);
        let detached = match &app.detached_terminal {
            yoctui_model::DetachedTerminalAvailability::Available { launcher } => {
                format!("Detached terminal — {launcher}")
            }
            yoctui_model::DetachedTerminalAvailability::Unavailable { reason } => {
                format!("Detached terminal — unavailable: {reason}")
            }
        };
        let command = std::iter::once(dialog.request.program.display().to_string())
            .chain(dialog.request.arguments.iter().cloned())
            .collect::<Vec<_>>()
            .join(" ");
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(format!("Session: {}", dialog.request.name)),
                Line::from(format!("Directory: {}", dialog.request.cwd.display())),
                Line::from(format!("Command: {command}")),
                Line::from(""),
                Line::styled(
                    format!(
                        "{} {}",
                        if dialog.destination == yoctui_model::TerminalLaunchDestination::Embedded {
                            "▶"
                        } else {
                            " "
                        },
                        if app.is_offline()
                            && dialog.request.kind == yoctui_model::TerminalCreationKind::GitUi
                        {
                            "Current terminal (offline GitUI; quit to return)"
                        } else {
                            "Embedded in Yoctui (daemon-owned PTY)"
                        }
                    ),
                    selected_style(
                        app,
                        dialog.destination == yoctui_model::TerminalLaunchDestination::Embedded,
                    ),
                ),
                Line::styled(
                    format!(
                        "{} {detached}",
                        if dialog.destination == yoctui_model::TerminalLaunchDestination::Detached {
                            "▶"
                        } else {
                            " "
                        }
                    ),
                    selected_style(
                        app,
                        dialog.destination == yoctui_model::TerminalLaunchDestination::Detached,
                    ),
                ),
                Line::from(""),
                Line::from("↑/↓ choose · Enter launch · Esc cancel without spawning"),
            ])
            .block(dialog_block(
                app,
                "Choose terminal destination",
                DialogTone::Confirmation,
            ))
            .wrap(Wrap { trim: true }),
            popup,
        );
        return true;
    } else if let Some(Dialog::RecipeTaskConfirmation(request)) = app.active_dialog() {
        let popup = Rect::new(area.width / 4, area.height / 3, area.width / 2, 5);
        clear_popup(frame, app, popup);
        frame.render_widget(
            Paragraph::new(format!(
                "Run `bitbake {}{} {}`?\n\nPress Enter to continue or Esc to cancel.",
                if request.force { "-f " } else { "" },
                request.targets.join(" "),
                request
                    .task
                    .as_deref()
                    .map_or(String::new(), |task| format!("-c {task}"))
            ))
            .block(dialog_block(
                app,
                "Confirm recipe task",
                DialogTone::Confirmation,
            ))
            .wrap(Wrap { trim: true }),
            popup,
        );
        return true;
    } else if let Some(Dialog::DevtoolModifyConfirmation(identity)) = app.active_dialog() {
        let width = area.width.saturating_sub(8).clamp(44, 100);
        let popup = Rect::new(
            (area.width.saturating_sub(width)) / 2,
            area.height.saturating_sub(7) / 2,
            width,
            7,
        );
        clear_popup(frame, app, popup);
        frame.render_widget(
            Paragraph::new(format!(
                "Run `devtool modify {}`?\n\nProvider: {}\n\nEnter continues; Esc cancels.",
                identity.name,
                identity.file.display()
            ))
            .block(dialog_block(
                app,
                "Confirm Devtool modify",
                DialogTone::Confirmation,
            ))
            .wrap(Wrap { trim: true }),
            popup,
        );
        return true;
    } else if let Some(Dialog::DevtoolResetConfirmation(plan)) = app.active_dialog() {
        let width = area.width.saturating_sub(8).clamp(44, 100);
        let popup = Rect::new(
            (area.width.saturating_sub(width)) / 2,
            area.height.saturating_sub(9) / 2,
            width,
            9,
        );
        clear_popup(frame, app, popup);
        frame.render_widget(
            Paragraph::new(format!(
                "Run `devtool reset {}`?\n\nProvider: {}\nWorkspace source to remove: {}\n\nThis removes the Devtool workspace. Enter continues; Esc cancels.",
                plan.identity.name,
                plan.identity.file.display(),
                plan.source_path.display()
            ))
            .block(dialog_block(
                app,
                "Confirm Devtool reset",
                DialogTone::Destructive,
            ))
            .wrap(Wrap { trim: true }),
            popup,
        );
        return true;
    } else if let Some(Dialog::DevtoolUpdateConfirmation(identity)) = app.active_dialog() {
        let width = area.width.saturating_sub(8).clamp(44, 100);
        let popup = Rect::new(
            (area.width.saturating_sub(width)) / 2,
            area.height.saturating_sub(7) / 2,
            width,
            7,
        );
        clear_popup(frame, app, popup);
        frame.render_widget(
            Paragraph::new(format!(
                "Run `devtool update-recipe {}`?\n\nProvider: {}\n\nEnter continues; Esc cancels.",
                identity.name,
                identity.file.display()
            ))
            .block(dialog_block(
                app,
                "Confirm Devtool update-recipe",
                DialogTone::Confirmation,
            ))
            .wrap(Wrap { trim: true }),
            popup,
        );
        return true;
    } else if let Some(Dialog::DevtoolPatchConfirmation(plan)) = app.active_dialog() {
        let width = area.width.saturating_sub(8).clamp(48, 110);
        let popup = Rect::new(
            (area.width.saturating_sub(width)) / 2,
            area.height.saturating_sub(10) / 2,
            width,
            10,
        );
        clear_popup(frame, app, popup);
        frame.render_widget(
            Paragraph::new(format!(
                "Create and install patches for {}?\n\nCommand: `devtool update-recipe --mode patch --append {} {}`\nProvider: {}\nConfigured layer: {}\n\nEnter continues; Esc cancels.",
                plan.identity.name,
                plan.layer.path.display(),
                plan.identity.name,
                plan.identity.file.display(),
                plan.layer.name,
            ))
            .block(dialog_block(
                app,
                "Confirm patch installation",
                DialogTone::Confirmation,
            ))
            .wrap(Wrap { trim: true }),
            popup,
        );
        return true;
    } else if let Some(Dialog::DevtoolPatchPicker(picker)) = app.active_dialog() {
        let width = area.width.saturating_sub(12).clamp(48, 110);
        let height = (picker.layers.len() as u16)
            .saturating_add(6)
            .min(area.height.saturating_sub(4))
            .max(8);
        let popup = Rect::new(
            (area.width.saturating_sub(width)) / 2,
            area.height.saturating_sub(height) / 2,
            width,
            height,
        );
        clear_popup(frame, app, popup);
        let viewport = selected_table_viewport(picker.selection, picker.layers.len(), popup);
        frame.render_widget(
            Table::new(
                picker.layers[viewport.clone()]
                    .iter()
                    .enumerate()
                    .map(|(offset, layer)| {
                        let index = viewport.start + offset;
                        Row::new([
                            format!(
                                "{} {}",
                                if index == picker.selection { "▶" } else { " " },
                                layer.name
                            ),
                            layer.path.display().to_string(),
                        ])
                        .style(selected_style(app, index == picker.selection))
                    }),
                [Constraint::Length(24), Constraint::Min(20)],
            )
            .header(
                Row::new(["Configured layer", "Patch destination"])
                    .style(Style::default().bold()),
            )
            .block(dialog_block(
                app,
                format!(
                    "Create/update patches for {} — ↑/↓ select, Enter preview, Esc cancel",
                    picker.identity.name
                ),
                DialogTone::Standard,
            )),
            popup,
        );
        return true;
    } else if let Some(Dialog::DevtoolFinishConfirmation(plan)) = app.active_dialog() {
        let width = area.width.saturating_sub(12).clamp(44, 100);
        let popup = Rect::new(
            (area.width.saturating_sub(width)) / 2,
            area.height.saturating_sub(9) / 2,
            width,
            9,
        );
        clear_popup(frame, app, popup);
        frame.render_widget(
            Paragraph::new(format!(
                "Run `devtool finish {} {}`?\n\nProvider: {}\nConfigured layer: {}\nDestination: {}\n\nEnter continues; Esc cancels.",
                plan.identity.name,
                plan.layer.path.display(),
                plan.identity.file.display(),
                plan.layer.name,
                plan.layer.path.display()
            ))
            .block(dialog_block(
                app,
                "Confirm Devtool finish",
                DialogTone::Destructive,
            ))
            .wrap(Wrap { trim: true }),
            popup,
        );
        return true;
    } else if let Some(Dialog::DevtoolDeployConfirmation(plan)) = app.active_dialog() {
        let width = area.width.saturating_sub(8).clamp(44, 100);
        let popup = Rect::new(
            (area.width.saturating_sub(width)) / 2,
            area.height.saturating_sub(8) / 2,
            width,
            8,
        );
        clear_popup(frame, app, popup);
        frame.render_widget(
            Paragraph::new(format!(
                "Deploy the built install tree with Devtool's SSH/SCP transport?\n\nCommand: `devtool deploy-target {} {}`\nProvider: {}\nTarget: {}\n\nEnter continues; Esc cancels.",
                plan.identity.name,
                plan.target,
                plan.identity.file.display(),
                plan.target
            ))
            .block(dialog_block(
                app,
                "Confirm SSH/SCP deployment",
                DialogTone::Confirmation,
            ))
            .wrap(Wrap { trim: true }),
            popup,
        );
        return true;
    } else if let Some(Dialog::DevtoolDeploy(draft)) = app.active_dialog() {
        let width = area.width.saturating_sub(12).clamp(44, 100);
        let popup = Rect::new(
            (area.width.saturating_sub(width)) / 2,
            area.height.saturating_sub(8) / 2,
            width,
            8,
        );
        clear_popup(frame, app, popup);
        frame.render_widget(
            Paragraph::new(format!(
                "Recipe: {}\nProvider: {}\nSSH target: {}_\n\nDevtool deploys the built install tree with SSH/SCP.\nEnter previews the command; Esc cancels.",
                draft.identity.name,
                draft.identity.file.display(),
                draft.target
            ))
            .block(dialog_block(
                app,
                "Deploy build with SSH/SCP",
                DialogTone::Standard,
            ))
            .wrap(Wrap { trim: false }),
            popup,
        );
        return true;
    } else if let Some(Dialog::DevtoolFinishPicker(picker)) = app.active_dialog() {
        let width = area.width.saturating_sub(12).clamp(44, 100);
        let height = (picker.layers.len() as u16)
            .saturating_add(5)
            .min(area.height.saturating_sub(4))
            .max(7);
        let popup = Rect::new(
            (area.width.saturating_sub(width)) / 2,
            area.height.saturating_sub(height) / 2,
            width,
            height,
        );
        clear_popup(frame, app, popup);
        let viewport = selected_table_viewport(picker.selection, picker.layers.len(), popup);
        frame.render_widget(
            Table::new(
                picker.layers[viewport.clone()]
                    .iter()
                    .enumerate()
                    .map(|(offset, layer)| {
                        let index = viewport.start + offset;
                        Row::new([
                            format!(
                                "{} {}",
                                if index == picker.selection {
                                    "▶"
                                } else {
                                    " "
                                },
                                layer.name
                            ),
                            layer.path.display().to_string(),
                        ])
                        .style(selected_style(app, index == picker.selection))
                    }),
                [Constraint::Length(24), Constraint::Min(20)],
            )
            .header(
                Row::new(["Configured layer", "Absolute destination"])
                    .style(Style::default().bold()),
            )
            .block(dialog_block(
                app,
                format!(
                    "Devtool finish {} — ↑/↓ select, Enter preview, Esc cancel",
                    picker.identity.name
                ),
                DialogTone::Standard,
            )),
            popup,
        );
        return true;
    } else if let Some(Dialog::BbmaskConfirmation(value)) = app.active_dialog() {
        let width = area.width.saturating_sub(12).clamp(40, 96);
        let popup = Rect::new(
            (area.width.saturating_sub(width)) / 2,
            area.height.saturating_sub(7) / 2,
            width,
            7,
        );
        clear_popup(frame, app, popup);
        frame.render_widget(
            Paragraph::new(format!(
                "Append this exact assignment to $BUILDDIR/conf/local.conf:\n\n{}\n\nEnter writes and refreshes configuration; Esc cancels.",
                bbmask_assignment(value)
            ))
            .block(dialog_block(
                app,
                "Confirm BBMASK change",
                DialogTone::Destructive,
            ))
            .wrap(Wrap { trim: false }),
            popup,
        );
        return true;
    } else if let Some(Dialog::BbmaskEdit(editor)) = app.active_dialog() {
        toml_popup_editor(frame, app, area, "BBMASK.toml", editor, None);
        return true;
    } else if let Some(Dialog::ImagePicker(picker)) = app.active_dialog() {
        let width = area.width.saturating_sub(24).clamp(42, 90);
        let height = area.height.saturating_sub(8).clamp(10, 24);
        let popup = Rect::new(
            (area.width.saturating_sub(width)) / 2,
            (area.height.saturating_sub(height)) / 2,
            width,
            height,
        );
        let machine = app
            .workspace
            .variables
            .get("MACHINE")
            .map_or("unknown", String::as_str);
        let viewport = yoctui_model::centered_viewport_range(
            (!picker.images.is_empty()).then_some(picker.selection),
            picker.images.len(),
            usize::from(popup.height.saturating_sub(6)).max(1),
        );
        let images = picker.images[viewport.clone()]
            .iter()
            .enumerate()
            .map(|(offset, image)| {
                let index = viewport.start + offset;
                format!(
                    "{} {}",
                    if index == picker.selection {
                        "▶"
                    } else {
                        " "
                    },
                    image
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        clear_popup(frame, app, popup);
        frame.render_widget(
            Paragraph::new(format!(
                "Active MACHINE: {machine}\n\n{images}\n\nUp/Down select  Enter choose image  Esc cancel"
            ))
            .block(dialog_block(
                app,
                "Available image targets",
                DialogTone::Standard,
            ))
            .wrap(Wrap { trim: false }),
            popup,
        );
        return true;
    } else if let Some(Dialog::RecipePicker(picker)) = app.active_dialog() {
        let width = area.width.saturating_sub(20).clamp(48, 100);
        let height = area.height.saturating_sub(8).clamp(10, 26);
        let popup = Rect::new(
            (area.width.saturating_sub(width)) / 2,
            (area.height.saturating_sub(height)) / 2,
            width,
            height,
        );
        let viewport = yoctui_model::centered_viewport_range(
            (!picker.recipes.is_empty()).then_some(picker.selection),
            picker.recipes.len(),
            usize::from(popup.height.saturating_sub(5)).max(1),
        );
        let recipes = picker.recipes[viewport.clone()]
            .iter()
            .enumerate()
            .map(|(offset, recipe)| {
                let index = viewport.start + offset;
                format!(
                    "{} {}  {}",
                    if index == picker.selection { "▶" } else { " " },
                    recipe.name,
                    recipe.file.display()
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        let title = match picker.purpose {
            yoctui_model::RecipePickerPurpose::Build => "Choose recipe to build",
            yoctui_model::RecipePickerPurpose::Dependencies => "Choose dependency root",
        };
        clear_popup(frame, app, popup);
        frame.render_widget(
            Paragraph::new(format!("{recipes}\n\nUp/Down select  Enter continue  Esc cancel"))
                .block(dialog_block(app, title, DialogTone::Standard))
                .wrap(Wrap { trim: false }),
            popup,
        );
        return true;
    }
    false
}
