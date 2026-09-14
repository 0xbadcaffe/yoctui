//! Render.
use super::*;

pub fn render(frame: &mut Frame, app: &App) {
    render_at(frame, app, SystemTime::now());
}

/// Render one frame with an injected clock.
///
/// Production uses [`render`]. Deterministic visual tests and the release
/// profiling workload use this entry point so clock changes cannot alter the
/// rendered cell buffer between otherwise identical frames.
pub(crate) fn selected_table_viewport(
    selection: usize,
    total: usize,
    area: Rect,
) -> std::ops::Range<usize> {
    yoctui_model::centered_viewport_range(
        (total > 0).then_some(selection),
        total,
        usize::from(area.height.saturating_sub(3)).max(1),
    )
}

pub fn render_at(frame: &mut Frame, app: &App, now: SystemTime) {
    let area = frame.area();
    let palette = ThemePalette::for_app(app);
    frame.render_widget(Block::default().style(palette.base()), area);
    if area.width < 80 || area.height < 24 {
        frame.render_widget(
            Paragraph::new(format!(
                "Yoctui needs at least 80x24.\nCurrent terminal: {}x{}.\nResize the terminal or press Q to quit.",
                area.width, area.height
            ))
                .block(Block::default().borders(Borders::ALL)),
            area,
        );
        return;
    }
    let concept_geometry = area.width == 160 && area.height == 50;
    let chunks = if concept_geometry {
        Layout::vertical([
            Constraint::Length(5),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(area)
    } else {
        Layout::vertical([
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .split(area)
    };
    workbench_header(frame, app, chunks[0], now);
    responsive_shell(frame, app, chunks[1], area.width, now);
    workbench_footer(frame, app, chunks[2], now);
    let screen_area = area;
    if app.menu.is_open() {
        if let Some(Dialog::RecipeEditor(editor)) = app.active_dialog() {
            recipe_editor(frame, app, editor, area);
        }
        menu_overlay(frame, app, area);
    } else if app.onboarding.open {
        onboarding_overlay(frame, app, area);
    } else if app.keymap_preferences_ui.open {
        keymap_preferences_overlay(frame, app, area);
    } else if app.command_palette_open {
        command_palette(frame, app, area);
    } else if app.screen == Screen::RawMode
        && let Some(form) = app
            .raw_mode
            .form
            .as_ref()
            .filter(|_| app.raw_mode.view == yoctui_model::RawModeView::Form)
    {
        raw_command_form_dialog(frame, app, form, area);
    } else if app.screen == Screen::RawMode
        && let Some(preview) = app
            .raw_mode
            .preview
            .as_ref()
            .filter(|_| app.raw_mode.view == yoctui_model::RawModeView::Preview)
    {
        let popup = dialog_popup_rect(area, 110, 30);
        clear_popup(frame, app, popup);
        render_raw_execution_preview(frame, preview, popup);
    } else if let Some(Dialog::RecipeEditor(editor)) = app.active_dialog() {
        recipe_editor(frame, app, editor, area);
    } else if let Some(Dialog::ImageConsole(dialog)) = app.active_dialog() {
        image_console_dialog(frame, app, dialog, area);
    } else if let Some(Dialog::QemuLaunch(dialog)) = app.active_dialog() {
        qemu_launch_dialog(frame, app, dialog, area);
    } else if let Some(Dialog::QemuLaunchConfirmation(preview)) = app.active_dialog() {
        qemu_launch_confirmation(frame, app, preview, area);
    } else if let Some(Dialog::QemuCancellationConfirmation(id)) = app.active_dialog() {
        qemu_cancellation_confirmation(frame, app, *id, area);
    } else if let Some(Dialog::WicCreateTomlEditor {
        editor,
        validation_error,
    }) = app.active_dialog()
    {
        toml_popup_editor(
            frame,
            app,
            area,
            "Wic create.toml",
            editor,
            validation_error.as_deref(),
        );
    } else if let Some(Dialog::WicCreate(dialog)) = app.active_dialog() {
        wic_create_dialog(frame, app, dialog, area);
    } else if let Some(Dialog::WicCreateConfirmation(preview)) = app.active_dialog() {
        wic_create_confirmation(frame, app, preview, area);
    } else if let Some(Dialog::WicDevicePicker(dialog)) = app.active_dialog() {
        wic_device_picker(frame, app, dialog, area);
    } else if let Some(Dialog::WicWritePhrase(dialog)) = app.active_dialog() {
        wic_write_phrase_dialog(frame, app, dialog, area);
    } else if let Some(Dialog::WicWriteConfirmation(preview)) = app.active_dialog() {
        wic_write_confirmation(frame, app, preview, area);
    } else if let Some(Dialog::WicCancellationConfirmation {
        id,
        incomplete_device_warning,
    }) = app.active_dialog()
    {
        wic_cancellation_confirmation(frame, app, *id, *incomplete_device_warning, area);
    } else if let Some(Dialog::SdkBuildConfirmation(preview)) = app.active_dialog() {
        sdk_build_confirmation(frame, app, preview, area);
    } else if let Some(Dialog::SdkPublishTomlEditor(editor)) = app.active_dialog() {
        toml_popup_editor(frame, app, area, "SDK publish.toml", editor, None);
    } else if let Some(Dialog::SdkPublish(draft)) = app.active_dialog() {
        sdk_publish_dialog(frame, app, draft, area);
    } else if let Some(Dialog::SdkPublishConfirmation(preview)) = app.active_dialog() {
        sdk_publish_confirmation(frame, app, preview, area);
    } else if let Some(Dialog::SdkNativeTomlEditor(editor)) = app.active_dialog() {
        toml_popup_editor(frame, app, area, "SDK native.toml", editor, None);
    } else if let Some(Dialog::SdkNative(draft)) = app.active_dialog() {
        sdk_native_dialog(frame, app, draft, area);
    } else if let Some(Dialog::SdkNativeConfirmation(preview)) = app.active_dialog() {
        sdk_native_confirmation(frame, app, preview, area);
    } else if let Some(Dialog::SdkCancellationConfirmation(id)) = app.active_dialog() {
        sdk_cancellation_confirmation(frame, app, *id, area);
    } else if let Some(Dialog::TestLaunchTomlEditor {
        editor,
        validation_error,
        ..
    }) = app.active_dialog()
    {
        toml_popup_editor(
            frame,
            app,
            area,
            "Test launch.toml",
            editor,
            validation_error.as_deref(),
        );
    } else if let Some(Dialog::TestLaunch(dialog)) = app.active_dialog() {
        test_launch_dialog(frame, app, dialog, area);
    } else if let Some(Dialog::TestLaunchConfirmation(preview)) = app.active_dialog() {
        test_launch_confirmation(frame, app, preview, area);
    } else if let Some(Dialog::TestCancellationConfirmation(id)) = app.active_dialog() {
        test_cancellation_confirmation(frame, app, *id, area);
    } else if let Some(Dialog::TestResultImportTomlEditor {
        editor,
        validation_error,
    }) = app.active_dialog()
    {
        toml_popup_editor(
            frame,
            app,
            area,
            "Test result import.toml",
            editor,
            validation_error.as_deref(),
        );
    } else if let Some(Dialog::TestResultImport(dialog)) = app.active_dialog() {
        test_result_import_dialog(frame, app, dialog, area);
    } else if let Some(Dialog::TestComparisonTomlEditor {
        editor,
        validation_error,
    }) = app.active_dialog()
    {
        toml_popup_editor(
            frame,
            app,
            area,
            "Test comparison.toml",
            editor,
            validation_error.as_deref(),
        );
    } else if let Some(Dialog::TestComparison(picker)) = app.active_dialog() {
        test_comparison_dialog(frame, app, picker, area);
    } else if let Some(Dialog::TestComparisonConfirmation(preview)) = app.active_dialog() {
        test_comparison_confirmation(frame, app, preview, area);
    } else if let Some(Dialog::TestJunitTomlEditor {
        editor,
        validation_error,
        ..
    }) = app.active_dialog()
    {
        toml_popup_editor(
            frame,
            app,
            area,
            "JUnit export.toml",
            editor,
            validation_error.as_deref(),
        );
    } else if let Some(Dialog::TestJunitExport(dialog)) = app.active_dialog() {
        test_junit_dialog(frame, app, dialog, area);
    } else if let Some(Dialog::TestJunitExportConfirmation(preview)) = app.active_dialog() {
        test_junit_confirmation(frame, app, preview, area);
    } else if let Some(Dialog::Security(SecurityDialog::Import {
        editor,
        validation_error,
    })) = app.active_dialog()
    {
        toml_popup_editor(
            frame,
            app,
            area,
            "Security import.toml",
            editor,
            validation_error.as_deref(),
        );
    } else if let Some(Dialog::Security(dialog)) = app.active_dialog() {
        security_dialog(frame, app, dialog, area);
    } else if let Some(Dialog::Qa(QaDialog::Import {
        editor,
        validation_error,
    })) = app.active_dialog()
    {
        toml_popup_editor(
            frame,
            app,
            area,
            "QA import.toml",
            editor,
            validation_error.as_deref(),
        );
    } else if let Some(Dialog::Qa(dialog)) = app.active_dialog() {
        qa_dialog(frame, app, dialog, area);
    } else if let Some(Dialog::Maintenance(dialog)) = app.active_dialog()
        && let Some((title, editor, validation_error)) = match dialog.as_ref() {
            MaintenanceDialog::ReadinessToml {
                editor,
                validation_error,
            } => Some(("Sstate readiness.toml", editor, validation_error)),
            MaintenanceDialog::CleanupToml {
                editor,
                validation_error,
            } => Some(("Sstate cleanup.toml", editor, validation_error)),
            MaintenanceDialog::PrServiceToml {
                operation,
                editor,
                validation_error,
            } => Some((
                match operation {
                    yoctui_model::PrServiceOperation::Export => "PR service export.toml",
                    yoctui_model::PrServiceOperation::Import => "PR service import.toml",
                },
                editor,
                validation_error,
            )),
            MaintenanceDialog::LockedCacheToml {
                editor,
                validation_error,
            } => Some(("Locked cache.toml", editor, validation_error)),
            MaintenanceDialog::BuildHistoryToml {
                editor,
                validation_error,
            } => Some(("Build history.toml", editor, validation_error)),
            MaintenanceDialog::GitArchiveToml {
                editor,
                validation_error,
            } => Some(("Git archive.toml", editor, validation_error)),
            _ => None,
        }
    {
        toml_popup_editor(frame, app, area, title, editor, validation_error.as_deref());
    } else if let Some(Dialog::Maintenance(dialog)) = app.active_dialog() {
        maintenance_dialog(frame, app, dialog, area);
    } else if matches!(app.active_dialog(), Some(Dialog::BuildCompletion)) {
        build_completion_popup(frame, app, area);
    } else if matches!(
        app.active_dialog(),
        Some(Dialog::BuildCancellationConfirmation)
    ) {
        let popup = bounded_dialog_rect(area, 58, 5);
        clear_popup(frame, app, popup);
        frame.render_widget(
            Paragraph::new(
                "Are you sure you want to cancel the build?\n\n[y/Enter] Cancel build  [n/Esc] Keep building",
            )
            .block(dialog_block(
                app,
                "Confirm build cancellation",
                DialogTone::Destructive,
            )),
            popup,
        )
    } else if matches!(app.active_dialog(), Some(Dialog::QuitConfirmation)) {
        let popup = bounded_dialog_rect(area, 56, 5);
        clear_popup(frame, app, popup);
        frame.render_widget(
            Paragraph::new(
                "Are you sure you want to exit yoctui?\n\n[y/Enter] Exit yoctui  [n/Esc] Stay",
            )
            .block(dialog_block(app, "Confirm exit", DialogTone::Destructive)),
            popup,
        )
    } else if let Some(Dialog::SignatureTaskPicker(picker)) = app.active_dialog() {
        let popup = Rect::new(
            area.width / 4,
            area.height / 4,
            area.width / 2,
            area.height / 2,
        );
        clear_popup(frame, app, popup);
        let viewport = selected_table_viewport(picker.selection, picker.tasks.len(), popup);
        frame.render_widget(
            Table::new(
                picker.tasks[viewport.clone()]
                    .iter()
                    .enumerate()
                    .map(|(offset, task)| {
                        let index = viewport.start + offset;
                        Row::new([format!(
                            "{} {task}",
                            if index == picker.selection {
                                "▶"
                            } else {
                                " "
                            }
                        )])
                        .style(selected_style(app, index == picker.selection))
                    }),
                [Constraint::Min(1)],
            )
            .header(Row::new(["Authoritative signature tasks"]).style(Style::default().bold()))
            .block(dialog_block(
                app,
                format!("Inspect signatures: {}", picker.recipe.name),
                DialogTone::Standard,
            )),
            popup,
        );
    } else if let Some(Dialog::RecipeTaskPicker(picker)) = app.active_dialog() {
        let popup = Rect::new(
            area.width / 4,
            area.height / 4,
            area.width / 2,
            area.height / 2,
        );
        clear_popup(frame, app, popup);
        let viewport = selected_table_viewport(picker.selection, picker.tasks.len(), popup);
        frame.render_widget(
            Table::new(
                picker.tasks[viewport.clone()]
                    .iter()
                    .enumerate()
                    .map(|(offset, task)| {
                        let index = viewport.start + offset;
                        Row::new([format!(
                            "{} {task}",
                            if index == picker.selection {
                                "▶"
                            } else {
                                " "
                            }
                        )])
                        .style(selected_style(app, index == picker.selection))
                    }),
                [Constraint::Min(1)],
            )
            .header(Row::new(["Authoritative BitBake tasks"]).style(Style::default().bold()))
            .block(dialog_block(
                app,
                format!(
                    "{} task: {}",
                    if picker.force { "Force" } else { "Run" },
                    picker.recipe
                ),
                DialogTone::Standard,
            )),
            popup,
        );
    } else if let Some(Dialog::RecipeTaskLogPicker(picker)) = app.active_dialog() {
        let popup = Rect::new(
            area.width / 5,
            area.height / 4,
            area.width * 3 / 5,
            area.height / 2,
        );
        clear_popup(frame, app, popup);
        let viewport = selected_table_viewport(picker.selection, picker.logs.len(), popup);
        frame.render_widget(
            Table::new(
                picker.logs[viewport.clone()]
                    .iter()
                    .enumerate()
                    .map(|(offset, log)| {
                        let index = viewport.start + offset;
                        Row::new([
                            Cell::from(format!(
                                "{} {}",
                                if index == picker.selection {
                                    "▶"
                                } else {
                                    " "
                                },
                                log.task
                            )),
                            Cell::from(format!("{:?}", log.state)),
                            Cell::from(log.path.display().to_string()),
                        ])
                        .style(selected_style(app, index == picker.selection))
                    }),
                [
                    Constraint::Length(20),
                    Constraint::Length(12),
                    Constraint::Min(20),
                ],
            )
            .header(
                Row::new(["Task", "State", "Authoritative log path"])
                    .style(Style::default().bold()),
            )
            .block(dialog_block(
                app,
                format!("{} retained task logs", picker.recipe),
                DialogTone::Standard,
            )),
            popup,
        );
    } else if let Some(Dialog::ConfigEdit {
        identity: _,
        editor,
    }) = app.active_dialog()
    {
        toml_popup_editor(frame, app, area, "Configuration.toml", editor, None);
    } else if let Some(Dialog::ConfigEditConfirmation(request)) = app.active_dialog() {
        let popup = Rect::new(
            area.width / 8,
            area.height / 4,
            area.width * 3 / 4,
            area.height / 2,
        );
        clear_popup(frame, app, popup);
        frame.render_widget(
            Paragraph::new(format!(
                "Destination:\n{}\n\nExact assignment:\n{}\n\nEnter confirms; Esc cancels.",
                request.destination.display(),
                request.assignment
            ))
            .block(dialog_block(
                app,
                "Preview configuration edit",
                DialogTone::Confirmation,
            ))
            .wrap(Wrap { trim: false }),
            popup,
        );
    } else if let Some(Dialog::ConfigComparison(comparison)) = app.active_dialog() {
        let popup = Rect::new(
            area.width / 8,
            area.height / 5,
            area.width * 3 / 4,
            area.height * 3 / 5,
        );
        clear_popup(frame, app, popup);
        let field = |name: &str, value: &yoctui_model::ConfigComparisonField| {
            format!(
                "{name}: {:?}\n  global: {}\n  {}: {}",
                value.outcome,
                value.global.as_deref().unwrap_or("unavailable"),
                comparison.recipe,
                value.recipe.as_deref().unwrap_or("unavailable")
            )
        };
        frame.render_widget(
            Paragraph::new(format!(
                "Variable: {}\nGlobal vs recipe {}\n\n{}\n\n{}\n\nEnter or Esc closes.",
                comparison.variable,
                comparison.recipe,
                field("Effective", &comparison.effective),
                field("Unexpanded", &comparison.unexpanded),
            ))
            .block(dialog_block(
                app,
                "Configuration comparison",
                DialogTone::Result,
            ))
            .wrap(Wrap { trim: false }),
            popup,
        );
    } else if let Some(Dialog::ConfigScopePicker(picker)) = app.active_dialog() {
        let popup = Rect::new(
            area.width / 5,
            area.height / 4,
            area.width * 3 / 5,
            area.height / 2,
        );
        clear_popup(frame, app, popup);
        let viewport = selected_table_viewport(picker.selection, picker.scopes.len(), popup);
        frame.render_widget(
            Table::new(
                picker.scopes[viewport.clone()]
                    .iter()
                    .enumerate()
                    .map(|(offset, scope)| {
                        let index = viewport.start + offset;
                        Row::new([format!(
                            "{} {}",
                            if index == picker.selection {
                                "▶"
                            } else {
                                " "
                            },
                            scope.as_deref().unwrap_or("(global)")
                        )])
                        .style(selected_style(app, index == picker.selection))
                    }),
                [Constraint::Min(20)],
            )
            .header(Row::new(["Variable scope"]).style(Style::default().bold()))
            .block(dialog_block(
                app,
                format!("{} scope", picker.variable),
                DialogTone::Standard,
            )),
            popup,
        );
    } else if let Some(Dialog::ConfigSourcePicker(picker)) = app.active_dialog() {
        let popup = Rect::new(
            area.width / 8,
            area.height / 4,
            area.width * 3 / 4,
            area.height / 2,
        );
        clear_popup(frame, app, popup);
        let viewport = selected_table_viewport(picker.selection, picker.sources.len(), popup);
        frame.render_widget(
            Table::new(
                picker.sources[viewport.clone()]
                    .iter()
                    .enumerate()
                    .map(|(offset, source)| {
                        let index = viewport.start + offset;
                        Row::new([
                            Cell::from(format!(
                                "{} {}",
                                if index == picker.selection {
                                    "▶"
                                } else {
                                    " "
                                },
                                source.operation
                            )),
                            Cell::from(source.path.display().to_string()),
                            Cell::from(
                                source
                                    .line
                                    .map_or_else(|| "—".into(), |line| line.to_string()),
                            ),
                        ])
                        .style(selected_style(app, index == picker.selection))
                    }),
                [
                    Constraint::Length(12),
                    Constraint::Min(20),
                    Constraint::Length(7),
                ],
            )
            .header(
                Row::new(["Operation", "Authoritative source", "Line"])
                    .style(Style::default().bold()),
            )
            .block(dialog_block(
                app,
                format!("{} defining sources", picker.identity.name),
                DialogTone::Standard,
            )),
            popup,
        );
    } else if let Some(Dialog::RecipePatchPicker(picker)) = app.active_dialog() {
        let popup = Rect::new(
            area.width / 5,
            area.height / 4,
            area.width * 3 / 5,
            area.height / 2,
        );
        clear_popup(frame, app, popup);
        let viewport = selected_table_viewport(picker.selection, picker.patches.len(), popup);
        frame.render_widget(
            Table::new(
                picker.patches[viewport.clone()]
                    .iter()
                    .enumerate()
                    .map(|(offset, patch)| {
                        let index = viewport.start + offset;
                        Row::new([format!(
                            "{} {}",
                            if index == picker.selection {
                                "▶"
                            } else {
                                " "
                            },
                            patch.display()
                        )])
                        .style(selected_style(app, index == picker.selection))
                    }),
                [Constraint::Min(20)],
            )
            .header(Row::new(["Authoritative local patch"]).style(Style::default().bold()))
            .block(dialog_block(
                app,
                format!("{} patch review", picker.recipe),
                DialogTone::Standard,
            )),
            popup,
        );
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
                        "{} Embedded in Yoctui (daemon-owned PTY)",
                        if dialog.destination == yoctui_model::TerminalLaunchDestination::Embedded {
                            "▶"
                        } else {
                            " "
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
                "Run `devtool deploy-target {} {}`?\n\nProvider: {}\nTarget: {}\n\nEnter continues; Esc cancels.",
                plan.identity.name,
                plan.target,
                plan.identity.file.display(),
                plan.target
            ))
            .block(dialog_block(
                app,
                "Confirm Devtool deploy-target",
                DialogTone::Confirmation,
            ))
            .wrap(Wrap { trim: true }),
            popup,
        );
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
                "Recipe: {}\nProvider: {}\nDeployment target: {}_\n\nEnter previews the command; Esc cancels.",
                draft.identity.name,
                draft.identity.file.display(),
                draft.target
            ))
            .block(dialog_block(
                app,
                "Devtool deploy target",
                DialogTone::Standard,
            ))
            .wrap(Wrap { trim: false }),
            popup,
        );
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
    } else if let Some(Dialog::BbmaskEdit(editor)) = app.active_dialog() {
        toml_popup_editor(frame, app, area, "BBMASK.toml", editor, None);
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
    } else if let Some(Dialog::EnvironmentSetup(setup)) = app.active_dialog() {
        environment_setup::environment_setup_popup(frame, app, setup, area);
    } else if let Some(Dialog::BuildEnvironmentCloneEditor(editor)) = app.active_dialog() {
        build_environment_clone_editor(frame, app, editor, area);
    } else if let Some(Dialog::BuildEnvironmentCloneReview(plan)) = app.active_dialog() {
        build_environment_clone_review(frame, app, plan, area);
    } else if let Some(Dialog::BuildEnvironmentEditor(editor)) = app.active_dialog() {
        build_environment_editor(frame, app, editor, area);
    } else if let Some(Dialog::ThemePicker { selection, .. }) = app.active_dialog() {
        theme_picker(frame, app, *selection, area);
    } else if matches!(app.active_dialog(), Some(Dialog::BuildOptions)) {
        let machine = app
            .workspace
            .variables
            .get("MACHINE")
            .map_or("unknown", String::as_str);
        let width = area.width.saturating_sub(12).clamp(38, 84);
        let popup = Rect::new(
            (area.width.saturating_sub(width)) / 2,
            area.height.saturating_sub(11) / 2,
            width,
            11,
        );
        clear_popup(frame, app, popup);
        frame.render_widget(
            Paragraph::new(format!(
                "Machine: {machine}\nCurrent image target: {}\n\nb  Build image\nc  Clean image\nm  Run menuconfig\ne  Enter a different image target\n\nEsc closes this menu.",
                app.build.target.as_deref().unwrap_or("not selected")
            ))
            .block(dialog_block(
                app,
                "Image build options",
                DialogTone::Standard,
            )),
            popup,
        );
    } else if let Some(Dialog::BuildTarget { editor, task }) = app.active_dialog() {
        let title = format!(
            "Build target.toml | requested task: {}",
            task.as_deref().unwrap_or("default")
        );
        toml_popup_editor(frame, app, area, &title, editor, None);
    }
    if !app.command_palette_open
        && let Some(dialog) = app.active_dialog()
    {
        dialog_compatibility_overlay(frame, app, dialog, screen_area);
    }
    if popup_notification(app).is_some()
        && app.active_dialog().is_none()
        && !app.menu.is_open()
        && !app.onboarding.open
        && !app.keymap_preferences_ui.open
        && !app.command_palette_open
        && !(app.screen == Screen::RawMode
            && matches!(
                app.raw_mode.view,
                yoctui_model::RawModeView::Form | yoctui_model::RawModeView::Preview
            ))
    {
        notification_popup(frame, app, screen_area);
    }
}
