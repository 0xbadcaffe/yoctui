//! Rendering only; no backend parsing or mutation lives in widgets.
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Cell, Clear, Gauge, Paragraph, Row, Table, Wrap},
};
use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};
use yoctui_model::{
    App, BackgroundJobKind, BackgroundJobStatus, BuildStatus, ConfigCopyValue, DependencyEdgeKind,
    DependencyGraph, DependencyGraphState, DependencyNodeId, DependencyPathResult, DevtoolAction,
    DevtoolCapability, DevtoolGitState, DevtoolStatus, DevtoolStatusError, DevtoolWorkspace,
    Dialog, FocusTarget, GitFileState, ImageArtifactField, ImageArtifactInventoryState,
    LayerBrowser, LayerBrowserEntry, LayerInspectorMode, PackageDetailState, PackageField,
    PackageIdentity, PackageInventoryState, PreviewKind, QemuCapability, QemuDisplayMode,
    QemuLaunchDialog, QemuLaunchField, QemuLaunchPreview, QemuNetworkingMode, QemuSerialMode,
    QemuSessionId, Recipe, RecipeBuildStatus, RecipeEditor, RecipeIdentity, Screen, Severity,
    SignatureComparisonState, SignatureDifferenceCategory, SignatureDumpState, TaskFilterField,
    TaskRow, TaskState, Theme, VariableIdentity, config_comparison, config_edit_disabled_reason,
    config_source_disabled_reason, format_duration, selected_config_copy_value,
};

fn matches_metadata(query: &str, values: &[&str]) -> bool {
    let query = query.to_lowercase();
    query.is_empty()
        || values
            .iter()
            .any(|value| value.to_lowercase().contains(query.as_str()))
}

fn metadata_title(base: String, app: &App) -> String {
    if app.metadata_searching {
        format!("{base} | search: {}_", app.metadata_query)
    } else if app.metadata_query.is_empty() {
        base
    } else {
        format!("{base} | search: {}", app.metadata_query)
    }
}

fn timestamp_text(timestamp: SystemTime) -> String {
    timestamp.duration_since(UNIX_EPOCH).map_or_else(
        |_| "before Unix epoch".into(),
        |duration| format!("{}s since Unix epoch", duration.as_secs()),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ThemePalette {
    foreground: Color,
    background: Color,
    border: Color,
    focused_border: Color,
    selection_foreground: Color,
    selection_background: Color,
    disabled: Color,
    info: Color,
    success: Color,
    warning: Color,
    error: Color,
    progress: Color,
    accent: Color,
    syntax_keyword: Color,
    syntax_name: Color,
    syntax_operator: Color,
    syntax_value: Color,
    syntax_comment: Color,
    attribute_only: bool,
}

impl ThemePalette {
    fn for_app(app: &App) -> Self {
        if !app.color_enabled {
            return Self::monochrome();
        }
        match app.theme {
            Theme::Dark => Self {
                foreground: Color::Gray,
                background: Color::Reset,
                border: Color::DarkGray,
                focused_border: Color::Cyan,
                selection_foreground: Color::White,
                selection_background: Color::DarkGray,
                disabled: Color::DarkGray,
                info: Color::LightBlue,
                success: Color::Green,
                warning: Color::Yellow,
                error: Color::Red,
                progress: Color::LightBlue,
                accent: Color::Magenta,
                syntax_keyword: Color::LightBlue,
                syntax_name: Color::Yellow,
                syntax_operator: Color::Magenta,
                syntax_value: Color::Green,
                syntax_comment: Color::DarkGray,
                attribute_only: false,
            },
            Theme::Light => Self {
                foreground: Color::Black,
                background: Color::White,
                border: Color::Gray,
                focused_border: Color::Blue,
                selection_foreground: Color::White,
                selection_background: Color::Blue,
                disabled: Color::DarkGray,
                info: Color::Blue,
                success: Color::Green,
                warning: Color::Rgb(160, 96, 0),
                error: Color::Red,
                progress: Color::Blue,
                accent: Color::Magenta,
                syntax_keyword: Color::Blue,
                syntax_name: Color::Rgb(160, 96, 0),
                syntax_operator: Color::Magenta,
                syntax_value: Color::Green,
                syntax_comment: Color::DarkGray,
                attribute_only: false,
            },
            Theme::MatrixGreen => Self {
                foreground: Color::Green,
                background: Color::Black,
                border: Color::DarkGray,
                focused_border: Color::LightGreen,
                selection_foreground: Color::Black,
                selection_background: Color::Green,
                disabled: Color::DarkGray,
                info: Color::LightGreen,
                success: Color::LightGreen,
                warning: Color::Yellow,
                error: Color::LightRed,
                progress: Color::Green,
                accent: Color::LightGreen,
                syntax_keyword: Color::LightGreen,
                syntax_name: Color::Green,
                syntax_operator: Color::White,
                syntax_value: Color::LightGreen,
                syntax_comment: Color::DarkGray,
                attribute_only: false,
            },
            Theme::HighContrast => Self {
                foreground: Color::White,
                background: Color::Black,
                border: Color::White,
                focused_border: Color::Yellow,
                selection_foreground: Color::Black,
                selection_background: Color::White,
                disabled: Color::Gray,
                info: Color::Cyan,
                success: Color::LightGreen,
                warning: Color::Yellow,
                error: Color::LightRed,
                progress: Color::Cyan,
                accent: Color::Yellow,
                syntax_keyword: Color::Cyan,
                syntax_name: Color::Yellow,
                syntax_operator: Color::Magenta,
                syntax_value: Color::LightGreen,
                syntax_comment: Color::Gray,
                attribute_only: false,
            },
            Theme::Monochrome => Self::monochrome(),
        }
    }

    fn monochrome() -> Self {
        Self {
            foreground: Color::Reset,
            background: Color::Reset,
            border: Color::Reset,
            focused_border: Color::Reset,
            selection_foreground: Color::Reset,
            selection_background: Color::Reset,
            disabled: Color::Reset,
            info: Color::Reset,
            success: Color::Reset,
            warning: Color::Reset,
            error: Color::Reset,
            progress: Color::Reset,
            accent: Color::Reset,
            syntax_keyword: Color::Reset,
            syntax_name: Color::Reset,
            syntax_operator: Color::Reset,
            syntax_value: Color::Reset,
            syntax_comment: Color::Reset,
            attribute_only: true,
        }
    }

    fn base(self) -> Style {
        Style::default().fg(self.foreground).bg(self.background)
    }

    fn focus(self) -> Style {
        let style = Style::default().fg(self.focused_border);
        if self.attribute_only {
            style.add_modifier(Modifier::BOLD)
        } else {
            style
        }
    }

    fn selected(self) -> Style {
        if self.attribute_only {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
                .fg(self.selection_foreground)
                .bg(self.selection_background)
        }
    }

    fn role(self, color: Color, modifier: Modifier) -> Style {
        let style = Style::default().fg(color);
        if self.attribute_only {
            style.add_modifier(modifier)
        } else {
            style
        }
    }
}

fn selected_style(app: &App, active: bool) -> Style {
    if active {
        ThemePalette::for_app(app).selected()
    } else {
        Style::default()
    }
}
fn selected_log_style(app: &App, severity: Severity) -> Style {
    let palette = ThemePalette::for_app(app);
    let style = severity_style(app, severity);
    if palette.attribute_only {
        style.add_modifier(Modifier::REVERSED | Modifier::BOLD)
    } else {
        style
            .bg(palette.selection_background)
            .add_modifier(Modifier::BOLD)
    }
}

fn severity_style(app: &App, severity: Severity) -> Style {
    let palette = ThemePalette::for_app(app);
    match severity {
        Severity::Trace => palette.role(palette.disabled, Modifier::DIM),
        Severity::Info => palette.role(palette.info, Modifier::ITALIC),
        Severity::Warning => palette.role(palette.warning, Modifier::BOLD),
        Severity::Error => palette.role(palette.error, Modifier::BOLD | Modifier::UNDERLINED),
    }
}

fn build_status_style(app: &App) -> Style {
    let palette = ThemePalette::for_app(app);
    match app.build.status {
        yoctui_model::BuildStatus::Completed => palette.role(palette.success, Modifier::BOLD),
        yoctui_model::BuildStatus::Cancelled => palette.role(palette.warning, Modifier::BOLD),
        yoctui_model::BuildStatus::Failed => {
            palette.role(palette.error, Modifier::BOLD | Modifier::UNDERLINED)
        }
        yoctui_model::BuildStatus::LoadingWorkspace
        | yoctui_model::BuildStatus::Parsing
        | yoctui_model::BuildStatus::Running
        | yoctui_model::BuildStatus::Cancelling => palette.role(palette.progress, Modifier::BOLD),
        yoctui_model::BuildStatus::Idle => palette.role(palette.disabled, Modifier::DIM),
    }
}

fn clear_popup(frame: &mut Frame, app: &App, area: Rect) {
    frame.render_widget(Clear, area);
    let palette = ThemePalette::for_app(app);
    frame.render_widget(
        Block::default()
            .style(palette.base())
            .border_style(palette.focus()),
        area,
    );
}

fn active_yocto(app: &App) -> String {
    let release = app
        .workspace
        .release
        .as_deref()
        .unwrap_or("unknown release");
    let location = app
        .workspace
        .source_dir
        .as_deref()
        .or(app.workspace.build_dir.as_deref())
        .map_or_else(
            || "workspace unavailable".into(),
            |path| path.display().to_string(),
        );
    format!("{release} @ {location}")
}

fn source_preview(content: &str, file_name: &str, app: &App) -> Text<'static> {
    let bitbake_source = ["bb", "bbappend", "inc", "conf"]
        .iter()
        .any(|extension| file_name.ends_with(&format!(".{extension}")));
    let markdown = file_name.ends_with(".md") || file_name.ends_with(".markdown");
    let palette = ThemePalette::for_app(app);
    if palette.attribute_only || (!bitbake_source && !markdown) {
        return Text::from(content.to_owned());
    }
    Text::from(
        content
            .lines()
            .map(|line| {
                if markdown {
                    let style = if line.starts_with('#') {
                        Style::default().fg(palette.syntax_keyword)
                    } else if line.starts_with("```") {
                        Style::default().fg(palette.syntax_operator)
                    } else {
                        Style::default()
                    };
                    return Line::from(Span::styled(line.to_owned(), style));
                }
                let (code, comment) = line
                    .split_once('#')
                    .map_or((line, None), |(code, comment)| (code, Some(comment)));
                let mut spans = Vec::new();
                let trimmed = code.trim_start();
                let indent_len = code.len().saturating_sub(trimmed.len());
                if indent_len > 0 {
                    spans.push(Span::raw(code[..indent_len].to_owned()));
                }
                if [
                    "inherit", "require", "include", "export", "addtask", "deltask",
                ]
                .iter()
                .any(|keyword| trimmed.starts_with(keyword))
                {
                    let keyword_end = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
                    spans.push(Span::styled(
                        trimmed[..keyword_end].to_owned(),
                        Style::default().fg(palette.syntax_keyword),
                    ));
                    spans.push(Span::raw(trimmed[keyword_end..].to_owned()));
                } else if let Some(equals) = trimmed.find('=') {
                    let lhs_end = trimmed[..equals]
                        .trim_end_matches([' ', '?', '+', ':'])
                        .len();
                    spans.push(Span::styled(
                        trimmed[..lhs_end].to_owned(),
                        Style::default().fg(palette.syntax_name),
                    ));
                    spans.push(Span::styled(
                        trimmed[lhs_end..=equals].to_owned(),
                        Style::default().fg(palette.syntax_operator),
                    ));
                    spans.push(Span::styled(
                        trimmed[equals + 1..].to_owned(),
                        Style::default().fg(palette.syntax_value),
                    ));
                } else {
                    spans.push(Span::raw(trimmed.to_owned()));
                }
                if let Some(comment) = comment {
                    spans.push(Span::styled(
                        format!("#{comment}"),
                        Style::default().fg(palette.syntax_comment),
                    ));
                }
                Line::from(spans)
            })
            .collect::<Vec<_>>(),
    )
}

fn numbered_source_preview(content: &str, file_name: &str, app: &App) -> Text<'static> {
    let palette = ThemePalette::for_app(app);
    let mut source = source_preview(content, file_name, app);
    for (index, line) in source.lines.iter_mut().enumerate() {
        line.spans.insert(
            0,
            Span::styled(
                format!("{:>4} │ ", index + 1),
                palette.role(palette.disabled, Modifier::DIM),
            ),
        );
    }
    source
}

fn task_activity(app: &App, task_progress: Option<u8>) -> &'static str {
    if task_progress.is_some() {
        return "";
    }
    if app.reduced_motion {
        return " active";
    }
    const FAST: [&str; 8] = [
        "▸▸▸▸▸▸▸▸",
        "▹▸▸▸▸▸▸▸",
        "▹▹▸▸▸▸▸▸",
        "▹▹▹▸▸▸▸▸",
        "▹▹▹▹▸▸▸▸",
        "▹▹▹▹▹▸▸▸",
        "▹▹▹▹▹▹▸▸",
        "▹▹▹▹▹▹▹▸",
    ];
    FAST[(app.animation_frame as usize
        / if app.animation_speed == yoctui_model::AnimationSpeed::Slow {
            3
        } else {
            1
        })
        % FAST.len()]
}

fn footer_shortcuts(app: &App) -> &'static str {
    if app.screen == Screen::Signatures {
        return "↑/↓ select | 1/2 sides | c compare | r refresh | e provider | Esc back/cancel";
    }
    if app.focus == FocusTarget::Navigator {
        return "j/k or ↑/↓ select | Enter open | Tab workspace | Shift+Tab inspector | q quit";
    }
    if app.focus == FocusTarget::Inspector {
        return "Tab navigator | Shift+Tab workspace | ↑/↓ scroll inspector | / search | q quit";
    }
    if app.layer_browser.is_some() {
        return "↑/↓ select | →/l expand | ←/h collapse | Enter open/toggle | e editor | r refresh | . hidden | / search | g Git | m metadata | d deps";
    }
    match app.screen {
        Screen::Dashboard => {
            "F5 build | Ctrl+P commands | Tab focus | ↑/↓ package progress | i image | ! shell | c cancel | r recipes | y layers | ? help | q quit"
        }
        Screen::Tasks => {
            "↑/↓ select | f state | F field | / edit filter | d duration | c cancel | Tab focus"
        }
        Screen::BuildHistory => "↑/↓ select | Esc dashboard | ? help | q quit",
        Screen::Dependencies => {
            "↑/↓ or j/k select | Enter recipe | o provider | L task log | r refresh | Tab focus | Esc dashboard"
        }
        Screen::Signatures => {
            "↑/↓ select | 1/2 sides | c compare | r refresh | e provider | Esc back/cancel"
        }
        Screen::LayerRelationships => "Esc dashboard | y layers | ? help | q quit",
        Screen::Recipes => {
            "↑/↓ select | Enter inspect | z task/Z signatures | e provider | o logs | p patches | b/f tasks | V CVE | X SPDX | d modify | u update | F finish | P deploy | D reset | / search"
        }
        Screen::Packages => {
            "↑/↓ select | Enter detail | / search | R refresh | D dep kind | [/] dep | d follow | u back | o recipe | e provider | c cancel"
        }
        Screen::Images => {
            "↑/↓ select | Q launch QEMU | x cancel QEMU | / search | R refresh | c cancel scan | b build | i image picker | o artifact | m manifest | l license | s SPDX | w Wic"
        }
        Screen::Layers => {
            "↑/↓ select | Enter browse | i image | R relationships | e in-TUI edit | o external editor | / search | Esc dashboard | ? help | q quit"
        }
        Screen::Configuration => {
            "↑/↓ select | Enter inspect | s scope | c compare | C copy effective | U copy unexpanded | o source | E edit | / search | x BBMASK | Esc dashboard | ? help | q quit"
        }
        Screen::Bbmask => {
            "e edit BBMASK | Enter preview/confirm | Esc cancel/dashboard | v configuration | ? help | q quit"
        }
        Screen::Logs => {
            "↑/↓ select | ←/→ horizontal | f follow | w wrap | s severity | R/T/B filters | / search | n/N match | o source | C copy"
        }
        Screen::Errors => {
            "↑/↓ select | Enter logs | o open source | Esc dashboard | ? help | q quit"
        }
        Screen::Help => "Esc dashboard | q quit",
        Screen::Settings => {
            "↑/↓ select | ←/→ change | r retry save | Ctrl+P commands | Tab focus | q quit"
        }
    }
}

pub fn render(frame: &mut Frame, app: &App) {
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
    let chunks = Layout::vertical([
        Constraint::Length(4),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(area);
    let elapsed = app
        .elapsed()
        .map(format_duration)
        .unwrap_or_else(|| "--:--:--".into());
    let machine = app
        .workspace
        .variables
        .get("MACHINE")
        .map_or("unknown", String::as_str);
    let distro = app
        .workspace
        .variables
        .get("DISTRO")
        .map_or("unknown", String::as_str);
    let disk = app.host_telemetry.disk_available_bytes.map_or_else(
        || "Disk --".into(),
        |bytes| format!("Disk {}", format_bytes(bytes)),
    );
    frame.render_widget(
        Paragraph::new(format!(
            " Yoctui | {:?} | Yocto: {} | Target {} | MACHINE {} | DISTRO {}\n Status {:?} | Tasks {}/{} | Active {} | W {} | E {} | {} | CPU {} | {}",
            app.backend,
            active_yocto(app),
            app.build.target.as_deref().unwrap_or("not selected"),
            machine,
            distro,
            app.build.status,
            app.build.completed,
            app.build.total.map_or_else(|| "?".into(), |total| total.to_string()),
            app.tasks.len(),
            app.build.warnings,
            app.build.errors,
            elapsed,
            app.host_telemetry.cpu_utilization_percent.map_or_else(|| "CPU --".into(), |cpu| format!("CPU {cpu}%")),
            disk,
        ))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(build_status_style(app)),
        ),
        chunks[0],
    );
    responsive_shell(frame, app, chunks[1], area.width);
    frame.render_widget(
        Paragraph::new(footer_shortcuts(app)).style(palette.focus()),
        chunks[2],
    );
    if app.command_palette_open {
        command_palette(frame, app, area);
    } else if let Some(Dialog::RecipeEditor(editor)) = app.active_dialog() {
        recipe_editor(frame, app, editor, area);
    } else if let Some(Dialog::QemuLaunch(dialog)) = app.active_dialog() {
        qemu_launch_dialog(frame, app, dialog, area);
    } else if let Some(Dialog::QemuLaunchConfirmation(preview)) = app.active_dialog() {
        qemu_launch_confirmation(frame, app, preview, area);
    } else if let Some(Dialog::QemuCancellationConfirmation(id)) = app.active_dialog() {
        qemu_cancellation_confirmation(frame, app, *id, area);
    } else if matches!(app.active_dialog(), Some(Dialog::BuildCompletion)) {
        build_completion_popup(frame, app, area);
    } else if matches!(app.active_dialog(), Some(Dialog::QuitConfirmation)) {
        let popup = Rect::new(area.width / 4, area.height / 3, area.width / 2, 3);
        clear_popup(frame, app, popup);
        frame.render_widget(
            Paragraph::new("Build is active. Press Y to quit UI, or Esc to continue.")
                .block(Block::default().title("Confirm quit").borders(Borders::ALL)),
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
        frame.render_widget(
            Table::new(
                picker.tasks.iter().enumerate().map(|(index, task)| {
                    Row::new([task.as_str()]).style(selected_style(app, index == picker.selection))
                }),
                [Constraint::Min(1)],
            )
            .header(Row::new(["Authoritative signature tasks"]).style(Style::default().bold()))
            .block(
                Block::default()
                    .title(format!("Inspect signatures: {}", picker.recipe.name))
                    .borders(Borders::ALL),
            ),
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
        frame.render_widget(
            Table::new(
                picker.tasks.iter().enumerate().map(|(index, task)| {
                    Row::new([task.as_str()]).style(selected_style(app, index == picker.selection))
                }),
                [Constraint::Min(1)],
            )
            .header(Row::new(["Authoritative BitBake tasks"]).style(Style::default().bold()))
            .block(
                Block::default()
                    .title(format!(
                        "{} task: {}",
                        if picker.force { "Force" } else { "Run" },
                        picker.recipe
                    ))
                    .borders(Borders::ALL),
            ),
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
        frame.render_widget(
            Table::new(
                picker.logs.iter().enumerate().map(|(index, log)| {
                    Row::new([
                        Cell::from(log.task.as_str()),
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
            .block(
                Block::default()
                    .title(format!("{} retained task logs", picker.recipe))
                    .borders(Borders::ALL),
            ),
            popup,
        );
    } else if let Some(Dialog::ConfigEdit { identity, input }) = app.active_dialog() {
        let popup = Rect::new(area.width / 6, area.height / 3, area.width * 2 / 3, 7);
        clear_popup(frame, app, popup);
        frame.render_widget(
            Paragraph::new(format!(
                "Variable: {}\nNew effective value:\n{}_\n\nEnter previews; Esc cancels.",
                identity.name, input
            ))
            .block(
                Block::default()
                    .title("Edit global configuration")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false }),
            popup,
        );
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
            .block(
                Block::default()
                    .title("Preview configuration edit")
                    .borders(Borders::ALL),
            )
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
            .block(
                Block::default()
                    .title("Configuration comparison")
                    .borders(Borders::ALL),
            )
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
        frame.render_widget(
            Table::new(
                picker.scopes.iter().enumerate().map(|(index, scope)| {
                    Row::new([scope.as_deref().unwrap_or("(global)")])
                        .style(selected_style(app, index == picker.selection))
                }),
                [Constraint::Min(20)],
            )
            .header(Row::new(["Variable scope"]).style(Style::default().bold()))
            .block(
                Block::default()
                    .title(format!("{} scope", picker.variable))
                    .borders(Borders::ALL),
            ),
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
        frame.render_widget(
            Table::new(
                picker.sources.iter().enumerate().map(|(index, source)| {
                    Row::new([
                        Cell::from(source.operation.as_str()),
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
            .block(
                Block::default()
                    .title(format!("{} defining sources", picker.identity.name))
                    .borders(Borders::ALL),
            ),
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
        frame.render_widget(
            Table::new(
                picker.patches.iter().enumerate().map(|(index, patch)| {
                    Row::new([patch.display().to_string()])
                        .style(selected_style(app, index == picker.selection))
                }),
                [Constraint::Min(20)],
            )
            .header(Row::new(["Authoritative local patch"]).style(Style::default().bold()))
            .block(
                Block::default()
                    .title(format!("{} patch review", picker.recipe))
                    .borders(Borders::ALL),
            ),
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
            .block(
                Block::default()
                    .title("Confirm recipe task")
                    .borders(Borders::ALL),
            )
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
            .block(
                Block::default()
                    .title("Confirm Devtool modify")
                    .borders(Borders::ALL),
            )
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
            .block(
                Block::default()
                    .title("Confirm Devtool reset")
                    .borders(Borders::ALL),
            )
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
            .block(
                Block::default()
                    .title("Confirm Devtool update-recipe")
                    .borders(Borders::ALL),
            )
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
            .block(
                Block::default()
                    .title("Confirm Devtool finish")
                    .borders(Borders::ALL),
            )
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
            .block(
                Block::default()
                    .title("Confirm Devtool deploy-target")
                    .borders(Borders::ALL),
            )
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
            .block(
                Block::default()
                    .title("Devtool deploy target")
                    .borders(Borders::ALL),
            )
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
        frame.render_widget(
            Table::new(
                picker.layers.iter().enumerate().map(|(index, layer)| {
                    Row::new([layer.name.clone(), layer.path.display().to_string()])
                        .style(selected_style(app, index == picker.selection))
                }),
                [Constraint::Length(24), Constraint::Min(20)],
            )
            .header(
                Row::new(["Configured layer", "Absolute destination"])
                    .style(Style::default().bold()),
            )
            .block(
                Block::default()
                    .title(format!(
                        "Devtool finish {} — ↑/↓ select, Enter preview, Esc cancel",
                        picker.identity.name
                    ))
                    .borders(Borders::ALL),
            ),
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
            .block(Block::default().title("Confirm BBMASK change").borders(Borders::ALL))
            .wrap(Wrap { trim: false }),
            popup,
        );
    } else if let Some(Dialog::BbmaskEdit { input }) = app.active_dialog() {
        let width = area.width.saturating_sub(12).clamp(40, 96);
        let popup = Rect::new(
            (area.width.saturating_sub(width)) / 2,
            area.height.saturating_sub(6) / 2,
            width,
            6,
        );
        clear_popup(frame, app, popup);
        frame.render_widget(
            Paragraph::new(format!(
                "BBMASK: {}_\n\nEnter previews the exact local.conf assignment; Esc cancels.",
                input
            ))
            .block(
                Block::default()
                    .title("Edit effective BBMASK")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false }),
            popup,
        );
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
        let images = picker
            .images
            .iter()
            .enumerate()
            .map(|(index, image)| {
                format!(
                    "{} {}",
                    if index == picker.selection { ">" } else { " " },
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
            .block(Block::default().title("Available image targets").borders(Borders::ALL))
            .wrap(Wrap { trim: false }),
            popup,
        );
    } else if matches!(app.active_dialog(), Some(Dialog::BuildOptions)) {
        let machine = app
            .workspace
            .variables
            .get("MACHINE")
            .map_or("unknown", String::as_str);
        let width = area.width.saturating_sub(12).clamp(38, 84);
        let popup = Rect::new(
            (area.width.saturating_sub(width)) / 2,
            area.height.saturating_sub(10) / 2,
            width,
            10,
        );
        clear_popup(frame, app, popup);
        frame.render_widget(
            Paragraph::new(format!(
                "Machine: {machine}\nCurrent image target: {}\n\nb  Build image\nc  Clean image\nm  Run menuconfig\ne  Enter a different image target\n\nEsc closes this menu.",
                app.build.target.as_deref().unwrap_or("not selected")
            ))
            .block(Block::default().title("Image build options").borders(Borders::ALL)),
            popup,
        );
    } else if let Some(Dialog::BuildTarget { input, task }) = app.active_dialog() {
        let width = area.width.saturating_sub(12).clamp(30, 80);
        let popup = Rect::new(
            (area.width.saturating_sub(width)) / 2,
            area.height.saturating_sub(5) / 2,
            width,
            5,
        );
        clear_popup(frame, app, popup);
        frame.render_widget(
            Paragraph::new(format!(
                "Target: {}_\nTask: {}\n\nEnter starts the build; Esc cancels.",
                input,
                task.as_deref().unwrap_or("default")
            ))
            .block(Block::default().title("Build target").borders(Borders::ALL)),
            popup,
        );
    } else if let Some(notification) = app.notification.as_deref() {
        let palette = ThemePalette::for_app(app);
        let width = area.width.saturating_sub(8).clamp(24, 80);
        let popup = Rect::new(
            (area.width.saturating_sub(width)) / 2,
            area.height.saturating_sub(5) / 2,
            width,
            5,
        );
        clear_popup(frame, app, popup);
        frame.render_widget(
            Paragraph::new(format!("{notification}\n\nPress Enter to dismiss."))
                .style(palette.role(palette.info, Modifier::BOLD))
                .block(
                    Block::default()
                        .title("Notice")
                        .borders(Borders::ALL)
                        .border_style(palette.role(palette.accent, Modifier::BOLD)),
                )
                .wrap(Wrap { trim: true }),
            popup,
        );
    }
}

fn command_palette(frame: &mut Frame, app: &App, area: Rect) {
    let width = area.width.saturating_sub(12).clamp(60, 100);
    let height = area.height.saturating_sub(4).clamp(16, 26);
    let popup = Rect::new(
        (area.width.saturating_sub(width)) / 2,
        (area.height.saturating_sub(height)) / 2,
        width,
        height,
    );
    let commands = app.filtered_command_palette_commands();
    let visible_count = usize::from(height.saturating_sub(10)).max(1);
    let start = app
        .command_palette_selection
        .saturating_sub(visible_count.saturating_sub(1));
    let palette = ThemePalette::for_app(app);
    let mut lines = vec![
        Line::from(format!("Search: {}_", app.command_palette_query)),
        Line::from(""),
    ];
    if commands.is_empty() {
        lines.push(Line::styled(
            "No commands match this search.",
            palette.role(palette.disabled, Modifier::DIM),
        ));
    } else {
        lines.extend(
            commands
                .iter()
                .enumerate()
                .skip(start)
                .take(visible_count)
                .map(|(index, command)| {
                    let disabled = command.disabled_reason.is_some();
                    let marker = if index == app.command_palette_selection {
                        ">"
                    } else {
                        " "
                    };
                    let suffix = if disabled { " (unavailable)" } else { "" };
                    let style = if index == app.command_palette_selection {
                        selected_style(app, true)
                    } else if disabled {
                        palette.role(palette.disabled, Modifier::DIM)
                    } else {
                        Style::default()
                    };
                    Line::styled(
                        format!("{marker} {}  [{}]{suffix}", command.label, command.shortcut),
                        style,
                    )
                }),
        );
    }
    lines.push(Line::from(""));
    if let Some(command) = commands.get(app.command_palette_selection) {
        lines.push(Line::styled(
            command.description,
            palette.role(palette.info, Modifier::ITALIC),
        ));
        if let Some(reason) = command.disabled_reason {
            lines.push(Line::styled(
                format!("Unavailable: {reason}."),
                palette.role(palette.warning, Modifier::BOLD),
            ));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from(
        "Type to search  ↑/↓ select  Enter run  Backspace edit  Esc cancel",
    ));

    clear_popup(frame, app, popup);
    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(
                Block::default()
                    .title("Command palette")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: true }),
        popup,
    );
}

fn responsive_shell(frame: &mut Frame, app: &App, area: Rect, terminal_width: u16) {
    if app.screen == Screen::Signatures {
        signatures_workspace(frame, app, area, terminal_width);
        return;
    }
    if terminal_width >= 130 {
        let panes = Layout::horizontal([
            Constraint::Length(22),
            Constraint::Percentage(43),
            Constraint::Min(28),
        ])
        .split(area);
        navigator(frame, app, panes[0]);
        workspace(frame, app, panes[1]);
        inspector(frame, app, panes[2]);
    } else if terminal_width >= 100 {
        let panes = Layout::horizontal([Constraint::Length(22), Constraint::Min(40)]).split(area);
        navigator(frame, app, panes[0]);
        workspace(frame, app, panes[1]);
        if app.focus == FocusTarget::Inspector {
            frame.render_widget(Clear, panes[1]);
            inspector(frame, app, panes[1]);
        }
    } else {
        let rows = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).split(area);
        pane_switcher(frame, app, rows[0]);
        match app.focus {
            FocusTarget::Navigator => navigator(frame, app, rows[1]),
            FocusTarget::Inspector => inspector(frame, app, rows[1]),
            FocusTarget::Workspace | FocusTarget::Dialog | FocusTarget::CommandPalette => {
                workspace(frame, app, rows[1]);
            }
        }
    }
}

fn pane_switcher(frame: &mut Frame, app: &App, area: Rect) {
    let label = |target: FocusTarget, name: &str| {
        if app.focus == target {
            format!("[{name}]")
        } else {
            name.to_owned()
        }
    };
    frame.render_widget(
        Paragraph::new(format!(
            "Panes: {}  {}  {}  Tab/Shift+Tab",
            label(FocusTarget::Navigator, "Navigator"),
            label(FocusTarget::Workspace, "Workspace"),
            label(FocusTarget::Inspector, "Inspector"),
        ))
        .style(ThemePalette::for_app(app).focus()),
        area,
    );
}

fn pane_block<'a>(app: &App, title: &'a str, focused: bool) -> Block<'a> {
    let palette = ThemePalette::for_app(app);
    let style = if focused {
        palette.focus()
    } else {
        Style::default().fg(palette.border)
    };
    Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(style)
}

fn signatures_workspace(frame: &mut Frame, app: &App, area: Rect, terminal_width: u16) {
    if terminal_width >= 110 {
        let panes = Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area);
        signature_records(frame, app, panes[0]);
        signature_detail(frame, app, panes[1]);
    } else {
        let panes =
            Layout::vertical([Constraint::Percentage(44), Constraint::Percentage(56)]).split(area);
        signature_records(frame, app, panes[0]);
        signature_detail(frame, app, panes[1]);
    }
}

fn signature_target_label(app: &App) -> String {
    app.signature_dump.target().map_or_else(
        || "no target".into(),
        |target| format!("{}:{}", target.recipe, target.task),
    )
}

fn signature_comparison_sides(
    state: &SignatureComparisonState,
) -> (
    Option<&yoctui_model::SignatureIdentity>,
    Option<&yoctui_model::SignatureIdentity>,
) {
    match state {
        SignatureComparisonState::NotSelected => (None, None),
        SignatureComparisonState::Ready { left, right } => (left.as_ref(), right.as_ref()),
        SignatureComparisonState::Loading { request }
        | SignatureComparisonState::AvailableEmpty { request }
        | SignatureComparisonState::Available { request, .. }
        | SignatureComparisonState::Partial { request, .. }
        | SignatureComparisonState::Failed { request, .. } => {
            (Some(&request.left), Some(&request.right))
        }
    }
}

fn signature_records(frame: &mut Frame, app: &App, area: Rect) {
    let title = format!("Signatures — {}", signature_target_label(app));
    let block = pane_block(app, &title, app.focus == FocusTarget::Workspace);
    let records = app.signature_dump.records();
    let text = match &app.signature_dump {
        SignatureDumpState::NotLoaded => {
            "Signatures have not been loaded.\n\nReturn to Recipes and press Z.".into()
        }
        SignatureDumpState::Loading { .. } => {
            "Loading authoritative signature artifacts…\n\nEsc requests cancellation.".into()
        }
        SignatureDumpState::AvailableEmpty { .. } => {
            "BitBake reported no signature artifacts for this recipe/task.".into()
        }
        SignatureDumpState::Failed { message, .. } => {
            format!("Signature dump failed:\n{message}\n\nr retries; Esc returns to Recipes.")
        }
        SignatureDumpState::Available { .. } | SignatureDumpState::Partial { .. } => {
            let (left, right) = signature_comparison_sides(&app.signature_comparison);
            let mut lines = records
                .unwrap_or_default()
                .iter()
                .map(|record| {
                    let selected = app.signature_selection.as_ref() == Some(&record.identity);
                    let side = match (
                        left == Some(&record.identity),
                        right == Some(&record.identity),
                    ) {
                        (true, true) => "12",
                        (true, false) => "1 ",
                        (false, true) => " 2",
                        (false, false) => "  ",
                    };
                    format!(
                        "{} [{}] {}\n    {}",
                        if selected { ">" } else { " " },
                        side,
                        record
                            .identity
                            .hash
                            .as_deref()
                            .unwrap_or("hash unavailable"),
                        record.identity.path.as_ref().map_or_else(
                            || "path unavailable".into(),
                            |path| path.display().to_string()
                        )
                    )
                })
                .collect::<Vec<_>>();
            if let SignatureDumpState::Partial { limitations, .. } = &app.signature_dump {
                lines.push(String::new());
                lines.push("Partial result:".into());
                lines.extend(limitations.iter().map(|value| format!("! {value}")));
            }
            lines.join("\n")
        }
    };
    frame.render_widget(
        Paragraph::new(text).block(block).wrap(Wrap { trim: false }),
        area,
    );
}

fn signature_detail(frame: &mut Frame, app: &App, area: Rect) {
    frame.render_widget(
        Paragraph::new(signature_detail_text(app))
            .block(pane_block(
                app,
                "Selected record and comparison",
                app.focus == FocusTarget::Inspector,
            ))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn signature_detail_text(app: &App) -> String {
    let selected = app.signature_dump.records().and_then(|records| {
        app.signature_selection
            .as_ref()
            .and_then(|identity| records.iter().find(|record| &record.identity == identity))
    });
    let mut lines = Vec::new();
    if let Some(record) = selected {
        lines.extend([
            format!(
                "Hash: {}",
                record.identity.hash.as_deref().unwrap_or("unavailable")
            ),
            format!(
                "Base hash: {}",
                record.base_hash.as_deref().unwrap_or("unavailable")
            ),
            format!(
                "Task hash: {}",
                record.task_hash.as_deref().unwrap_or("unavailable")
            ),
            String::new(),
            format!("Variables ({})", record.variables.len()),
        ]);
        lines.extend(record.variables.iter().take(120).map(|value| {
            format!(
                "{} = {}",
                value.name,
                value.value.as_deref().unwrap_or("unavailable")
            )
        }));
        if record.variables.len() > 120 {
            lines.push(format!(
                "… {} more bounded variables",
                record.variables.len() - 120
            ));
        }
        lines.push(String::new());
        lines.push(format!("Task dependencies ({})", record.dependencies.len()));
        lines.extend(
            record
                .dependencies
                .iter()
                .take(80)
                .map(|dependency| format!("• {dependency}")),
        );
        if record.dependencies.len() > 80 {
            lines.push(format!(
                "… {} more bounded dependencies",
                record.dependencies.len() - 80
            ));
        }
    } else {
        lines.push("No current signature record is selected.".into());
    }
    lines.push(String::new());
    lines.push("Comparison".into());
    match &app.signature_comparison {
        SignatureComparisonState::NotSelected => {
            lines.push("Assign two records with 1 and 2.".into());
        }
        SignatureComparisonState::Ready { left, right } => {
            lines.push(format!(
                "1: {}\n2: {}",
                left.as_ref()
                    .and_then(|identity| identity.hash.as_deref())
                    .unwrap_or("not selected"),
                right
                    .as_ref()
                    .and_then(|identity| identity.hash.as_deref())
                    .unwrap_or("not selected")
            ));
        }
        SignatureComparisonState::Loading { .. } => {
            lines.push("Comparing authoritative signature artifacts…".into());
        }
        SignatureComparisonState::AvailableEmpty { .. } => {
            lines.push("No typed differences were found.".into());
        }
        SignatureComparisonState::Failed { message, .. } => {
            lines.push(format!("Comparison failed: {message}"));
        }
        SignatureComparisonState::Available { differences, .. }
        | SignatureComparisonState::Partial { differences, .. } => {
            lines.extend(differences.iter().take(160).map(|difference| {
                let category = match difference.category {
                    SignatureDifferenceCategory::BaseHash => "hash",
                    SignatureDifferenceCategory::ChangedValue => "value",
                    SignatureDifferenceCategory::Dependency => "dependency",
                    SignatureDifferenceCategory::Unavailable => "unavailable",
                };
                format!(
                    "[{category}] {}: {} → {}",
                    difference.key,
                    difference.left.as_deref().unwrap_or("unavailable"),
                    difference.right.as_deref().unwrap_or("unavailable")
                )
            }));
            if differences.len() > 160 {
                lines.push(format!(
                    "… {} more bounded differences",
                    differences.len() - 160
                ));
            }
            if let SignatureComparisonState::Partial { limitations, .. } = &app.signature_comparison
            {
                lines.push(String::new());
                lines.push("Partial comparison:".into());
                lines.extend(limitations.iter().map(|value| format!("! {value}")));
            }
        }
    }
    lines.join("\n")
}

fn navigator(frame: &mut Frame, app: &App, area: Rect) {
    let entries = [
        ("Dashboard", Screen::Dashboard),
        ("Layers", Screen::Layers),
        ("Recipes", Screen::Recipes),
        ("Packages", Screen::Packages),
        ("Images", Screen::Images),
        ("Tasks", Screen::Tasks),
        ("Logs", Screen::Logs),
        ("Errors", Screen::Errors),
        ("Configuration", Screen::Configuration),
        ("Dependencies", Screen::Dependencies),
        ("Devtool", Screen::Recipes),
        ("Maintenance", Screen::Bbmask),
        ("Settings", Screen::Settings),
    ];
    let text = entries
        .iter()
        .enumerate()
        .map(|(index, (name, _))| {
            format!(
                "{} {}",
                if index == app.navigator_selection {
                    "▶"
                } else {
                    " "
                },
                name
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    frame.render_widget(
        Paragraph::new(text).block(pane_block(
            app,
            "Navigator",
            app.focus == FocusTarget::Navigator,
        )),
        area,
    );
}

fn workspace(frame: &mut Frame, app: &App, area: Rect) {
    match app.screen {
        Screen::Dashboard => dashboard(frame, app, area),
        Screen::Tasks => tasks_workspace(frame, app, area),
        Screen::BuildHistory => build_history(frame, app, area),
        Screen::Dependencies => dependencies(frame, app, area),
        Screen::Signatures => signature_records(frame, app, area),
        Screen::LayerRelationships => layer_relationships(frame, app, area),
        Screen::Logs => logs(frame, app, area),
        Screen::Errors => errors(frame, app, area),
        Screen::Recipes => recipes(frame, app, area),
        Screen::Packages => packages_workspace(frame, app, area),
        Screen::Images => images_workspace(frame, app, area),
        Screen::Layers => {
            if let Some(browser) = app.layer_browser.as_ref() {
                layer_browser(frame, app, browser, area)
            } else {
                layers(frame, app, area)
            }
        }
        Screen::Configuration => config(frame, app, area),
        Screen::Bbmask => bbmask(frame, app, area),
        Screen::Help => help(frame, area),
        Screen::Settings => settings_workspace(frame, app, area),
    }
}

fn inspector(frame: &mut Frame, app: &App, area: Rect) {
    let details = match app.screen {
        Screen::Recipes => app.workspace.recipes.get(app.recipe_selection).map_or_else(
            || "No recipe selected.".into(),
            |recipe| recipe_inspector(app, recipe),
        ),
        Screen::Layers => app.layer_browser.as_ref().map_or_else(
            || {
                app.workspace.layers.get(app.layer_selection).map_or_else(
                    || "No layer selected.".into(),
                    |layer| {
                        format!(
                            "Layer: {}\nPath: {}\nPriority: {}\n\nEnter browses this layer.",
                            layer.name,
                            layer.path.display(),
                            layer
                                .priority
                                .map_or_else(|| "unknown".into(), |value| value.to_string())
                        )
                    },
                )
            },
            |browser| {
                browser.selected_entry().map_or_else(
                    || format!("Layer: {}\n\nThis layer is empty.", browser.layer),
                    |entry| {
                        let detail = layer_entry_metadata(app, browser, entry);
                        let preview = match browser.preview_kind {
                            PreviewKind::Binary => "Binary preview unavailable.",
                            PreviewKind::Text if !browser.preview.is_empty() => {
                                "Text preview is visible in the workspace."
                            }
                            _ => "Preview unavailable.",
                        };
                        format!("{detail}\n\n{preview}")
                    },
                )
            },
        ),
        Screen::Configuration => config_inspector(app),
        Screen::Logs => app.logs.selected().map_or_else(
            || "No logs retained.".into(),
            |entry| {
                format!(
                    "Time: {}\nSeverity: {:?}\nBuild: {}\nRecipe: {}\nTask: {}\nSource: {}\nProtected: {}\n\n{}",
                    timestamp_text(entry.timestamp),
                    entry.severity,
                    entry.build.as_deref().unwrap_or("unavailable"),
                    entry.recipe.as_deref().unwrap_or("unavailable"),
                    entry.task.as_deref().unwrap_or("unavailable"),
                    entry
                        .path
                        .as_ref()
                        .map_or_else(|| "unavailable".into(), |path| path.display().to_string()),
                    if entry.protected { "yes" } else { "no" },
                    entry.message,
                )
            },
        ),
        Screen::Errors => app
            .logs
            .diagnostics()
            .nth(app.error_selection)
            .map_or_else(
                || "No retained warnings or errors.".into(),
                |entry| diagnostic_detail(app, entry),
            ),
        Screen::Tasks => app.selected_task_row().map_or_else(
            || "No task selected.\n\nTask details appear as typed BitBake events arrive.".into(),
            |row| match row {
                TaskRow::WaitingSummary(count) => format!(
                    "Waiting tasks: {count}\n\nBitBake reported the overall task total, but individual queued-task metadata is not available yet."
                ),
                TaskRow::Task(task) => {
                    let now = SystemTime::now();
                    let elapsed = task
                        .elapsed_at(now)
                        .map(format_duration)
                        .unwrap_or_else(|| "unavailable".into());
                    let dependencies = if task.dependencies.is_empty() {
                        "unavailable".into()
                    } else {
                        task.dependencies
                            .iter()
                            .map(|dependency| dependency.0.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    };
                    let live_log = app
                        .logs
                        .entries
                        .iter()
                        .rev()
                        .find(|entry| {
                            entry.recipe.as_deref() == Some(task.recipe.as_str())
                                && entry.task.as_deref() == Some(task.task.as_str())
                        })
                        .map_or("unavailable", |entry| entry.message.as_str());
                    format!(
                        "Recipe: {}\nTask: {}\nState: {:?}\nProgress: {}\nWorker: {}\nPID: {}\nStarted: {}\nElapsed: {}\nDependencies: {}\nSource log: {}\nCancellation: {}\n\nLive log:\n{}",
                        task.recipe,
                        task.task,
                        task.state,
                        task.progress
                            .map_or_else(|| "unknown".into(), |value| format!("{value}%")),
                        task.worker.as_deref().unwrap_or("unavailable"),
                        task.pid.map_or_else(|| "unavailable".into(), |pid| pid.to_string()),
                        task.started
                            .map(timestamp_text)
                            .unwrap_or_else(|| "unavailable".into()),
                        elapsed,
                        dependencies,
                        task.log_path
                            .as_ref()
                            .map_or_else(|| "unavailable".into(), |path| path.display().to_string()),
                        task.cancellation.as_deref().unwrap_or("none"),
                        live_log,
                    )
                }
            },
        ),
        Screen::Dependencies => dependency_inspector(app),
        Screen::Signatures => signature_detail_text(app),
        Screen::Packages => package_inspector_text(app),
        Screen::Images => image_artifact_inspector_text(app),
        _ => format!(
            "Target: {}\nStatus: {:?}\n\nSelect an item in the workspace to inspect its details.",
            app.build.target.as_deref().unwrap_or("not selected"),
            app.build.status
        ),
    };
    frame.render_widget(
        Paragraph::new(details)
            .block(pane_block(
                app,
                "Inspector",
                app.focus == FocusTarget::Inspector,
            ))
            .wrap(Wrap { trim: false }),
        area,
    );
}

#[allow(dead_code)]
fn build_progress_popup(frame: &mut Frame, app: &App, area: Rect) {
    let width = area.width.saturating_sub(14).clamp(50, 110);
    let height = area.height.saturating_sub(6).clamp(12, 28);
    let popup = Rect::new(
        (area.width.saturating_sub(width)) / 2,
        (area.height.saturating_sub(height)) / 2,
        width,
        height,
    );
    let mut task_lines = app
        .tasks
        .values()
        .map(|task| {
            format!(
                "  {:<28} {:<18} {:>3}%",
                task.recipe,
                task.task,
                task.progress.unwrap_or(0)
            )
        })
        .collect::<Vec<_>>();
    task_lines.sort();
    if task_lines.is_empty() {
        task_lines.push("  Waiting for BitBake task events…".into());
    }
    let cpu = app
        .host_telemetry
        .cpu_utilization_percent
        .map_or_else(|| "sampling".into(), |value| format!("{value}%"));
    let disk = app
        .host_telemetry
        .disk_available_bytes
        .map_or_else(|| "unavailable".into(), format_bytes);
    let parse = match (app.build.parse_current, app.build.parse_total) {
        (Some(current), Some(total)) if total > 0 => format!(
            "{current}/{total} ({:.0}%)",
            current as f64 / total as f64 * 100.0
        ),
        _ => "not parsing".into(),
    };
    clear_popup(frame, app, popup);
    frame.render_widget(
        Paragraph::new(format!(
            "Target: {}\nStatus: {:?}    Tasks: {} complete, {} active\nParse: {parse}    CPU: {cpu}    Free disk: {disk}\n\nActive recipe tasks:\n{}\n\nBitBake is running. c cancels the build.",
            app.build.target.as_deref().unwrap_or("unknown"),
            app.build.status,
            app.build.completed,
            app.tasks.len(),
            task_lines.join("\n"),
        ))
        .block(Block::default().title("Build progress").borders(Borders::ALL))
        .wrap(Wrap { trim: false }),
        popup,
    );
}

fn build_completion_popup(frame: &mut Frame, app: &App, area: Rect) {
    let width = area.width.saturating_sub(24).clamp(44, 90);
    let popup = Rect::new(
        (area.width.saturating_sub(width)) / 2,
        area.height.saturating_sub(9) / 2,
        width,
        9,
    );
    let result = match app.build.status {
        yoctui_model::BuildStatus::Completed => "completed successfully",
        yoctui_model::BuildStatus::Cancelled => "was cancelled",
        _ => "failed",
    };
    let action = if app.build.status == yoctui_model::BuildStatus::Failed && app.build.errors > 0 {
        "Press Enter to investigate Errors; any other key returns to Yoctui."
    } else {
        "Press any key to return to Yoctui."
    };
    clear_popup(frame, app, popup);
    frame.render_widget(
        Paragraph::new(format!(
            "Build {} for {}.\n\nTasks completed: {}\nWarnings: {}    Errors: {}    Exit code: {}\nElapsed: {}\n\n{}",
            result,
            app.build.target.as_deref().unwrap_or("unknown target"),
            app.build.completed,
            app.build.warnings,
            app.build.errors,
            app.build.exit_code.map_or_else(|| "unknown".into(), |code| code.to_string()),
            app.elapsed().map(format_duration).unwrap_or_else(|| "unknown".into()),
            action,
        ))
        .style(build_status_style(app))
        .block(Block::default().title("Build finished").borders(Borders::ALL))
        .wrap(Wrap { trim: true }),
        popup,
    );
}
fn recipe_editor(frame: &mut Frame, app: &App, editor: &RecipeEditor, area: Rect) {
    let width = area.width.saturating_sub(4).max(30);
    let height = area.height.saturating_sub(2).max(8);
    let popup = Rect::new(
        (area.width.saturating_sub(width)) / 2,
        (area.height.saturating_sub(height)) / 2,
        width,
        height,
    );
    clear_popup(frame, app, popup);
    let columns =
        Layout::horizontal([Constraint::Percentage(35), Constraint::Percentage(65)]).split(popup);
    let files = editor
        .files
        .iter()
        .enumerate()
        .map(|(index, path)| {
            format!(
                "{} {}",
                if index == editor.selection { ">" } else { " " },
                path.display()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    frame.render_widget(
        Paragraph::new(files)
            .block(
                Block::default()
                    .title(format!("Workspace file tree: {}", editor.recipe))
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false }),
        columns[0],
    );
    let selected = editor
        .files
        .get(editor.selection)
        .map_or_else(|| "no file".into(), |path| path.display().to_string());
    let mode = if editor.editing {
        "editing"
    } else {
        "read-only"
    };
    let modified = if editor.dirty { " modified" } else { "" };
    let content = if editor.editing {
        format!("{}▏", editor.content)
    } else {
        editor.content.clone()
    };
    frame.render_widget(
        Paragraph::new(source_preview(&content, &selected, app))
            .block(
                Block::default()
                    .title(format!("{selected} ({mode}{modified})"))
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false }),
        columns[1],
    );
    let footer = Rect::new(
        popup.x,
        popup.y.saturating_add(popup.height.saturating_sub(1)),
        popup.width,
        1,
    );
    frame.render_widget(
        Paragraph::new(
            "↑/↓ file  Enter/e edit  Ctrl+S save  Ctrl+B build recipe  Esc return to Yoctui",
        )
        .style(ThemePalette::for_app(app).role(ThemePalette::for_app(app).disabled, Modifier::DIM)),
        footer,
    );
}
fn dashboard(frame: &mut Frame, app: &App, area: Rect) {
    let mut active = app.tasks.values().collect::<Vec<_>>();
    active.sort_by(|left, right| {
        (left.recipe.as_str(), left.task.as_str())
            .cmp(&(right.recipe.as_str(), right.task.as_str()))
    });
    let mut package_tasks = active.iter().map(|task| (*task, None)).collect::<Vec<_>>();
    package_tasks.extend(
        app.completed_tasks
            .iter()
            .rev()
            .map(|completed| (&completed.task, Some(completed.success))),
    );
    let recent = app
        .logs
        .entries
        .iter()
        .rev()
        .take(8)
        .rev()
        .map(|l| l.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let chunks =
        Layout::horizontal([Constraint::Percentage(45), Constraint::Percentage(55)]).split(area);
    let parse_progress = app.build.parse_current.map_or_else(
        || "not parsing".into(),
        |current| {
            app.build
                .parse_total
                .map_or_else(|| current.to_string(), |total| format!("{current}/{total}"))
        },
    );
    let cpu_utilization = app
        .host_telemetry
        .cpu_utilization_percent
        .map_or_else(|| "sampling".into(), |percent| format!("{percent}%"));
    let disk_available = app
        .host_telemetry
        .disk_available_bytes
        .map_or_else(|| "unavailable".into(), format_bytes);
    let build_panels =
        Layout::vertical([Constraint::Length(13), Constraint::Min(3)]).split(chunks[0]);
    frame.render_widget(
        Paragraph::new(format!(
            "Target: {}\nBackend: {}\nStatus: {}\nExit code: {}\nParse progress: {}\nMachine: {}\nDistro: {}\nRelease: {}\nTasks: {}/{} (active: {})\nWarnings: {}  Errors: {}\nHost CPU: {}  Build disk free: {}",
            app.build.target.as_deref().unwrap_or("none"),
            app.backend,
            app.build.status,
            app.build.exit_code.map_or_else(|| "none".into(), |code| code.to_string()),
            parse_progress,
            app.workspace
                .variables
                .get("MACHINE")
                .map_or("unknown", String::as_str),
            app.workspace
                .variables
                .get("DISTRO")
                .map_or("unknown", String::as_str),
            app.workspace.release.as_deref().unwrap_or("unknown"),
            app.build.completed,
            app.build
                .total
                .map_or_else(|| "?".into(), |total| total.to_string()),
            app.tasks.len(),
            app.build.warnings,
            app.build.errors,
            cpu_utilization,
            disk_available,
        ))
        .block(Block::default().title("Build").borders(Borders::ALL)),
        build_panels[0],
    );
    let task_count = package_tasks.len();
    let start = app.task_progress_scroll.min(task_count.saturating_sub(1));
    let task_block = Block::default()
        .title(format!(
            "Package task progress (? = progress unknown; {} active, {} complete; use Up/Down to scroll)",
            active.len(),
            app.completed_tasks.len()
        ))
        .borders(Borders::ALL);
    let task_area = task_block.inner(build_panels[1]);
    frame.render_widget(task_block, build_panels[1]);
    if package_tasks.is_empty() {
        frame.render_widget(
            Paragraph::new("Waiting for BitBake task events."),
            task_area,
        );
    } else {
        let rows = Layout::vertical(
            package_tasks[start..]
                .iter()
                .take(task_area.height as usize)
                .map(|_| Constraint::Length(1))
                .collect::<Vec<_>>(),
        )
        .split(task_area);
        for ((task, completed), row) in package_tasks[start..]
            .iter()
            .take(rows.len())
            .zip(rows.iter().copied())
        {
            let progress = if completed.is_some() {
                100
            } else {
                task.progress.unwrap_or(0).min(100)
            };
            let palette = ThemePalette::for_app(app);
            let progress_style = if *completed == Some(false) {
                palette.role(palette.error, Modifier::BOLD | Modifier::UNDERLINED)
            } else if progress >= 100 {
                palette.role(palette.success, Modifier::BOLD)
            } else if progress >= 75 {
                palette.role(palette.warning, Modifier::BOLD)
            } else {
                palette.role(palette.progress, Modifier::BOLD)
            };
            let label = if completed.is_some() {
                format!(
                    "{}:{} {progress}%{}",
                    task.recipe,
                    task.task,
                    match completed {
                        Some(true) => " complete",
                        Some(false) => " failed",
                        None => "",
                    }
                )
            } else if task.progress.is_some() {
                format!("{}:{} {progress}%", task.recipe, task.task)
            } else {
                format!(
                    "? {}:{}{}",
                    task.recipe,
                    task.task,
                    task_activity(app, None)
                )
            };
            frame.render_widget(
                Gauge::default()
                    .ratio(f64::from(progress) / 100.0)
                    .label(label)
                    .gauge_style(progress_style),
                row,
            );
        }
    }
    frame.render_widget(
        Paragraph::new(recent)
            .block(
                Block::default()
                    .title("Recent output")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false }),
        chunks[1],
    )
}

fn tasks_workspace(frame: &mut Frame, app: &App, area: Rect) {
    let rows = app.visible_task_rows();
    let waiting = app.waiting_task_count();
    let active = app
        .tasks
        .values()
        .filter(|task| task.state == TaskState::Active)
        .count();
    let failed = app
        .completed_tasks
        .iter()
        .filter(|task| !task.success)
        .count();
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Min(3),
    ])
    .split(area);
    let overall = Block::default()
        .title("Overall build progress")
        .borders(Borders::ALL);
    if let Some(total) = app.build.total.filter(|total| *total > 0) {
        let completed = app.build.completed.min(total);
        frame.render_widget(
            Gauge::default()
                .block(overall)
                .ratio(completed as f64 / total as f64)
                .label(format!(
                    "{}%  {completed}/{total} | active {active} | waiting {waiting} | failed {failed}",
                    completed.saturating_mul(100) / total
                ))
                .gauge_style(
                    ThemePalette::for_app(app)
                        .role(ThemePalette::for_app(app).progress, Modifier::BOLD),
                ),
            chunks[0],
        );
    } else {
        frame.render_widget(
            Paragraph::new(format!(
                "progress unknown | {} complete | {active} active | waiting unknown | {failed} failed",
                app.build.completed
            ))
            .block(overall),
            chunks[0],
        );
    }
    let duration = app
        .task_filters
        .minimum_duration
        .map_or_else(|| "all".into(), |value| format!("≥{}s", value.as_secs()));
    let active_field = match app.task_filter_field {
        TaskFilterField::Recipe => "recipe",
        TaskFilterField::Task => "task",
        TaskFilterField::Worker => "worker",
    };
    frame.render_widget(
        Paragraph::new(format!(
            "State {:?} | recipe '{}' | task '{}' | worker '{}' | duration {duration}\n{} field: {active_field}  F next field  / edit  f state  d duration",
            app.task_filters.state,
            app.task_filters.recipe,
            app.task_filters.task,
            app.task_filters.worker,
            if app.task_filter_editing { "Editing" } else { "Filter" },
        ))
        .block(Block::default().title("Filters").borders(Borders::ALL)),
        chunks[1],
    );
    let table_rows = rows.iter().enumerate().map(|(index, row)| {
        let values = match row {
            TaskRow::WaitingSummary(count) => vec![
                Cell::from("(queue)"),
                Cell::from(format!("{count} tasks")),
                Cell::from("--"),
                Cell::from("WAITING"),
                Cell::from("queued metadata unavailable"),
            ],
            TaskRow::Task(task) => vec![
                Cell::from(task.recipe.clone()),
                Cell::from(task.task.clone()),
                Cell::from(
                    task.elapsed_at(SystemTime::now())
                        .map(format_duration)
                        .unwrap_or_else(|| "--".into()),
                ),
                Cell::from(format!("{:?}", task.state).to_uppercase()),
                Cell::from(match (task.state, task.progress) {
                    (TaskState::Active, None) => {
                        format!("progress unknown{}", task_activity(app, None))
                    }
                    (_, Some(progress)) => format!("{progress}%"),
                    _ => "--".into(),
                }),
            ],
        };
        Row::new(values).style(selected_style(app, index == app.task_progress_scroll))
    });
    frame.render_widget(
        Table::new(
            table_rows,
            [
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Length(10),
                Constraint::Length(11),
                Constraint::Min(16),
            ],
        )
        .header(
            Row::new(["Recipe", "Task", "Elapsed", "State", "Progress"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(
            Block::default()
                .title(format!("Live Tasks ({} visible)", rows.len()))
                .borders(Borders::ALL),
        ),
        chunks[2],
    );
}

fn qemu_popup_rect(area: Rect, preferred_width: u16, preferred_height: u16) -> Rect {
    let width = preferred_width.min(area.width.saturating_sub(2)).max(1);
    let height = preferred_height.min(area.height.saturating_sub(2)).max(1);
    Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    )
}

fn qemu_launch_field_label(field: QemuLaunchField) -> &'static str {
    match field {
        QemuLaunchField::Machine => "Machine",
        QemuLaunchField::Image => "Image",
        QemuLaunchField::Kernel => "Kernel",
        QemuLaunchField::Rootfs => "Root filesystem",
        QemuLaunchField::Networking => "Networking",
        QemuLaunchField::Memory => "Memory MiB",
        QemuLaunchField::Display => "Display",
        QemuLaunchField::Serial => "Serial",
        QemuLaunchField::ExtraArguments => "Extra arguments",
    }
}

fn qemu_launch_field_value(dialog: &QemuLaunchDialog, field: QemuLaunchField) -> String {
    match field {
        QemuLaunchField::Machine => dialog.draft.machine.clone(),
        QemuLaunchField::Image => dialog.draft.image.path.display().to_string(),
        QemuLaunchField::Kernel => {
            if dialog.draft.kernel.is_empty() {
                "not set".into()
            } else {
                dialog.draft.kernel.clone()
            }
        }
        QemuLaunchField::Rootfs => {
            if dialog.draft.rootfs.is_empty() {
                "not set".into()
            } else {
                dialog.draft.rootfs.clone()
            }
        }
        QemuLaunchField::Networking => match dialog.draft.networking {
            QemuNetworkingMode::Slirp => "slirp",
            QemuNetworkingMode::Tap => "tap",
            QemuNetworkingMode::None => "none",
        }
        .into(),
        QemuLaunchField::Memory => dialog.draft.memory_mib.clone(),
        QemuLaunchField::Display => match dialog.draft.display {
            QemuDisplayMode::Graphical => "graphical",
            QemuDisplayMode::Nographic => "nographic",
        }
        .into(),
        QemuLaunchField::Serial => match dialog.draft.serial {
            QemuSerialMode::Stdio => "stdio",
            QemuSerialMode::Telnet => "telnet",
            QemuSerialMode::None => "none",
        }
        .into(),
        QemuLaunchField::ExtraArguments => {
            if dialog.draft.extra_arguments.is_empty() {
                "none".into()
            } else {
                dialog.draft.extra_arguments.clone()
            }
        }
    }
}

fn qemu_launch_dialog(frame: &mut Frame, app: &App, dialog: &QemuLaunchDialog, area: Rect) {
    let popup = qemu_popup_rect(area, 100, 20);
    clear_popup(frame, app, popup);
    let palette = ThemePalette::for_app(app);
    let fields = [
        QemuLaunchField::Machine,
        QemuLaunchField::Image,
        QemuLaunchField::Kernel,
        QemuLaunchField::Rootfs,
        QemuLaunchField::Networking,
        QemuLaunchField::Memory,
        QemuLaunchField::Display,
        QemuLaunchField::Serial,
        QemuLaunchField::ExtraArguments,
    ];
    let rows = fields.into_iter().map(|field| {
        let selected = dialog.selected_field == field;
        let suffix = if field.is_read_only() {
            " [read-only]"
        } else if selected && dialog.editing {
            " [editing]"
        } else {
            ""
        };
        Row::new([
            format!("{}{}", qemu_launch_field_label(field), suffix),
            qemu_launch_field_value(dialog, field),
        ])
        .style(selected_style(app, selected))
    });
    let mut title =
        String::from("Launch runqemu | ↑/↓ field ←/→ choice Enter edit p preview Esc close");
    if popup.width < 80 {
        title = "Launch runqemu | p preview | Esc close".into();
    }
    frame.render_widget(
        Table::new(rows, [Constraint::Length(23), Constraint::Min(1)])
            .header(Row::new(["Field", "Value"]).style(Style::default().bold()))
            .block(Block::default().title(title).borders(Borders::ALL)),
        popup,
    );
    if let Some(message) = &dialog.validation_error {
        let error = Rect::new(
            popup.x.saturating_add(1),
            popup.y + popup.height.saturating_sub(3),
            popup.width.saturating_sub(2),
            2.min(popup.height.saturating_sub(1)),
        );
        frame.render_widget(
            Paragraph::new(format!("Validation: {message}"))
                .style(palette.role(palette.error, Modifier::BOLD))
                .wrap(Wrap { trim: false }),
            error,
        );
    }
}

fn qemu_launch_confirmation(frame: &mut Frame, app: &App, preview: &QemuLaunchPreview, area: Rect) {
    let popup = qemu_popup_rect(area, 100, 18);
    clear_popup(frame, app, popup);
    let mut lines = vec![
        Line::from(format!("Machine: {}", preview.request.machine)),
        Line::from(format!("Image: {}", preview.request.image.image)),
        Line::from(format!(
            "Artifact: {}",
            preview.request.image.path.display()
        )),
        Line::from("Exact argument vector (one argument per line):"),
    ];
    lines.extend(
        preview
            .argv
            .iter()
            .enumerate()
            .map(|(index, argument)| Line::from(format!("[{index}] {}", argument.display()))),
    );
    lines.push(Line::from(""));
    lines.push(Line::from(
        "Enter confirms launch. Esc closes without launch.",
    ));
    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .title("Confirm managed runqemu launch")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false }),
        popup,
    );
}

fn qemu_cancellation_confirmation(frame: &mut Frame, app: &App, id: QemuSessionId, area: Rect) {
    let popup = qemu_popup_rect(area, 72, 7);
    clear_popup(frame, app, popup);
    let detail = app.qemu_session(id).map_or_else(
        || format!("Session {} is no longer available.", id.0),
        |session| {
            format!(
                "Cancel managed session {}?\nImage: {}\nArtifact: {}",
                id.0,
                session.request.image.image,
                session.request.image.path.display()
            )
        },
    );
    frame.render_widget(
        Paragraph::new(format!(
            "{detail}\n\nEnter confirms cancellation. Esc keeps it running."
        ))
        .block(
            Block::default()
                .title("Confirm runqemu cancellation")
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: false }),
        popup,
    );
}

fn images_workspace(frame: &mut Frame, app: &App, area: Rect) {
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
    let mut lines = vec![Line::from(format!(
        "MACHINE {machine} | build target {} | {recipe_count} image recipe target(s)",
        app.build.target.as_deref().unwrap_or("not selected")
    ))];
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
    if app.image_artifact_searching {
        lines.push(Line::from(format!("Search: {}_", app.image_artifact_query)));
    } else if !app.image_artifact_query.is_empty() {
        lines.push(Line::from(format!("Search: {}", app.image_artifact_query)));
    }
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
            let artifacts = app.filtered_image_artifacts();
            if artifacts.is_empty() {
                lines.push(Line::from("No artifacts match the active search."));
            }
            for artifact in artifacts {
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

fn image_artifact_inspector_text(app: &App) -> String {
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
            format!(
                "Machine: {}\nImage: {}\nKind: {}\nPath: {}\nDeploy directory: {}\nSize: {}\nTimestamp: {}\n\nChecksums:\n{}\n\nManifests:\n{}\n\nLicenses:\n{}\n\nSPDX/SBOM:\n{}\n\nWic files:\n{}\n\nLimitations:\n{}",
                artifact.identity.machine,
                artifact.identity.image,
                artifact.kind.label(),
                artifact.identity.path.display(),
                deploy,
                artifact
                    .size_bytes
                    .available()
                    .map_or_else(|| "unavailable".into(), |value| format!("{value} bytes")),
                artifact.modified_unix_seconds.available().map_or_else(
                    || "unavailable".into(),
                    |value| format!("{value}s since Unix epoch")
                ),
                checksums,
                paths(&artifact.manifests),
                paths(&artifact.licenses),
                paths(&artifact.spdx),
                paths(&artifact.wic_files),
                limitations,
            )
        },
    );
    format!(
        "runqemu capability\n{}\nLaunch: {}\n\n{}\n\nSelected artifact\n{artifact_text}",
        qemu_capability_text(app),
        app.qemu_launch_unavailable_reason()
            .unwrap_or_else(|| "ready for selected artifact (Q)".into()),
        qemu_session_text(app)
    )
}

fn qemu_capability_text(app: &App) -> String {
    match &app.qemu_capability {
        QemuCapability::NotInspected => "not inspected".into(),
        QemuCapability::MissingTool => "missing runqemu executable".into(),
        QemuCapability::MissingCompatibleImage => "no compatible deployed image".into(),
        QemuCapability::Failed { message } => format!("inspection failed: {message}"),
        QemuCapability::Available {
            executable,
            compatible_images,
        } => format!(
            "available: {}\nCompatible images: {}",
            executable.display(),
            compatible_images.len()
        ),
    }
}

fn qemu_session_text(app: &App) -> String {
    let Some(session) = app.latest_qemu_session() else {
        return "Managed runqemu session\nNo session has been launched.".into();
    };
    let Some(job) = app.background_jobs.get(session.background_job_id) else {
        return format!(
            "Managed runqemu session {}\nLifecycle record unavailable.",
            session.id.0
        );
    };
    let status = match job.status {
        BackgroundJobStatus::Queued => "queued",
        BackgroundJobStatus::Starting => "starting",
        BackgroundJobStatus::Running => "running",
        BackgroundJobStatus::Cancelling => "cancelling",
        BackgroundJobStatus::Succeeded => "succeeded",
        BackgroundJobStatus::Failed => "failed",
        BackgroundJobStatus::Cancelled => "cancelled",
        BackgroundJobStatus::Lost => "lost",
    };
    let mut retained = job.output.iter().rev().take(80).collect::<Vec<_>>();
    retained.reverse();
    let output = if retained.is_empty() {
        "none".into()
    } else {
        retained
            .into_iter()
            .map(|entry| {
                let source = match entry.source {
                    yoctui_model::BackgroundJobOutputSource::Backend => "backend",
                    yoctui_model::BackgroundJobOutputSource::Stdout => "stdout",
                    yoctui_model::BackgroundJobOutputSource::Stderr => "stderr",
                };
                format!(
                    "[{source}] {}{}",
                    entry.message,
                    if entry.truncated { " [truncated]" } else { "" }
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let result = job
        .result
        .as_ref()
        .map_or_else(|| "none".into(), |result| result.summary.clone());
    let error = job.error.as_ref().map_or_else(
        || session.error_detail.as_deref().unwrap_or("none").into(),
        |error| {
            error.detail.as_ref().map_or_else(
                || error.summary.clone(),
                |detail| format!("{}: {detail}", error.summary),
            )
        },
    );
    format!(
        "Managed runqemu session {}\nStatus: {}\nMachine: {}\nImage: {}\nArtifact: {}\nNetworking: {:?}\nDisplay: {:?}\nSerial: {:?}\nMemory: {} MiB\nQueued: {}\nStarted: {}\nFinished: {}\nExit code: {}\nResult: {}\nError: {}\nRetained output: {} entries (showing latest {})\nDropped output: {} entries\n\nOutput:\n{}",
        session.id.0,
        status,
        session.request.machine,
        session.request.image.image,
        session.request.image.path.display(),
        session.request.networking,
        session.request.display,
        session.request.serial,
        session.request.memory_mib,
        timestamp_text(job.queued_at),
        job.started_at
            .map(timestamp_text)
            .unwrap_or_else(|| "unavailable".into()),
        job.finished_at
            .map(timestamp_text)
            .unwrap_or_else(|| "unavailable".into()),
        session
            .exit_code
            .map_or_else(|| "unavailable".into(), |code| code.to_string()),
        result,
        error,
        job.output.len(),
        job.output.len().min(80),
        job.dropped_output_entries,
        output,
    )
}

fn settings_workspace(frame: &mut Frame, app: &App, area: Rect) {
    let rows = [
        ("Theme", format!("{:?}", app.theme)),
        ("Animation speed", format!("{:?}", app.animation_speed)),
        ("Reduced motion", app.reduced_motion.to_string()),
        ("Color", app.color_enabled.to_string()),
        ("Log wrap", app.logs.wrap.to_string()),
        ("Log follow", app.logs.follow.to_string()),
    ];
    let chunks = Layout::vertical([Constraint::Min(8), Constraint::Length(4)]).split(area);
    frame.render_widget(
        Table::new(
            rows.into_iter().enumerate().map(|(index, (name, value))| {
                Row::new([name.to_owned(), value])
                    .style(selected_style(app, index == app.settings_selection))
            }),
            [Constraint::Percentage(55), Constraint::Percentage(45)],
        )
        .header(
            Row::new(["Setting", "Active value"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(
            Block::default()
                .title(if app.settings_dirty {
                    "Settings (not saved)"
                } else {
                    "Settings"
                })
                .borders(Borders::ALL),
        ),
        chunks[0],
    );
    frame.render_widget(
        Paragraph::new(
            "↑/↓ or j/k select  ←/→ or Enter change  r retry unsaved changes\nChanges apply immediately and are saved to Yoctui's session preferences.\nCLI flags override session values; session values override config.toml defaults.",
        )
        .block(
            Block::default()
                .title("Settings controls")
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: true }),
        chunks[1],
    );
}

fn build_history(frame: &mut Frame, app: &App, area: Rect) {
    let records = app.build_history.iter().rev().collect::<Vec<_>>();
    let selected = records.get(app.build_history_selection).copied();
    let chunks = Layout::vertical([Constraint::Min(4), Constraint::Length(7)]).split(area);
    frame.render_widget(
        Table::new(
            records.iter().enumerate().map(|(index, record)| {
                Row::new(vec![
                    Cell::from(record.target.as_deref().unwrap_or("unknown")),
                    Cell::from(if record.success { "success" } else { "failed" }),
                    Cell::from(
                        record
                            .exit_code
                            .map_or_else(|| "--".into(), |code| code.to_string()),
                    ),
                    Cell::from(
                        record
                            .elapsed
                            .map_or_else(|| "--:--:--".into(), format_duration),
                    ),
                    Cell::from(record.completed_tasks.to_string()),
                ])
                .style(selected_style(app, index == app.build_history_selection))
            }),
            [
                Constraint::Percentage(35),
                Constraint::Percentage(16),
                Constraint::Percentage(12),
                Constraint::Percentage(18),
                Constraint::Percentage(19),
            ],
        )
        .header(
            Row::new(["Target", "Result", "Exit", "Elapsed", "Tasks"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(
            Block::default()
                .title(format!(
                    "Build history ({} retained; newest first)",
                    records.len()
                ))
                .borders(Borders::ALL),
        ),
        chunks[0],
    );
    let detail = selected.map_or_else(
        || "No completed builds are retained in this session.".into(),
        |record| {
            format!(
                "Target: {}\nResult: {}\nWarnings: {}  Errors: {}\nCompleted package tasks: {}",
                record.target.as_deref().unwrap_or("unknown"),
                if record.success { "success" } else { "failed" },
                record.warnings,
                record.errors,
                record.completed_tasks,
            )
        },
    );
    frame.render_widget(
        Paragraph::new(detail).block(
            Block::default()
                .title("Selected build")
                .borders(Borders::ALL),
        ),
        chunks[1],
    );
}

fn dependency_identity_text(identity: &DependencyNodeId) -> String {
    match identity {
        DependencyNodeId::Recipe(recipe) => recipe.clone(),
        DependencyNodeId::Task { recipe, task } => format!("{recipe}:{task}"),
    }
}

fn dependency_kind_text(identity: &DependencyNodeId) -> &'static str {
    match identity {
        DependencyNodeId::Recipe(_) => "recipe",
        DependencyNodeId::Task { .. } => "task",
    }
}

fn dependency_edge_kind_text(kind: DependencyEdgeKind) -> &'static str {
    match kind {
        DependencyEdgeKind::Build => "build",
        DependencyEdgeKind::Runtime => "runtime",
        DependencyEdgeKind::Task => "task",
    }
}

fn dependency_edge_context(
    graph: &DependencyGraph,
    selected: &DependencyNodeId,
    incoming: bool,
) -> String {
    let edges = if incoming {
        graph.incoming(selected)
    } else {
        graph.outgoing(selected)
    };
    if edges.is_empty() {
        return "none reported".into();
    }
    let total = edges.len();
    let mut values = edges
        .into_iter()
        .take(8)
        .map(|edge| {
            let identity = if incoming { &edge.from } else { &edge.to };
            format!(
                "{}: {}",
                dependency_edge_kind_text(edge.kind),
                dependency_identity_text(identity)
            )
        })
        .collect::<Vec<_>>();
    if total > values.len() {
        values.push(format!("… {} more", total - values.len()));
    }
    values.join("\n")
}

fn dependency_why_built(graph: &DependencyGraph, selected: &DependencyNodeId) -> String {
    match graph.why_built(selected, 64, 4_096) {
        DependencyPathResult::Found(path) if path.len() == 1 => "root selected".into(),
        DependencyPathResult::Found(path) => {
            let mut text = dependency_identity_text(&path[0]);
            for pair in path.windows(2) {
                let kind = graph
                    .edges
                    .iter()
                    .find(|edge| edge.from == pair[0] && edge.to == pair[1])
                    .map_or("unknown", |edge| dependency_edge_kind_text(edge.kind));
                text.push_str(&format!(
                    "\n  --{kind}--> {}",
                    dependency_identity_text(&pair[1])
                ));
            }
            text
        }
        DependencyPathResult::Unreachable => "unreachable from root".into(),
        DependencyPathResult::LimitReached => "path limit reached".into(),
    }
}

fn dependency_inspector(app: &App) -> String {
    match &app.dependency_graph {
        DependencyGraphState::NotLoaded => {
            "Dependency graph: not loaded\n\nSelect a recipe in Recipes and press g.".into()
        }
        DependencyGraphState::Loading { root } => format!(
            "Dependency graph: loading\nRoot: {}\n\nNo stale graph is shown while the authoritative query runs.",
            dependency_identity_text(root)
        ),
        DependencyGraphState::AvailableEmpty { root } => format!(
            "Root: {}\nState: available-empty\n\nNo dependency edges reported.",
            dependency_identity_text(root)
        ),
        DependencyGraphState::Failed { root, message } => format!(
            "Root: {}\nState: failed\n\n{message}\n\nNo stale graph is presented as current.",
            dependency_identity_text(root)
        ),
        DependencyGraphState::Available(graph) | DependencyGraphState::Partial { graph, .. } => {
            let selected = app
                .dependency_graph_selection
                .as_ref()
                .and_then(|identity| graph.nodes.iter().find(|node| &node.id == identity));
            let Some(node) = selected else {
                return format!(
                    "Root: {}\n\nNo dependency node is selected.",
                    dependency_identity_text(&graph.root)
                );
            };
            let limitations = match &app.dependency_graph {
                DependencyGraphState::Partial { limitations, .. } if !limitations.is_empty() => {
                    limitations
                        .iter()
                        .map(|value| format!("- {value}"))
                        .collect::<Vec<_>>()
                        .join("\n")
                }
                _ => "none".into(),
            };
            format!(
                "Root: {}\nSelected: {} ({})\nProvider: {}\nTask log: {}\n\nReverse / incoming:\n{}\n\nDependencies / outgoing:\n{}\n\nWhy built:\n{}\n\nLimitations:\n{}",
                dependency_identity_text(&graph.root),
                dependency_identity_text(&node.id),
                dependency_kind_text(&node.id),
                node.provider
                    .as_ref()
                    .map_or_else(|| "unavailable".into(), |path| path.display().to_string()),
                node.log
                    .as_ref()
                    .map_or_else(|| "unavailable".into(), |path| path.display().to_string()),
                dependency_edge_context(graph, &node.id, true),
                dependency_edge_context(graph, &node.id, false),
                dependency_why_built(graph, &node.id),
                limitations,
            )
        }
    }
}

fn dependencies(frame: &mut Frame, app: &App, area: Rect) {
    let (graph, partial) = match &app.dependency_graph {
        DependencyGraphState::NotLoaded => {
            frame.render_widget(
                Paragraph::new(
                    "Dependency graph is not loaded.\n\nSelect a recipe in Recipes and press g.",
                )
                .block(
                    Block::default()
                        .title("Dependency graph · not loaded")
                        .borders(Borders::ALL),
                )
                .wrap(Wrap { trim: false }),
                area,
            );
            return;
        }
        DependencyGraphState::Loading { root } => {
            frame.render_widget(
                Paragraph::new(format!(
                    "Loading authoritative dependency graph for {}…\n\nStale rows are hidden.",
                    dependency_identity_text(root)
                ))
                .block(
                    Block::default()
                        .title("Dependency graph · loading")
                        .borders(Borders::ALL),
                )
                .wrap(Wrap { trim: false }),
                area,
            );
            return;
        }
        DependencyGraphState::AvailableEmpty { root } => {
            frame.render_widget(
                Paragraph::new(format!(
                    "Root: {}\n\nNo dependency edges reported.",
                    dependency_identity_text(root)
                ))
                .block(
                    Block::default()
                        .title("Dependency graph · available-empty")
                        .borders(Borders::ALL),
                )
                .wrap(Wrap { trim: false }),
                area,
            );
            return;
        }
        DependencyGraphState::Failed { root, message } => {
            frame.render_widget(
                Paragraph::new(format!(
                    "Root: {}\n\n{message}\n\nNo stale graph is presented as current.",
                    dependency_identity_text(root)
                ))
                .block(
                    Block::default()
                        .title("Dependency graph · failed")
                        .borders(Borders::ALL),
                )
                .wrap(Wrap { trim: false }),
                area,
            );
            return;
        }
        DependencyGraphState::Available(graph) => (graph, false),
        DependencyGraphState::Partial { graph, .. } => (graph, true),
    };

    let mut counts: HashMap<&DependencyNodeId, (usize, usize)> = HashMap::new();
    for edge in &graph.edges {
        counts.entry(&edge.from).or_default().1 += 1;
        counts.entry(&edge.to).or_default().0 += 1;
    }
    let selected_index = app
        .dependency_graph_selection
        .as_ref()
        .and_then(|selected| graph.nodes.iter().position(|node| &node.id == selected))
        .unwrap_or(0);
    let capacity = area.height.saturating_sub(3).max(1) as usize;
    let start = selected_index.saturating_add(1).saturating_sub(capacity);
    let end = graph.nodes.len().min(start.saturating_add(capacity));
    let rows = graph.nodes[start..end].iter().map(|node| {
        let (incoming, outgoing) = counts.get(&node.id).copied().unwrap_or_default();
        Row::new(vec![
            Cell::from(dependency_kind_text(&node.id)),
            Cell::from(dependency_identity_text(&node.id)),
            Cell::from(incoming.to_string()),
            Cell::from(outgoing.to_string()),
        ])
        .style(selected_style(
            app,
            app.dependency_graph_selection.as_ref() == Some(&node.id),
        ))
    });
    frame.render_widget(
        Table::new(
            rows,
            [
                Constraint::Length(8),
                Constraint::Min(18),
                Constraint::Length(4),
                Constraint::Length(4),
            ],
        )
        .header(
            Row::new(["Kind", "Identity", "In", "Out"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(
            Block::default()
                .title(format!(
                    "Dependency graph: {} · {} nodes · {} edges{}",
                    dependency_identity_text(&graph.root),
                    graph.nodes.len(),
                    graph.edges.len(),
                    if partial { " · partial" } else { "" }
                ))
                .borders(Borders::ALL),
        ),
        area,
    );
}

fn layer_relationships(frame: &mut Frame, app: &App, area: Rect) {
    let text = app.layer_relationships.as_ref().map_or_else(
        || "No layer relationship data is loaded. Open Layers and press i.".into(),
        |relationships| relationships.layers.iter().map(|layer| format!(
            "{} (priority: {})\n  compatible: {}\n  depends: {}\n  overlays: {}\n  appends: {}",
            layer.name, layer.priority.map_or_else(|| "unknown".into(), |value| value.to_string()),
            list_or_none(&layer.compatible), list_or_none(&layer.depends), list_or_none(&layer.overlays), list_or_none(&layer.appends)
        )).collect::<Vec<_>>().join("\n\n"),
    );
    frame.render_widget(
        Paragraph::new(text)
            .block(
                Block::default()
                    .title("Layer relationships (server supplied)")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn list_or_none(values: &[String]) -> String {
    if values.is_empty() {
        "(none)".into()
    } else {
        values.join(", ")
    }
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KiB", "MiB", "GiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} {}", UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}
fn logs(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::vertical([Constraint::Length(6), Constraint::Min(3)]).split(area);
    let log_area = chunks[1];
    let all_visible = app
        .logs
        .filtered()
        .filter(|l| {
            app.screen != Screen::Errors
                || matches!(l.severity, Severity::Warning | Severity::Error)
        })
        .collect::<Vec<_>>();
    let height = log_area.height.saturating_sub(3) as usize;
    let selection = app.logs.selection.min(all_visible.len().saturating_sub(1));
    let end = selection
        .saturating_add(1)
        .max(height)
        .min(all_visible.len());
    let start = end.saturating_sub(height);
    let visible = &all_visible[start..end];
    let mode = format!(
        "{} | {} | {}",
        if app.logs.follow {
            "following"
        } else {
            "paused"
        },
        if app.logs.wrap {
            "wrapped"
        } else {
            "unwrapped"
        },
        app.logs
            .filter
            .map_or_else(|| "all".into(), |severity| format!("{severity:?}"))
            + &format!(
                " | recipe: {} | task: {}",
                app.logs.recipe_filter.as_deref().unwrap_or("all"),
                app.logs.task_filter.as_deref().unwrap_or("all")
            )
            + &format!(
                " | build: {}",
                app.logs.build_filter.as_deref().unwrap_or("all")
            )
    );
    let pressure = if app.logs.dropped > 0 || app.logs.coalesced > 0 {
        format!(
            "{} evicted [W {} E {}], {} coalesced; retained {}/{} bytes",
            app.logs.dropped,
            app.logs.dropped_warnings,
            app.logs.dropped_errors,
            app.logs.coalesced,
            app.logs.retained_bytes,
            app.logs.max_bytes
        )
    } else {
        format!(
            "No eviction or coalescing; retained {}/{} bytes",
            app.logs.retained_bytes, app.logs.max_bytes
        )
    };
    let search = if app.logs.searching {
        format!("search: {}_", app.logs.query)
    } else if app.logs.query.is_empty() {
        "search: none".into()
    } else {
        format!("search: {}", app.logs.query)
    };
    let search = app
        .logs
        .match_position()
        .map_or(search.clone(), |(current, count)| {
            format!("{search} | selected {current}/{count}")
        });
    frame.render_widget(
        Paragraph::new(format!("{mode}\n{pressure}\n{search}"))
            .block(Block::default().title("Log status").borders(Borders::ALL))
            .wrap(Wrap { trim: false }),
        chunks[0],
    );
    if app.logs.wrap {
        let mut lines = Vec::new();
        for (offset, log) in visible.iter().enumerate() {
            let selected = start + offset == selection;
            for (line_index, message) in log.message.lines().enumerate() {
                let prefix = if line_index == 0 {
                    format!(
                        "{} {:?} {} ",
                        if selected { "▶" } else { " " },
                        log.severity,
                        log.recipe.as_deref().unwrap_or("")
                    )
                } else {
                    "    ".into()
                };
                let style = if selected {
                    selected_log_style(app, log.severity)
                } else {
                    severity_style(app, log.severity)
                };
                lines.push(Line::styled(format!("{prefix}{message}"), style));
            }
        }
        let text = Text::from(lines);
        frame.render_widget(
            Paragraph::new(text)
                .block(Block::default().title("Logs").borders(Borders::ALL))
                .wrap(Wrap { trim: false }),
            log_area,
        );
        return;
    }
    let rows = visible.iter().enumerate().map(|(offset, l)| {
        let selected = start + offset == selection;
        Row::new(vec![
            Cell::from(format!("{:?}", l.severity)),
            Cell::from(l.recipe.as_deref().unwrap_or("")),
            Cell::from(l.task.as_deref().unwrap_or("")),
            Cell::from(
                l.message
                    .chars()
                    .skip(app.logs.horizontal_offset)
                    .collect::<String>(),
            ),
        ])
        .style(if selected {
            selected_log_style(app, l.severity)
        } else {
            severity_style(app, l.severity)
        })
    });
    frame.render_widget(
        Table::new(
            rows,
            [
                Constraint::Length(9),
                Constraint::Length(16),
                Constraint::Length(18),
                Constraint::Min(10),
            ],
        )
        .header(
            Row::new(["Level", "Recipe", "Task", "Message"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(Block::default().title("Logs").borders(Borders::ALL)),
        log_area,
    )
}
fn errors(frame: &mut Frame, app: &App, area: Rect) {
    let errors = app.logs.diagnostics().collect::<Vec<_>>();
    let selected = errors.get(app.error_selection).copied();
    let chunks = Layout::vertical([Constraint::Min(4), Constraint::Length(12)]).split(area);
    let height = chunks[0].height.saturating_sub(3) as usize;
    let selection = app.error_selection.min(errors.len().saturating_sub(1));
    let end = selection.saturating_add(1).max(height).min(errors.len());
    let start = end.saturating_sub(height);
    let rows = errors[start..end].iter().enumerate().map(|(offset, log)| {
        let index = start + offset;
        let diagnostic = log.diagnostic.as_ref();
        Row::new(vec![
            Cell::from(timestamp_text(log.timestamp)),
            Cell::from(format!("{:?}", log.severity)),
            Cell::from(log.recipe.as_deref().unwrap_or("")),
            Cell::from(log.task.as_deref().unwrap_or("")),
            Cell::from(diagnostic.map_or("", |value| value.summary.as_str())),
            Cell::from(log.build.as_deref().unwrap_or("")),
        ])
        .style(if index == selection {
            selected_log_style(app, log.severity)
        } else {
            severity_style(app, log.severity)
        })
    });
    frame.render_widget(
        Table::new(
            rows,
            [
                Constraint::Length(14),
                Constraint::Length(9),
                Constraint::Length(14),
                Constraint::Length(16),
                Constraint::Min(18),
                Constraint::Length(20),
            ],
        )
        .header(
            Row::new(["Time", "Severity", "Recipe", "Task", "Summary", "Build"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(
            Block::default()
                .title(format!(
                    "Errors and warnings ({} retained; {} warning / {} error records evicted)",
                    errors.len(),
                    app.logs.dropped_warnings,
                    app.logs.dropped_errors,
                ))
                .borders(Borders::ALL),
        ),
        chunks[0],
    );
    let detail = selected.map_or_else(
        || "No retained warnings or errors.".into(),
        |log| diagnostic_detail(app, log),
    );
    frame.render_widget(
        Paragraph::new(format!(
            "{detail}\n\nEnter jumps to matching logs.  o opens the selected source log."
        ))
        .block(
            Block::default()
                .title("Selected diagnostic")
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: false }),
        chunks[1],
    );
}

fn diagnostic_detail(app: &App, log: &yoctui_model::LogEntry) -> String {
    let diagnostic = log.diagnostic.as_ref();
    let metadata = diagnostic.map_or_else(
        || "unavailable".into(),
        |value| {
            value
                .event_metadata
                .iter()
                .map(|(name, value)| format!("{name}={value}"))
                .collect::<Vec<_>>()
                .join(", ")
        },
    );
    let suggestions = diagnostic.map_or_else(
        || "No suggested actions are available.".into(),
        |value| value.suggestions.join("\n- "),
    );
    let related = app
        .logs
        .diagnostics()
        .filter(|candidate| {
            candidate.id != log.id
                && ((log.recipe.is_some() && candidate.recipe == log.recipe)
                    || (log.task.is_some() && candidate.task == log.task)
                    || (log.build.is_some() && candidate.build == log.build))
        })
        .filter_map(|candidate| candidate.diagnostic.as_ref())
        .map(|value| value.summary.as_str())
        .take(3)
        .collect::<Vec<_>>();
    format!(
        "Category: {}\nSummary: {}\nTime: {}\nBuild: {}\nRecipe: {}  Task: {}\nSource log: {}\nEvent metadata: {}\n\nFull message:\n{}\n\nSuggested actions:\n- {}\n\nRelated diagnostics:\n{}",
        diagnostic.map_or("unavailable", |value| value.category.as_str()),
        diagnostic.map_or("unavailable", |value| value.summary.as_str()),
        timestamp_text(log.timestamp),
        log.build.as_deref().unwrap_or("unavailable"),
        log.recipe.as_deref().unwrap_or("unavailable"),
        log.task.as_deref().unwrap_or("unavailable"),
        log.path
            .as_ref()
            .map_or_else(|| "unavailable".into(), |path| path.display().to_string()),
        metadata,
        log.message,
        suggestions,
        if related.is_empty() {
            "none".into()
        } else {
            related.join("\n")
        },
    )
}
fn recipe_build_state(app: &App, recipe: &str) -> String {
    if let Some(task) = app.tasks.values().find(|task| task.recipe == recipe) {
        return format!("{:?}", task.state).to_ascii_lowercase();
    }
    if let Some(task) = app
        .completed_tasks
        .iter()
        .find(|task| task.task.recipe == recipe)
    {
        return format!("{:?}", task.task.state).to_ascii_lowercase();
    }
    if app.build.target.as_deref() == Some(recipe) {
        return match app.build.status {
            BuildStatus::Idle | BuildStatus::LoadingWorkspace => "idle",
            BuildStatus::Parsing => "parsing",
            BuildStatus::Running => "running",
            BuildStatus::Cancelling => "cancelling",
            BuildStatus::Completed => "succeeded",
            BuildStatus::Cancelled => "cancelled",
            BuildStatus::Failed => "failed",
        }
        .into();
    }
    app.recipe_metadata
        .get(recipe)
        .and_then(|metadata| metadata.build_status)
        .map_or_else(
            || "unavailable".into(),
            |status| {
                match status {
                    RecipeBuildStatus::Idle => "idle",
                    RecipeBuildStatus::Queued => "queued",
                    RecipeBuildStatus::Running => "running",
                    RecipeBuildStatus::Succeeded => "succeeded",
                    RecipeBuildStatus::Failed => "failed",
                    RecipeBuildStatus::Cancelled => "cancelled",
                }
                .into()
            },
        )
}

fn recipe_workspace_state(app: &App, recipe: &Recipe) -> String {
    let identity = recipe.file.as_ref().and_then(|file| {
        file.is_absolute().then(|| RecipeIdentity {
            name: recipe.name.clone(),
            file: file.clone(),
        })
    });
    let Some(identity) = identity else {
        return "unavailable (absolute provider path not reported)".into();
    };
    if app.devtool_status_loading.contains(&identity) {
        return "loading authoritative status…".into();
    }
    let Some(status) = app.devtool_statuses.get(&identity) else {
        return "not inspected; press t (or Enter)".into();
    };
    if status.capability == DevtoolCapability::MissingExecutable {
        return "Devtool executable missing; all actions disabled".into();
    }
    if let Some(error) = &status.error {
        return match error {
            DevtoolStatusError::InvalidRecipeIdentity => {
                "invalid recipe identity; all actions disabled".into()
            }
            DevtoolStatusError::DevtoolFailed { exit_code, message } => format!(
                "status failed (exit {}): {message}; all actions disabled",
                exit_code.map_or_else(|| "unavailable".into(), |code| code.to_string())
            ),
            DevtoolStatusError::MalformedOutput { line } => {
                format!("malformed Devtool output: {line}; all actions disabled")
            }
        };
    }
    let state = match &status.workspace {
        DevtoolWorkspace::NotMember => "not in workspace".into(),
        DevtoolWorkspace::MissingDirectory { source_path } => {
            format!("workspace source missing: {}", source_path.display())
        }
        DevtoolWorkspace::Present {
            source_path,
            recipe_file,
        } => {
            let git = match &status.git {
                DevtoolGitState::Available {
                    branch,
                    head,
                    modified,
                    untracked,
                    conflicted,
                } => format!(
                    "Git branch {}, head {}, {}, modified {modified}, untracked {untracked}, conflicted {conflicted}",
                    branch.as_deref().unwrap_or("detached"),
                    head.as_deref().unwrap_or("initial"),
                    if modified + untracked + conflicted == 0 {
                        "clean"
                    } else {
                        "dirty"
                    }
                ),
                DevtoolGitState::MissingExecutable => "Git executable missing".into(),
                DevtoolGitState::NotRepository => "source is not a Git repository".into(),
                DevtoolGitState::Failed { exit_code, message } => format!(
                    "Git status failed (exit {}): {message}",
                    exit_code.map_or_else(|| "unavailable".into(), |code| code.to_string())
                ),
                DevtoolGitState::Malformed { message } => {
                    format!("malformed Git status: {message}")
                }
                DevtoolGitState::NotApplicable => "Git status not applicable".into(),
            };
            format!(
                "member at {}\nWorkspace recipe: {}\n{git}",
                source_path.display(),
                recipe_file
                    .as_ref()
                    .map_or_else(|| "unavailable".into(), |path| path.display().to_string())
            )
        }
    };
    format!("{state}\n{}", devtool_action_status(status))
}

fn devtool_action_status(status: &DevtoolStatus) -> String {
    [
        ("d modify/edit", DevtoolAction::ModifyOrEdit),
        ("u update", DevtoolAction::UpdateRecipe),
        ("F finish", DevtoolAction::Finish),
        ("P deploy", DevtoolAction::Deploy),
        ("D reset", DevtoolAction::Reset),
    ]
    .into_iter()
    .map(|(label, action)| {
        status.disabled_reason(action).map_or_else(
            || format!("{label}: enabled"),
            |reason| format!("{label}: disabled ({reason})"),
        )
    })
    .collect::<Vec<_>>()
    .join(" | ")
}

fn recipe_values(label: &str, values: Option<&Vec<String>>) -> String {
    let value = values.map_or_else(
        || "unavailable".into(),
        |values| {
            if values.is_empty() {
                "none".into()
            } else {
                values.join(", ")
            }
        },
    );
    format!("{label}: {value}")
}

fn recipe_paths(label: &str, values: Option<&Vec<std::path::PathBuf>>) -> String {
    let value = values.map_or_else(
        || "unavailable".into(),
        |values| {
            if values.is_empty() {
                "none".into()
            } else {
                values
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join("\n  ")
            }
        },
    );
    format!("{label}: {value}")
}

fn recipe_inspector(app: &App, recipe: &Recipe) -> String {
    let load_state = if app.recipe_metadata_loading.contains(&recipe.name) {
        "loading selected recipe metadata…".into()
    } else if let Some(error) = app.recipe_metadata_errors.get(&recipe.name) {
        format!("metadata unavailable: {error}")
    } else if app.recipe_metadata.contains_key(&recipe.name) {
        "metadata loaded".into()
    } else {
        "not loaded; press Enter to inspect".into()
    };
    let metadata = app.recipe_metadata.get(&recipe.name);
    let dependencies = app
        .dependencies
        .as_ref()
        .filter(|dependencies| dependencies.recipe == recipe.name);
    let active_tasks = app
        .tasks
        .values()
        .filter(|task| task.recipe == recipe.name)
        .map(|task| format!("{} ({:?})", task.task, task.state))
        .collect::<Vec<_>>();
    let reported_tasks = metadata.and_then(|metadata| metadata.tasks.as_ref());
    let standard_tasks = [
        "clean",
        "cleansstate",
        "devshell",
        "menuconfig",
        "diffconfig",
        "diffsigs",
    ];
    let (enabled, disabled): (Vec<_>, Vec<_>) = standard_tasks.into_iter().partition(|task| {
        reported_tasks.is_some_and(|tasks| {
            let canonical = format!("do_{task}");
            tasks
                .iter()
                .any(|candidate| candidate == task || candidate == &canonical)
        })
    });
    let task_capabilities = if reported_tasks.is_none() {
        "Task actions unavailable until metadata is loaded.".into()
    } else {
        format!(
            "Task actions enabled: {}\nTask actions unavailable: {}",
            if enabled.is_empty() {
                "none".into()
            } else {
                enabled.join(", ")
            },
            if disabled.is_empty() {
                "none".into()
            } else {
                disabled.join(", ")
            }
        )
    };
    let retained_log_count = app
        .tasks
        .values()
        .chain(app.completed_tasks.iter().map(|completed| &completed.task))
        .filter(|task| task.recipe == recipe.name && task.log_path.is_some())
        .count();
    let patch_capability = match metadata.and_then(|value| value.patches.as_ref()) {
        None => "unavailable until metadata is loaded".into(),
        Some(patches) if patches.is_empty() => "unavailable (BitBake reported none)".into(),
        Some(patches) => {
            let local = patches
                .iter()
                .filter(|patch| std::path::Path::new(patch).is_absolute())
                .count();
            if local == 0 {
                "unavailable (remote or unresolved paths)".into()
            } else {
                format!("enabled ({local} local)")
            }
        }
    };
    let navigation_capabilities = format!(
        "Navigation: provider {}; task logs {}; patches {patch_capability}\nDevtool availability is authoritative above; press t to refresh.",
        if recipe.file.is_some() {
            "enabled"
        } else {
            "unavailable (provider path not reported)"
        },
        if retained_log_count == 0 {
            "unavailable (no retained path)".into()
        } else {
            format!("enabled ({retained_log_count})")
        },
    );
    let supports_task = |task: &str| {
        reported_tasks.is_some_and(|tasks| {
            let canonical = format!("do_{task}");
            tasks
                .iter()
                .any(|candidate| candidate == task || candidate == &canonical)
        })
    };
    let qa_capabilities = if reported_tasks.is_none() {
        "QA actions unavailable until authoritative task metadata is loaded.".into()
    } else {
        format!(
            "QA actions: CVE check {}; SPDX generation {}.",
            if supports_task("cve_check") {
                "enabled"
            } else {
                "unavailable (do_cve_check not reported)"
            },
            if supports_task("create_spdx") {
                "enabled"
            } else {
                "unavailable (do_create_spdx not reported)"
            }
        )
    };
    let latest_qa_job = app.background_jobs.jobs.iter().rev().find(|job| {
        job.context.recipe.as_deref() == Some(recipe.name.as_str())
            && matches!(
                job.kind,
                BackgroundJobKind::CveCheck | BackgroundJobKind::Spdx
            )
    });
    let qa_job = latest_qa_job.map_or_else(
        || "Latest QA job: not run.".into(),
        |job| {
            let artifacts = if job
                .result
                .as_ref()
                .is_some_and(|result| !result.artifacts.is_empty())
            {
                job.result
                    .as_ref()
                    .unwrap()
                    .artifacts
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            } else {
                "none reported".into()
            };
            let detail = job
                .error
                .as_ref()
                .map(|error| error.summary.as_str())
                .or_else(|| job.result.as_ref().map(|result| result.summary.as_str()))
                .or_else(|| job.output.back().map(|entry| entry.message.as_str()))
                .unwrap_or("no retained result yet");
            format!(
                "Latest QA job: {} [{:?}], progress {:?}, warnings {}, errors {}.\nQA result: {detail}; artifacts: {artifacts}.",
                job.title, job.status, job.progress, job.warnings, job.errors
            )
        },
    );
    let devtool_job = app
        .background_jobs
        .jobs
        .iter()
        .rev()
        .find(|job| {
            job.kind == BackgroundJobKind::Devtool
                && job.context.recipe.as_deref() == Some(recipe.name.as_str())
        })
        .map_or_else(
            || "Latest Devtool job: not run.".into(),
            |job| {
                let output = job.output.back().map_or_else(
                    || "no retained output".into(),
                    |entry| {
                        format!(
                            "{:?}: {}{}",
                            entry.source,
                            entry.message,
                            if entry.truncated { " [truncated]" } else { "" }
                        )
                    },
                );
                let outcome = job
                    .result
                    .as_ref()
                    .map(|result| result.summary.as_str())
                    .or_else(|| job.error.as_ref().map(|error| error.summary.as_str()))
                    .unwrap_or("in progress");
                format!(
                    "Latest Devtool job: {} [{:?}].\nDevtool output: {output}; outcome: {outcome}.",
                    job.title, job.status
                )
            },
        );
    let tasks = if active_tasks.is_empty() {
        recipe_values("Tasks", reported_tasks)
    } else {
        format!("Active tasks: {}", active_tasks.join(", "))
    };
    format!(
        "Recipe: {}\nResolved version: {}\nPreferred version: {}\nProvider layer: {}\nProvider file: {}\nAppends: {}\nWorkspace/Devtool: {}\nBuild: {}\nDetail: {load_state}\n{task_capabilities}\n{navigation_capabilities}\n{qa_capabilities}\n{qa_job}\n{devtool_job}\n\nDependencies: {}\nRuntime dependencies: {}\nReverse dependencies: unavailable\n{tasks}\n{}\n{}\n{}\n{}",
        recipe.name,
        recipe.version.as_deref().unwrap_or("unavailable"),
        recipe.preferred_version.as_deref().unwrap_or("unavailable"),
        recipe.layer.as_deref().unwrap_or("unavailable"),
        recipe
            .file
            .as_ref()
            .map_or_else(|| "unavailable".into(), |path| path.display().to_string()),
        recipe
            .append_count
            .map_or_else(|| "unavailable".into(), |count| count.to_string()),
        recipe_workspace_state(app, recipe),
        recipe_build_state(app, &recipe.name),
        dependencies.map_or_else(
            || "unavailable; press g to query".into(),
            |value| if value.build.is_empty() {
                "none".into()
            } else {
                value.build.join(", ")
            }
        ),
        dependencies.map_or_else(
            || "unavailable".into(),
            |value| if value.runtime.is_empty() {
                "none".into()
            } else {
                value.runtime.join(", ")
            }
        ),
        recipe_paths(
            "Metadata sources",
            metadata.and_then(|value| value.sources.as_ref())
        ),
        recipe_values("Patches", metadata.and_then(|value| value.patches.as_ref())),
        recipe_values(
            "Package outputs",
            metadata.and_then(|value| value.packages.as_ref())
        ),
        recipe_values("History", metadata.and_then(|value| value.history.as_ref())),
    )
}

fn packages_workspace(frame: &mut Frame, app: &App, area: Rect) {
    let palette = ThemePalette::for_app(app);
    let title = if app.package_searching {
        format!("Packages | search: {}_", app.package_query)
    } else if app.package_query.is_empty() {
        "Packages".into()
    } else {
        format!("Packages | search: {}", app.package_query)
    };
    let block = pane_block(app, &title, app.focus == FocusTarget::Workspace);
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
            let rows = app
                .filtered_packages()
                .into_iter()
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

fn package_field_text(field: &PackageField<String>) -> String {
    match field {
        PackageField::Available(value) if value.is_empty() => "empty".into(),
        PackageField::Available(value) => value.clone(),
        PackageField::Unavailable => "unavailable".into(),
    }
}

fn package_path_field_text(field: &PackageField<std::path::PathBuf>) -> String {
    match field {
        PackageField::Available(value) => value.display().to_string(),
        PackageField::Unavailable => "unavailable".into(),
    }
}

fn package_identity_list(
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

fn package_path_list(field: &PackageField<Vec<std::path::PathBuf>>) -> Vec<String> {
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

fn package_membership_text(field: &PackageField<Vec<String>>) -> String {
    match field {
        PackageField::Unavailable => "unavailable".into(),
        PackageField::Available(values) if values.is_empty() => "empty".into(),
        PackageField::Available(values) => values.join(", "),
    }
}

fn package_inspector_text(app: &App) -> String {
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

fn recipes(frame: &mut Frame, app: &App, area: Rect) {
    let recipes = app
        .workspace
        .recipes
        .iter()
        .enumerate()
        .filter(|(_, recipe)| {
            matches_metadata(
                &app.metadata_query,
                &[
                    recipe.name.as_str(),
                    recipe.version.as_deref().unwrap_or(""),
                    recipe.preferred_version.as_deref().unwrap_or(""),
                    recipe.layer.as_deref().unwrap_or(""),
                    recipe
                        .file
                        .as_ref()
                        .and_then(|path| path.to_str())
                        .unwrap_or(""),
                ],
            )
        })
        .collect::<Vec<_>>();
    let recipe_count = recipes.len();
    let selected = app.workspace.recipes.get(app.recipe_selection);
    let chunks = Layout::vertical([Constraint::Min(4), Constraint::Length(12)]).split(area);
    frame.render_widget(
        Table::new(
            recipes.into_iter().map(|(index, recipe)| {
                Row::new(vec![
                    Cell::from(recipe.name.as_str()),
                    Cell::from(recipe.version.as_deref().unwrap_or("?")),
                    Cell::from(recipe.preferred_version.as_deref().unwrap_or("?")),
                    Cell::from(recipe.layer.as_deref().unwrap_or("?")),
                    Cell::from(
                        recipe
                            .append_count
                            .map_or_else(|| "?".into(), |count| count.to_string()),
                    ),
                    Cell::from(recipe_workspace_state(app, recipe)),
                    Cell::from(recipe_build_state(app, &recipe.name)),
                ])
                .style(selected_style(app, index == app.recipe_selection))
            }),
            [
                Constraint::Min(12),
                Constraint::Length(10),
                Constraint::Length(10),
                Constraint::Length(10),
                Constraint::Length(4),
                Constraint::Length(11),
                Constraint::Length(10),
            ],
        )
        .header(
            Row::new([
                "Recipe",
                "Resolved",
                "Preferred",
                "Layer",
                "App",
                "Workspace",
                "Build",
            ])
            .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(
            Block::default()
                .title(metadata_title(
                    format!(
                        "Recipes (shown: {} of {})",
                        recipe_count,
                        app.workspace.recipes.len()
                    ),
                    app,
                ))
                .borders(Borders::ALL),
        ),
        chunks[0],
    );
    let detail = selected.map_or_else(
        || "No recipes supplied by the backend.".into(),
        |recipe| recipe_inspector(app, recipe),
    );
    frame.render_widget(
        Paragraph::new(detail)
            .block(
                Block::default()
                    .title("Selected recipe Inspector")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false }),
        chunks[1],
    );
}
fn git_state_text(state: GitFileState) -> &'static str {
    match state {
        GitFileState::Clean => "clean",
        GitFileState::Modified => "modified",
        GitFileState::Untracked => "untracked",
        GitFileState::Ignored => "ignored/generated",
        GitFileState::Unavailable => "Git unavailable",
    }
}

fn layer_relationship<'a>(
    app: &'a App,
    layer: &str,
) -> Option<&'a yoctui_model::LayerRelationship> {
    app.layer_relationships
        .as_ref()?
        .layers
        .iter()
        .find(|relationship| relationship.name == layer)
}

fn active_build_layer(app: &App, layer: &str) -> bool {
    app.build
        .target
        .as_ref()
        .and_then(|target| {
            app.workspace
                .recipes
                .iter()
                .find(|recipe| &recipe.name == target)
        })
        .and_then(|recipe| recipe.layer.as_deref())
        .is_some_and(|recipe_layer| recipe_layer == layer)
        || app.tasks.values().any(|task| {
            app.workspace
                .recipes
                .iter()
                .find(|recipe| recipe.name == task.recipe)
                .and_then(|recipe| recipe.layer.as_deref())
                .is_some_and(|recipe_layer| recipe_layer == layer)
        })
}

fn layer_entry_metadata(app: &App, browser: &LayerBrowser, entry: &LayerBrowserEntry) -> String {
    let relationship = layer_relationship(app, &browser.layer);
    let size = if entry.is_dir {
        "directory".into()
    } else {
        entry
            .size
            .map_or_else(|| "unavailable".into(), |size| format!("{size} bytes"))
    };
    let modified = entry
        .modified
        .map_or_else(|| "unavailable".into(), timestamp_text);
    let compatibility = relationship.map_or_else(
        || "unavailable".into(),
        |value| {
            if value.compatible.is_empty() {
                "not reported".into()
            } else {
                value.compatible.join(", ")
            }
        },
    );
    format!(
        "Path: {}\nType/size: {size}\nModified: {modified}\nGit: {}\nLayer: {}\nCompatibility: {compatibility}",
        entry.path.display(),
        git_state_text(entry.git),
        browser.layer
    )
}

fn layer_inspector_text(app: &App, browser: &LayerBrowser) -> Text<'static> {
    let Some(entry) = browser.selected_entry() else {
        return Text::from("This layer is empty.");
    };
    let metadata = layer_entry_metadata(app, browser, entry);
    let relationship = layer_relationship(app, &browser.layer);
    match browser.inspector_mode {
        LayerInspectorMode::Git => Text::from(format!(
            "{metadata}\n\nGit state: {}\nGit status is detected per loaded subtree; missing Git is non-fatal.",
            git_state_text(entry.git)
        )),
        LayerInspectorMode::Metadata => Text::from(metadata),
        LayerInspectorMode::Dependencies => Text::from(format!(
            "{metadata}\n\nDepends: {}\nOverlays: {}\nAppends: {}",
            relationship.map_or("unavailable".into(), |value| {
                if value.depends.is_empty() {
                    "none reported".into()
                } else {
                    value.depends.join(", ")
                }
            }),
            relationship.map_or("unavailable".into(), |value| {
                if value.overlays.is_empty() {
                    "none reported".into()
                } else {
                    value.overlays.join(", ")
                }
            }),
            relationship.map_or("unavailable".into(), |value| {
                if value.appends.is_empty() {
                    "none reported".into()
                } else {
                    value.appends.join(", ")
                }
            })
        )),
        LayerInspectorMode::Preview if entry.is_dir => Text::from(format!(
            "{metadata}\n\nDirectory contents are loaded only when expanded."
        )),
        LayerInspectorMode::Preview => match browser.preview_kind {
            PreviewKind::Binary => Text::from(format!(
                "{metadata}\n\nBinary preview unavailable.{}",
                if browser.preview_truncated {
                    "\nPreview exceeds the 64 KiB bound."
                } else {
                    ""
                }
            )),
            PreviewKind::Unavailable => Text::from(format!(
                "{metadata}\n\nPreview unavailable or still loading."
            )),
            PreviewKind::Text => {
                let file_name = entry
                    .path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("");
                let mut preview = numbered_source_preview(&browser.preview, file_name, app);
                if browser.preview_truncated {
                    preview.lines.insert(
                        0,
                        Line::from("[preview truncated at 64 KiB]").style(
                            ThemePalette::for_app(app)
                                .role(ThemePalette::for_app(app).warning, Modifier::BOLD),
                        ),
                    );
                }
                preview
            }
        },
    }
}

fn layer_browser(frame: &mut Frame, app: &App, browser: &LayerBrowser, area: Rect) {
    let chunks =
        Layout::horizontal([Constraint::Percentage(42), Constraint::Percentage(58)]).split(area);
    let layer_height = app.workspace.layers.len().saturating_add(2).min(8) as u16;
    let left =
        Layout::vertical([Constraint::Length(layer_height), Constraint::Min(3)]).split(chunks[0]);
    let palette = ThemePalette::for_app(app);
    let configured = app
        .workspace
        .layers
        .iter()
        .filter(|layer| {
            matches_metadata(
                &app.metadata_query,
                &[layer.name.as_str(), layer.path.to_str().unwrap_or("")],
            )
        })
        .map(|layer| {
            let relationship = layer_relationship(app, &layer.name);
            let compatibility = relationship.map_or("?", |value| {
                if value.compatible.is_empty() {
                    "-"
                } else {
                    "yes"
                }
            });
            let active = active_build_layer(app, &layer.name);
            let style = if layer.name == browser.layer {
                palette.selected()
            } else if active {
                palette.role(palette.success, Modifier::BOLD)
            } else {
                Style::default()
            };
            Row::new([
                if active {
                    format!("● {}", layer.name)
                } else {
                    format!("  {}", layer.name)
                },
                layer
                    .priority
                    .map_or_else(|| "?".into(), |priority| priority.to_string()),
                compatibility.into(),
            ])
            .style(style)
        });
    frame.render_widget(
        Table::new(
            configured,
            [
                Constraint::Min(8),
                Constraint::Length(4),
                Constraint::Length(6),
            ],
        )
        .header(Row::new(["Configured layers", "Pri", "Compat"]).style(Style::default().bold()))
        .block(Block::default().borders(Borders::ALL)),
        left[0],
    );

    let query = app.metadata_query.to_ascii_lowercase();
    let entries = browser.entries.iter().enumerate().filter(|(_, entry)| {
        query.is_empty()
            || entry
                .path
                .to_string_lossy()
                .to_ascii_lowercase()
                .contains(&query)
    });
    frame.render_widget(
        Table::new(
            entries.map(|(index, entry)| {
                let name = entry.path.file_name().map_or_else(
                    || entry.path.display().to_string(),
                    |name| name.to_string_lossy().into_owned(),
                );
                let marker = if entry.is_dir {
                    if browser.expanded.contains(&entry.path) {
                        "▾"
                    } else {
                        "▸"
                    }
                } else {
                    " "
                };
                let git = match entry.git {
                    GitFileState::Modified => " M",
                    GitFileState::Untracked => " ?",
                    GitFileState::Ignored => " I",
                    GitFileState::Clean => "  ",
                    GitFileState::Unavailable => " -",
                };
                let indent = "  ".repeat(entry.depth);
                Row::new([format!(
                    "{indent}{marker} {name}{}{git}",
                    if entry.is_dir { "/" } else { "" }
                )])
                .style(selected_style(app, index == browser.selection))
            }),
            [Constraint::Min(1)],
        )
        .block(
            Block::default()
                .title(metadata_title(
                    format!(
                        "{} tree | hidden {}",
                        browser.layer,
                        if browser.show_hidden { "on" } else { "off" }
                    ),
                    app,
                ))
                .borders(Borders::ALL),
        ),
        left[1],
    );

    let mode = match browser.inspector_mode {
        LayerInspectorMode::Preview => "Preview",
        LayerInspectorMode::Git => "Git",
        LayerInspectorMode::Metadata => "Metadata",
        LayerInspectorMode::Dependencies => "Dependencies",
    };
    frame.render_widget(
        Paragraph::new(layer_inspector_text(app, browser))
            .block(
                Block::default()
                    .title(format!("Inspector [{mode}]"))
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false }),
        chunks[1],
    );
}

fn layers(frame: &mut Frame, app: &App, area: Rect) {
    let layers = app
        .workspace
        .layers
        .iter()
        .filter(|layer| {
            matches_metadata(
                &app.metadata_query,
                &[layer.name.as_str(), layer.path.to_str().unwrap_or("")],
            )
        })
        .collect::<Vec<_>>();
    let selected = layers.get(app.layer_selection).copied();
    let recipes = selected.map_or_else(Vec::new, |layer| {
        let mut recipes = app
            .workspace
            .recipes
            .iter()
            .filter(|recipe| recipe.layer.as_deref() == Some(layer.name.as_str()))
            .collect::<Vec<_>>();
        recipes.sort_by(|left, right| left.name.cmp(&right.name));
        recipes
    });
    let chunks =
        Layout::horizontal([Constraint::Percentage(48), Constraint::Percentage(52)]).split(area);
    frame.render_widget(
        Table::new(
            layers.into_iter().enumerate().map(|(index, layer)| {
                Row::new(vec![
                    Cell::from(format!("▸ {}", layer.name)),
                    Cell::from(layer.path.display().to_string()),
                    Cell::from(
                        layer
                            .priority
                            .map_or_else(String::new, |priority| priority.to_string()),
                    ),
                ])
                .style({
                    let mut style = selected_style(app, index == app.layer_selection);
                    let palette = ThemePalette::for_app(app);
                    if index != app.layer_selection {
                        style = style.fg(palette.success);
                    }
                    if palette.attribute_only {
                        style = style.add_modifier(Modifier::BOLD);
                    }
                    style
                })
            }),
            [
                Constraint::Percentage(32),
                Constraint::Percentage(53),
                Constraint::Percentage(15),
            ],
        )
        .header(
            Row::new(["Layer", "Path", "Priority"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(
            Block::default()
                .title(metadata_title(
                    format!(
                        "Active layer tree (shown: {} of {})",
                        app.workspace
                            .layers
                            .iter()
                            .filter(|layer| matches_metadata(
                                &app.metadata_query,
                                &[layer.name.as_str(), layer.path.to_str().unwrap_or("")]
                            ))
                            .count(),
                        app.workspace.layers.len()
                    ),
                    app,
                ))
                .borders(Borders::ALL),
        ),
        chunks[0],
    );
    frame.render_widget(
        Table::new(
            recipes.iter().map(|recipe| {
                Row::new(vec![
                    Cell::from(recipe.name.as_str()),
                    Cell::from(recipe.version.as_deref().unwrap_or("")),
                ])
            }),
            [Constraint::Percentage(68), Constraint::Percentage(32)],
        )
        .header(
            Row::new(["Recipe in selected layer", "Version"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(
            Block::default()
                .title(selected.map_or_else(
                    || "Layer recipes".into(),
                    |layer| format!("Recipes: {} ({})", layer.name, recipes.len()),
                ))
                .borders(Borders::ALL),
        ),
        chunks[1],
    );
}
fn config_variables(app: &App) -> Vec<(&String, &String)> {
    let mut variables = app.workspace.variables.iter().collect::<Vec<_>>();
    variables.sort_by_key(|(name, _)| *name);
    variables.retain(|(name, value)| {
        matches_metadata(&app.metadata_query, &[name.as_str(), value.as_str()])
    });
    variables
}

fn config_inspector(app: &App) -> String {
    let variables = config_variables(app);
    let Some((name, summary_value)) = variables.get(app.config_selection).copied() else {
        let state = if app.workspace.variables.is_empty() {
            "No configuration variables supplied by the backend."
        } else {
            "No configuration variables match the active search."
        };
        return format!("{state}\n\n{}", config_copy_status(app));
    };
    let identity = VariableIdentity {
        name: name.clone(),
        recipe: app.config_scope.clone(),
    };
    if app.variable_detail_loading.contains(&identity) {
        return format!(
            "Variable: {name}\nEffective summary: {summary_value}\n\nLoading authoritative detail…\n\n{}",
            config_copy_status(app)
        );
    }
    if let Some(error) = app.variable_detail_errors.get(&identity) {
        return format!(
            "Variable: {name}\nEffective summary: {summary_value}\n\nDetail unavailable: {error}\nPress Enter to retry.\n\n{}",
            config_copy_status(app)
        );
    }
    let Some(detail) = app.variable_details.get(&identity) else {
        return format!(
            "Variable: {name}\nEffective summary: {summary_value}\nScope: {}\n\nDetail not loaded; press Enter to inspect.\n\n{}",
            app.config_scope.as_deref().unwrap_or("global"),
            config_copy_status(app)
        );
    };
    let operations = if detail.operations.is_empty() {
        "none reported".into()
    } else {
        detail
            .operations
            .iter()
            .map(|operation| {
                let source = operation.file.as_ref().map_or_else(
                    || "source unavailable".into(),
                    |file| {
                        operation.line.map_or_else(
                            || file.display().to_string(),
                            |line| format!("{}:{line}", file.display()),
                        )
                    },
                );
                format!(
                    "{} @ {}{}",
                    operation.operation,
                    source,
                    operation
                        .value
                        .as_ref()
                        .map_or_else(String::new, |value| format!(" = {value}"))
                )
            })
            .collect::<Vec<_>>()
            .join("\n  ")
    };
    format!(
        "Variable: {}\nScope: {}\nEffective value: {}\nUnexpanded value: {}\n{}\nProvenance: {}\nActive overrides: {}\nOperations:\n  {}",
        detail.identity.name,
        detail
            .identity
            .recipe
            .as_deref()
            .map_or("global", |recipe| recipe),
        detail.effective_value.as_deref().unwrap_or("unavailable"),
        detail.unexpanded_value.as_deref().unwrap_or("unavailable"),
        config_copy_status(app),
        detail.provenance.as_deref().unwrap_or("unavailable"),
        if detail.active_overrides.is_empty() {
            "none reported".into()
        } else {
            detail.active_overrides.join(", ")
        },
        operations,
    )
}

fn config_copy_status(app: &App) -> String {
    let copy = [
        ("C effective", ConfigCopyValue::Effective),
        ("U unexpanded", ConfigCopyValue::Unexpanded),
    ]
    .into_iter()
    .map(|(label, value)| {
        selected_config_copy_value(app, value).map_or_else(
            |reason| format!("{label}: disabled ({reason})"),
            |_| format!("{label}: enabled"),
        )
    })
    .collect::<Vec<_>>()
    .join(" | ");
    let source = config_source_disabled_reason(app).map_or_else(
        || "o source: enabled".into(),
        |reason| format!("o source: disabled ({reason})"),
    );
    let scope = if app.workspace.recipes.is_empty() {
        "s scope: global only (no recipes reported)".into()
    } else {
        format!(
            "s scope: enabled ({} recipes; active {})",
            app.workspace.recipes.len(),
            app.config_scope.as_deref().unwrap_or("global")
        )
    };
    let compare = config_comparison(app).map_or_else(
        |reason| format!("c compare: disabled ({reason})"),
        |_| "c compare: enabled".into(),
    );
    let edit = if config_edit_disabled_reason(app).is_none() {
        "E edit: enabled"
    } else {
        "E edit: disabled"
    };
    format!("{scope} | {edit}\n{compare}\n{source}\n{copy}")
}

fn config(frame: &mut Frame, app: &App, area: Rect) {
    let variables = config_variables(app);
    let variable_count = variables.len();
    let chunks = Layout::vertical([Constraint::Percentage(32), Constraint::Min(5)]).split(area);
    frame.render_widget(
        Table::new(
            variables
                .into_iter()
                .enumerate()
                .map(|(index, (name, value))| {
                    Row::new(vec![Cell::from(name.as_str()), Cell::from(value.as_str())])
                        .style(selected_style(app, index == app.config_selection))
                }),
            [Constraint::Percentage(35), Constraint::Percentage(65)],
        )
        .header(
            Row::new(["Variable", "Effective value"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(
            Block::default()
                .title(metadata_title(
                    format!(
                        "Effective configuration (shown: {} of {}, read-only)",
                        variable_count,
                        app.workspace.variables.len()
                    ),
                    app,
                ))
                .borders(Borders::ALL),
        ),
        chunks[0],
    );
    let detail = config_inspector(app);
    frame.render_widget(
        Paragraph::new(format!(
            "{detail}\n\nEnter refreshes detail; o opens provenance when available."
        ))
        .block(
            Block::default()
                .title("Selected variable")
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: false }),
        chunks[1],
    );
}
fn bbmask(frame: &mut Frame, app: &App, area: Rect) {
    let value = app.workspace.variables.get("BBMASK").map_or(
        "(BBMASK is not set in the effective configuration)",
        String::as_str,
    );
    let provenance = app
        .workspace
        .variable_provenance
        .get("BBMASK")
        .map_or("backend did not provide source provenance", String::as_str);
    let patterns = value
        .split_whitespace()
        .enumerate()
        .map(|(index, pattern)| format!("{:>3}. {pattern}", index + 1))
        .collect::<Vec<_>>();
    let pattern_text = if patterns.is_empty() {
        "No masked recipe patterns are active.".into()
    } else {
        patterns.join("\n")
    };
    frame.render_widget(
        Paragraph::new(format!(
            "Effective BBMASK patterns:\n{pattern_text}\n\nProvenance: {provenance}\n\ne edits the value; Yoctui previews the exact local.conf assignment and requires confirmation."
        ))
        .block(Block::default().title("Effective BBMASK").borders(Borders::ALL))
        .wrap(Wrap { trim: false }),
        area,
    );
}
fn bbmask_assignment(value: &str) -> String {
    format!(
        "BBMASK = \"{}\"",
        value.replace('\\', "\\\\").replace('"', "\\\"")
    )
}
fn help(frame: &mut Frame, area: Rect) {
    frame.render_widget(Paragraph::new("B Image build options for the effective MACHINE; b build, c clean, m menuconfig, e choose target\n! Open an inherited Yocto shell; exit returns to Yoctui\nb Choose target and start build; h build history; Dashboard Up/Down scrolls observed package task progress\nc Cancel active build\nl Logs   f toggle follow   w toggle wrapping   s cycle severity\nR cycle recipe filter   T cycle task filter   n/N previous/next match\ne Errors   o open selected source log, layer directory, or config provenance\nr Recipes: z confirmed diffsigs task, Z signature inspection, e provider, o logs, p patches, b/f tasks, V CVE, X SPDX, d modify, u update, F finish, P deploy, D reset\ny Layers: e in-TUI edit, o external editor   v Configuration   x effective BBMASK, e edit with preview\n/ Search recipes, layers, or configuration   Esc Dashboard   q Quit\n\nSignatures: Up/Down select, 1/2 choose sides, c compare, r refresh, e provider, Esc back/cancel.\nCVE/SPDX, cleansstate, forced tasks, Devtool reset/update-recipe/finish/deploy, BBMASK changes, and quitting an active build require confirmation.").block(Block::default().title("Help").borders(Borders::ALL)),area)
}
#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};
    use yoctui_model::{Action, BuildRequest, update};

    fn rendered_text(app: &App, width: u16, height: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|frame| render(frame, app)).unwrap();
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    #[test]
    fn theme_palettes_define_distinct_semantic_roles() {
        for theme in [
            Theme::Dark,
            Theme::Light,
            Theme::MatrixGreen,
            Theme::HighContrast,
            Theme::Monochrome,
        ] {
            let mut app = App::new(10, 1_000);
            app.theme = theme;
            let palette = ThemePalette::for_app(&app);
            let mut terminal = Terminal::new(TestBackend::new(100, 25)).unwrap();
            terminal.draw(|frame| render(frame, &app)).unwrap();
            assert!(
                terminal
                    .backend()
                    .buffer()
                    .content
                    .iter()
                    .any(|cell| cell.bg == palette.background)
            );
            if theme == Theme::Monochrome {
                assert!(palette.attribute_only);
                assert!(palette.focus().add_modifier.contains(Modifier::BOLD));
                assert!(palette.selected().add_modifier.contains(Modifier::REVERSED));
            } else {
                assert!(!palette.attribute_only);
                assert_ne!(palette.focused_border, palette.border);
                assert_ne!(palette.selection_background, palette.background);
                assert_ne!(palette.error, palette.success);
                assert_ne!(palette.warning, palette.info);
            }
        }
    }

    #[test]
    fn theme_no_color_uses_attributes_for_focus_selection_and_severity() {
        let mut app = App::new(10, 1_000);
        app.theme = Theme::Light;
        app.color_enabled = false;
        let palette = ThemePalette::for_app(&app);

        assert!(palette.attribute_only);
        assert_eq!(palette.foreground, Color::Reset);
        assert!(palette.focus().add_modifier.contains(Modifier::BOLD));
        assert!(
            selected_style(&app, true)
                .add_modifier
                .contains(Modifier::REVERSED)
        );
        assert!(
            severity_style(&app, Severity::Error)
                .add_modifier
                .contains(Modifier::UNDERLINED)
        );
        assert_ne!(
            severity_style(&app, Severity::Warning).add_modifier,
            severity_style(&app, Severity::Trace).add_modifier
        );

        app.focus = FocusTarget::Navigator;
        let mut terminal = Terminal::new(TestBackend::new(100, 25)).unwrap();
        terminal.draw(|frame| render(frame, &app)).unwrap();
        assert!(terminal.backend().buffer().content.iter().any(|cell| {
            cell.modifier.contains(Modifier::REVERSED) || cell.modifier.contains(Modifier::BOLD)
        }));
    }

    #[test]
    fn theme_light_shell_and_dialog_apply_the_semantic_background() {
        let mut terminal = Terminal::new(TestBackend::new(100, 25)).unwrap();
        let mut app = App::new(10, 1_000);
        app.theme = Theme::Light;
        app.dialogs.push_back(Dialog::BuildOptions);
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let buffer = terminal.backend().buffer();

        assert!(buffer.content.iter().any(|cell| cell.bg == Color::White));
        assert!(buffer.content.iter().any(|cell| cell.fg == Color::Blue));
    }

    #[test]
    fn theme_progress_and_log_severity_use_semantic_roles() {
        let mut terminal = Terminal::new(TestBackend::new(140, 30)).unwrap();
        let mut app = App::new(10, 1_000);
        app.theme = Theme::HighContrast;
        app.tasks.insert(
            yoctui_model::TaskId("busybox:do_compile".into()),
            yoctui_model::TaskInfo {
                id: yoctui_model::TaskId("busybox:do_compile".into()),
                recipe: "busybox".into(),
                task: "do_compile".into(),
                progress: Some(50),
                ..yoctui_model::TaskInfo::default()
            },
        );
        terminal.draw(|frame| render(frame, &app)).unwrap();
        assert!(
            terminal
                .backend()
                .buffer()
                .content
                .iter()
                .any(|cell| cell.fg == Color::Cyan)
        );

        app.screen = Screen::Logs;
        app.logs.insert(yoctui_model::LogEntry {
            id: 0,
            severity: Severity::Error,
            message: "compile failed".into(),
            recipe: Some("busybox".into()),
            task: Some("do_compile".into()),
            path: None,
            timestamp: SystemTime::UNIX_EPOCH,
            build: None,
            protected: false,
            diagnostic: None,
        });
        terminal.draw(|frame| render(frame, &app)).unwrap();
        assert!(
            terminal
                .backend()
                .buffer()
                .content
                .iter()
                .any(|cell| cell.fg == Color::LightRed)
        );
    }

    #[test]
    fn settings_workspace_renders_typed_rows_and_controls_on_narrow_terminals() {
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Settings;
        app.settings_selection = 5;
        app.settings_dirty = true;
        app.theme = Theme::MatrixGreen;
        app.animation_speed = yoctui_model::AnimationSpeed::Slow;
        app.reduced_motion = true;
        app.logs.follow = false;

        let output = rendered_text(&app, 80, 24);
        assert!(output.contains("Settings (not saved)"));
        assert!(output.contains("Theme"));
        assert!(output.contains("MatrixGreen"));
        assert!(output.contains("Animation speed"));
        assert!(output.contains("Reduced motion"));
        assert!(output.contains("Log wrap"));
        assert!(output.contains("Log follow"));
        assert!(output.contains("select"));
        assert!(output.contains("change"));
    }

    #[test]
    fn animation_fast_slow_and_reduced_motion_have_deterministic_cadence() {
        let mut fast = App::new(10, 1_000);
        fast.animation_frame = 0;
        let first = task_activity(&fast, None);
        fast.animation_frame = 1;
        assert_ne!(task_activity(&fast, None), first);

        let mut slow = App::new(10, 1_000);
        slow.animation_speed = yoctui_model::AnimationSpeed::Slow;
        slow.animation_frame = 0;
        let first = task_activity(&slow, None);
        slow.animation_frame = 1;
        assert_eq!(task_activity(&slow, None), first);
        slow.animation_frame = 3;
        assert_ne!(task_activity(&slow, None), first);

        slow.reduced_motion = true;
        assert_eq!(task_activity(&slow, None), " active");
        assert_eq!(task_activity(&slow, Some(0)), "");
    }

    #[test]
    fn animation_unknown_progress_never_fabricates_a_percentage() {
        for (theme, color_enabled) in [
            (Theme::Dark, true),
            (Theme::Light, true),
            (Theme::MatrixGreen, true),
            (Theme::HighContrast, true),
            (Theme::Monochrome, true),
            (Theme::Dark, false),
        ] {
            let mut app = App::new(10, 1_000);
            app.theme = theme;
            app.color_enabled = color_enabled;
            app.tasks.insert(
                yoctui_model::TaskId("busybox:do_compile".into()),
                yoctui_model::TaskInfo {
                    id: yoctui_model::TaskId("busybox:do_compile".into()),
                    recipe: "busybox".into(),
                    task: "do_compile".into(),
                    progress: None,
                    ..yoctui_model::TaskInfo::default()
                },
            );
            let output = rendered_text(&app, 300, 30);
            assert!(output.contains("progress unknown"), "{output}");
            assert!(!output.contains("busybox:do_compile 0%"));
            let _ = rendered_text(&app, 80, 24);
        }
    }

    #[test]
    fn images_workspace_renders_typed_artifacts_inspector_and_responsive_modes() {
        let mut app = App::new(20, 20_000);
        app.screen = Screen::Images;
        app.workspace
            .variables
            .insert("MACHINE".into(), "qemux86-64".into());
        app.build.target = Some("core-image-minimal".into());
        let request = yoctui_model::ImageArtifactRequest {
            generation: 1,
            machine: "qemux86-64".into(),
        };
        let path = std::path::PathBuf::from("/deploy/qemux86-64/core-image-minimal-qemux86-64.wic");
        let artifact = yoctui_model::ImageArtifact {
            identity: yoctui_model::ImageArtifactIdentity {
                machine: "qemux86-64".into(),
                image: "core-image-minimal".into(),
                path: path.clone(),
            },
            kind: yoctui_model::ImageArtifactKind::Wic,
            size_bytes: ImageArtifactField::Available(8192),
            modified_unix_seconds: ImageArtifactField::Available(1_700_000_000),
            checksums: ImageArtifactField::Available(vec![yoctui_model::ImageChecksum {
                algorithm: "sha256".into(),
                digest: "abcdef".into(),
                source: "/deploy/qemux86-64/image.sha256".into(),
            }]),
            manifests: ImageArtifactField::Available(vec![
                "/deploy/qemux86-64/image.manifest".into(),
            ]),
            licenses: ImageArtifactField::Unavailable,
            spdx: ImageArtifactField::Available(vec!["/deploy/qemux86-64/image.spdx.json".into()]),
            wic_files: ImageArtifactField::Available(vec![path]),
        };
        app.image_artifact_selection = Some(artifact.identity.clone());
        app.image_artifacts = ImageArtifactInventoryState::Partial {
            request,
            inventory: yoctui_model::ImageArtifactInventory {
                machine: "qemux86-64".into(),
                deploy_directory: ImageArtifactField::Available("/deploy/qemux86-64".into()),
                artifacts: vec![artifact],
            },
            limitations: vec!["one symlink was not followed".into()],
        };

        for (width, theme, color) in [
            (180, Theme::Dark, true),
            (120, Theme::Light, true),
            (80, Theme::Monochrome, false),
        ] {
            app.theme = theme;
            app.color_enabled = color;
            let output = rendered_text(&app, width, 32);
            assert!(output.contains("core-image-minimal"), "{output}");
            assert!(output.contains("wic"), "{output}");
            assert!(output.contains("refresh"), "{output}");
        }
        let wide = rendered_text(&app, 180, 40);
        assert!(wide.contains("Deploy directory"), "{wide}");
        assert!(wide.contains("sha256"), "{wide}");
        assert!(wide.contains("SPDX/SBOM"), "{wide}");
        assert!(wide.contains("one symlink was not followed"), "{wide}");
    }

    #[test]
    fn images_workspace_renders_loading_empty_failure_and_search_empty_states() {
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Images;
        app.workspace
            .variables
            .insert("MACHINE".into(), "qemux86-64".into());
        let request = yoctui_model::ImageArtifactRequest {
            generation: 1,
            machine: "qemux86-64".into(),
        };
        app.image_artifacts = ImageArtifactInventoryState::Loading {
            request: request.clone(),
        };
        assert!(rendered_text(&app, 100, 25).contains("Loading deployed image artifacts"));
        app.image_artifacts = ImageArtifactInventoryState::AvailableEmpty {
            request: request.clone(),
            inventory: yoctui_model::ImageArtifactInventory {
                machine: "qemux86-64".into(),
                deploy_directory: ImageArtifactField::Available("/deploy/qemux86-64".into()),
                artifacts: Vec::new(),
            },
        };
        assert!(rendered_text(&app, 100, 25).contains("No deployed image artifacts"));
        app.image_artifacts = ImageArtifactInventoryState::Failed {
            request,
            message: "DEPLOY_DIR_IMAGE is unavailable".into(),
        };
        assert!(rendered_text(&app, 100, 25).contains("Artifact scan failed"));
    }

    fn qemu_workspace_app() -> App {
        let mut app = App::new(20, 20_000);
        app.screen = Screen::Images;
        app.workspace
            .variables
            .insert("MACHINE".into(), "qemux86-64".into());
        let identity = yoctui_model::ImageArtifactIdentity {
            machine: "qemux86-64".into(),
            image: "core-image-minimal".into(),
            path: "/deploy/qemux86-64/core-image-minimal.wic".into(),
        };
        let artifact = yoctui_model::ImageArtifact {
            identity: identity.clone(),
            kind: yoctui_model::ImageArtifactKind::Wic,
            size_bytes: ImageArtifactField::Available(8_192),
            modified_unix_seconds: ImageArtifactField::Available(1_700_000_000),
            checksums: ImageArtifactField::Unavailable,
            manifests: ImageArtifactField::Unavailable,
            licenses: ImageArtifactField::Unavailable,
            spdx: ImageArtifactField::Unavailable,
            wic_files: ImageArtifactField::Available(vec![identity.path.clone()]),
        };
        app.image_artifact_selection = Some(identity.clone());
        app.image_artifacts = ImageArtifactInventoryState::Available {
            request: yoctui_model::ImageArtifactRequest {
                generation: 1,
                machine: "qemux86-64".into(),
            },
            inventory: yoctui_model::ImageArtifactInventory {
                machine: "qemux86-64".into(),
                deploy_directory: ImageArtifactField::Available("/deploy/qemux86-64".into()),
                artifacts: vec![artifact],
            },
        };
        app.qemu_capability = QemuCapability::Available {
            executable: "/opt/poky/scripts/runqemu".into(),
            compatible_images: vec![identity],
        };
        app
    }

    fn qemu_running_workspace_app() -> (App, QemuSessionId) {
        let mut app = qemu_workspace_app();
        let _ = yoctui_model::update(&mut app, yoctui_model::Action::BeginSelectedQemuLaunch);
        let _ = yoctui_model::update(&mut app, yoctui_model::Action::PreviewQemuLaunch);
        let Some(yoctui_model::Effect::StartQemuSession { id, .. }) =
            yoctui_model::update(&mut app, yoctui_model::Action::ConfirmQemuLaunch)
        else {
            panic!("expected session");
        };
        let _ = yoctui_model::update(
            &mut app,
            yoctui_model::Action::QemuSessionStarting {
                id,
                started_at: SystemTime::UNIX_EPOCH,
            },
        );
        let _ = yoctui_model::update(&mut app, yoctui_model::Action::QemuSessionRunning { id });
        app.focus = FocusTarget::Inspector;
        (app, id)
    }

    #[test]
    fn qemu_workspace_renders_capability_dialogs_session_and_responsive_states() {
        let mut app = qemu_workspace_app();
        app.focus = FocusTarget::Inspector;
        for (width, height) in [(80, 24), (100, 30), (160, 40)] {
            let output = rendered_text(&app, width, height);
            if width == 160 {
                assert!(output.contains("runqemu capability"), "{output}");
                assert!(output.contains("ready for selected artifact"), "{output}");
            }
        }
        assert!(rendered_text(&app, 70, 20).contains("needs at least 80x24"));

        let artifact = app.selected_image_artifact().unwrap().clone();
        let mut launch = QemuLaunchDialog::new(yoctui_model::QemuLaunchDraft::for_artifact(
            artifact.identity,
            artifact.kind,
        ));
        launch.selected_field = QemuLaunchField::Kernel;
        launch.editing = true;
        launch.draft.kernel = "relative/kernel".into();
        launch.validation_error = Some("kernel path must be absolute".into());
        app.dialogs.push_front(Dialog::QemuLaunch(launch.clone()));
        for (width, height) in [(80, 24), (100, 30), (160, 40)] {
            let output = rendered_text(&app, width, height);
            assert!(output.contains("Launch runqemu"), "{output}");
            assert!(output.contains("Kernel"), "{output}");
            assert!(output.contains("editing"), "{output}");
            assert!(output.contains("Validation"), "{output}");
        }

        app.dialogs.clear();
        let preview = launch.draft.preview(&app.qemu_capability).unwrap_err();
        assert!(preview.contains("normalized absolute"));
        launch.draft.kernel.clear();
        let preview = launch.draft.preview(&app.qemu_capability).unwrap();
        app.dialogs
            .push_front(Dialog::QemuLaunchConfirmation(preview));
        let confirmation = rendered_text(&app, 100, 30);
        assert!(
            confirmation.contains("Exact argument vector"),
            "{confirmation}"
        );
        assert!(confirmation.contains("qemumemory=1024"), "{confirmation}");

        app.dialogs.clear();
        let _ = yoctui_model::update(&mut app, yoctui_model::Action::BeginSelectedQemuLaunch);
        let _ = yoctui_model::update(&mut app, yoctui_model::Action::PreviewQemuLaunch);
        let Some(yoctui_model::Effect::StartQemuSession { id, .. }) =
            yoctui_model::update(&mut app, yoctui_model::Action::ConfirmQemuLaunch)
        else {
            panic!("expected session");
        };
        let _ = yoctui_model::update(
            &mut app,
            yoctui_model::Action::QemuSessionStarting {
                id,
                started_at: SystemTime::UNIX_EPOCH,
            },
        );
        let _ = yoctui_model::update(&mut app, yoctui_model::Action::QemuSessionRunning { id });
        let _ = yoctui_model::update(
            &mut app,
            yoctui_model::Action::AppendQemuSessionOutput {
                id,
                stream: yoctui_model::QemuOutputStream::Stderr,
                line: "guest warning".into(),
                truncated: true,
                timestamp: SystemTime::UNIX_EPOCH,
            },
        );
        app.focus = FocusTarget::Inspector;
        let running = rendered_text(&app, 160, 40);
        assert!(running.contains("Status: running"), "{running}");
        assert!(
            running.contains("[stderr] guest warning [truncated]"),
            "{running}"
        );

        app.dialogs
            .push_front(Dialog::QemuCancellationConfirmation(id));
        let cancellation = rendered_text(&app, 80, 24);
        assert!(
            cancellation.contains("Confirm runqemu cancellation"),
            "{cancellation}"
        );
        app.dialogs.clear();
        let _ = yoctui_model::update(
            &mut app,
            yoctui_model::Action::FailQemuSession {
                id,
                message: "display unavailable".into(),
                exit_code: Some(1),
                finished_at: SystemTime::UNIX_EPOCH,
            },
        );
        let failed = rendered_text(&app, 160, 40);
        assert!(failed.contains("Status: failed"), "{failed}");
        assert!(failed.contains("display unavailable"), "{failed}");

        for capability in [
            QemuCapability::MissingTool,
            QemuCapability::MissingCompatibleImage,
            QemuCapability::Failed {
                message: "inspection denied".into(),
            },
        ] {
            app.qemu_capability = capability;
            let output = rendered_text(&app, 160, 40);
            assert!(output.contains("runqemu capability"), "{output}");
        }
    }

    #[test]
    fn qemu_workspace_renders_each_terminal_session_outcome() {
        let (mut succeeded, succeeded_id) = qemu_running_workspace_app();
        let _ = yoctui_model::update(
            &mut succeeded,
            yoctui_model::Action::CompleteQemuSession {
                id: succeeded_id,
                exit_code: 0,
                finished_at: SystemTime::UNIX_EPOCH,
            },
        );
        assert!(rendered_text(&succeeded, 160, 40).contains("Status: succeeded"));

        let (mut failed, failed_id) = qemu_running_workspace_app();
        let _ = yoctui_model::update(
            &mut failed,
            yoctui_model::Action::FailQemuSession {
                id: failed_id,
                message: "failed display".into(),
                exit_code: Some(1),
                finished_at: SystemTime::UNIX_EPOCH,
            },
        );
        assert!(rendered_text(&failed, 160, 40).contains("Status: failed"));

        let (mut lost, lost_id) = qemu_running_workspace_app();
        let _ = yoctui_model::update(
            &mut lost,
            yoctui_model::Action::LoseQemuSession {
                id: lost_id,
                message: "runner lost".into(),
                finished_at: SystemTime::UNIX_EPOCH,
            },
        );
        assert!(rendered_text(&lost, 160, 40).contains("Status: lost"));

        let (mut cancelled, cancelled_id) = qemu_running_workspace_app();
        let _ = yoctui_model::update(
            &mut cancelled,
            yoctui_model::Action::BeginQemuSessionCancellation { id: cancelled_id },
        );
        let _ = yoctui_model::update(
            &mut cancelled,
            yoctui_model::Action::ConfirmQemuSessionCancellation,
        );
        let _ = yoctui_model::update(
            &mut cancelled,
            yoctui_model::Action::CancelQemuSession {
                id: cancelled_id,
                exit_code: Some(130),
                finished_at: SystemTime::UNIX_EPOCH,
            },
        );
        assert!(rendered_text(&cancelled, 160, 40).contains("Status: cancelled"));
    }

    #[test]
    fn animation_is_absent_from_determinate_and_terminal_rows() {
        let mut app = App::new(10, 1_000);
        app.tasks.insert(
            yoctui_model::TaskId("busybox:do_compile".into()),
            yoctui_model::TaskInfo {
                id: yoctui_model::TaskId("busybox:do_compile".into()),
                recipe: "busybox".into(),
                task: "do_compile".into(),
                progress: Some(42),
                ..yoctui_model::TaskInfo::default()
            },
        );
        app.completed_tasks.push_back(yoctui_model::CompletedTask {
            task: yoctui_model::TaskInfo {
                id: yoctui_model::TaskId("base-files:do_install".into()),
                recipe: "base-files".into(),
                task: "do_install".into(),
                progress: None,
                ..yoctui_model::TaskInfo::default()
            },
            success: true,
        });
        let output = rendered_text(&app, 300, 30);
        assert!(output.contains("busybox:do_compile 42%"));
        assert!(
            output.contains("base-files:do_install 100% complete"),
            "{output}"
        );
        assert!(!output.contains("base-files:do_install▸"));
        assert!(!output.contains("base-files:do_install active"));
    }

    #[test]
    fn renders_small_terminal() {
        let mut terminal = Terminal::new(TestBackend::new(62, 18)).unwrap();
        terminal.draw(|f| render(f, &App::new(1, 1))).unwrap();
        assert!(
            terminal
                .backend()
                .buffer()
                .content
                .iter()
                .any(|c| c.symbol() == "Y")
        );
    }
    #[test]
    fn persistent_shell_degrades_across_supported_terminal_widths() {
        for (width, height, expected) in [
            (140, 30, "Inspector"),
            (100, 24, "Navigator"),
            (80, 24, "Build"),
        ] {
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal
                .draw(|frame| render(frame, &App::new(10, 1_000)))
                .unwrap();
            let output = terminal
                .backend()
                .buffer()
                .content
                .iter()
                .map(|cell| cell.symbol())
                .collect::<String>();
            assert!(
                output.contains(expected),
                "{width}x{height} should show {expected}"
            );
        }
    }
    #[test]
    fn responsive_shell_uses_semantic_content_at_every_breakpoint() {
        let mut app = App::new(10, 1_000);

        let wide = rendered_text(&app, 130, 24);
        assert!(wide.contains("Navigator"));
        assert!(wide.contains("Build"));
        assert!(wide.contains("Inspector"));

        let medium = rendered_text(&app, 129, 24);
        assert!(medium.contains("Navigator"));
        assert!(medium.contains("Build"));
        assert!(!medium.contains("Inspector"));

        app.focus = FocusTarget::Inspector;
        let medium_inspector = rendered_text(&app, 100, 24);
        assert!(medium_inspector.contains("Navigator"));
        assert!(medium_inspector.contains("Inspector"));

        app.focus = FocusTarget::Workspace;
        let narrow_workspace = rendered_text(&app, 99, 24);
        assert!(narrow_workspace.contains("Panes: Navigator  [Workspace]  Inspector"));
        assert!(narrow_workspace.contains("Build"));

        app.focus = FocusTarget::Navigator;
        let narrow_navigator = rendered_text(&app, 80, 24);
        assert!(narrow_navigator.contains("Panes: [Navigator]  Workspace  Inspector"));
        assert!(narrow_navigator.contains("Dashboard"));

        app.focus = FocusTarget::Inspector;
        let narrow_inspector = rendered_text(&app, 80, 24);
        assert!(narrow_inspector.contains("Panes: Navigator  Workspace  [Inspector]"));
        assert!(narrow_inspector.contains("Select an item in the workspace"));

        let too_small = rendered_text(&app, 79, 23);
        assert!(too_small.contains("Yoctui needs at least 80x24"));
        assert!(too_small.contains("Current terminal: 79x23"));
    }
    #[test]
    fn responsive_resize_preserves_the_selected_pane() {
        let mut app = App::new(10, 1_000);
        app.focus = FocusTarget::Inspector;
        let mut terminal = Terminal::new(TestBackend::new(130, 24)).unwrap();
        terminal.draw(|frame| render(frame, &app)).unwrap();

        terminal.backend_mut().resize(100, 24);
        terminal.autoresize().unwrap();
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let medium = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(medium.contains("Inspector"));

        terminal.backend_mut().resize(80, 24);
        terminal.autoresize().unwrap();
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let narrow = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(narrow.contains("[Inspector]"));
        assert_eq!(app.focus, FocusTarget::Inspector);
    }
    #[test]
    fn responsive_all_screens_and_dialogs_render_at_boundary_sizes() {
        let screens = [
            Screen::Dashboard,
            Screen::Tasks,
            Screen::BuildHistory,
            Screen::Dependencies,
            Screen::LayerRelationships,
            Screen::Recipes,
            Screen::Images,
            Screen::Layers,
            Screen::Configuration,
            Screen::Bbmask,
            Screen::Logs,
            Screen::Errors,
            Screen::Help,
            Screen::Settings,
        ];
        for screen in screens {
            for (width, height) in [(130, 24), (129, 24), (100, 24), (99, 24), (80, 24)] {
                let mut app = App::new(10, 1_000);
                app.screen = screen;
                let _ = rendered_text(&app, width, height);
            }
        }

        let mut build_options = App::new(10, 1_000);
        build_options.dialogs.push_back(Dialog::BuildOptions);
        build_options.focus = FocusTarget::Dialog;
        let _ = rendered_text(&build_options, 80, 24);

        let mut palette = App::new(10, 1_000);
        palette.command_palette_open = true;
        palette.focus = FocusTarget::CommandPalette;
        let _ = rendered_text(&palette, 80, 24);

        let mut confirmation = App::new(10, 1_000);
        confirmation
            .dialogs
            .push_back(Dialog::RecipeTaskConfirmation(BuildRequest {
                targets: vec!["base-files".into()],
                task: Some("listtasks".into()),
                force: false,
            }));
        confirmation.focus = FocusTarget::Dialog;
        let _ = rendered_text(&confirmation, 80, 24);
    }
    #[test]
    fn dialog_families_render_on_narrow_supported_terminals() {
        let dialogs = vec![
            (Dialog::BuildOptions, "Image build options"),
            (Dialog::BuildCompletion, "Build finished"),
            (
                Dialog::BuildTarget {
                    input: "busybox".into(),
                    task: None,
                },
                "Build target",
            ),
            (
                Dialog::ImagePicker(yoctui_model::ImagePicker {
                    images: vec!["core-image-minimal".into()],
                    selection: 0,
                }),
                "Available image targets",
            ),
            (
                Dialog::RecipeTaskConfirmation(BuildRequest {
                    targets: vec!["busybox".into()],
                    task: None,
                    force: false,
                }),
                "Confirm recipe task",
            ),
            (
                Dialog::DevtoolModifyConfirmation(yoctui_model::RecipeIdentity {
                    name: "busybox".into(),
                    file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
                }),
                "Confirm Devtool modify",
            ),
            (
                Dialog::DevtoolResetConfirmation(yoctui_model::DevtoolResetPlan {
                    identity: yoctui_model::RecipeIdentity {
                        name: "busybox".into(),
                        file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
                    },
                    source_path: "/build/workspace/sources/busybox".into(),
                }),
                "Confirm Devtool reset",
            ),
            (
                Dialog::DevtoolUpdateConfirmation(yoctui_model::RecipeIdentity {
                    name: "busybox".into(),
                    file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
                }),
                "Confirm Devtool update-recipe",
            ),
            (
                Dialog::DevtoolFinishPicker(yoctui_model::DevtoolFinishPicker {
                    identity: yoctui_model::RecipeIdentity {
                        name: "busybox".into(),
                        file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
                    },
                    layers: vec![yoctui_model::Layer {
                        name: "meta".into(),
                        path: "/layers/meta".into(),
                        priority: Some(5),
                    }],
                    selection: 0,
                }),
                "Devtool finish busybox",
            ),
            (
                Dialog::DevtoolFinishConfirmation(yoctui_model::DevtoolFinishPlan {
                    identity: yoctui_model::RecipeIdentity {
                        name: "busybox".into(),
                        file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
                    },
                    layer: yoctui_model::Layer {
                        name: "meta".into(),
                        path: "/layers/meta".into(),
                        priority: Some(5),
                    },
                }),
                "Confirm Devtool finish",
            ),
            (
                Dialog::DevtoolDeploy(yoctui_model::DevtoolDeployDraft {
                    identity: yoctui_model::RecipeIdentity {
                        name: "busybox".into(),
                        file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
                    },
                    target: "qemu".into(),
                }),
                "Devtool deploy target",
            ),
            (
                Dialog::DevtoolDeployConfirmation(yoctui_model::DevtoolDeployPlan {
                    identity: yoctui_model::RecipeIdentity {
                        name: "busybox".into(),
                        file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
                    },
                    target: "qemu".into(),
                }),
                "Confirm Devtool deploy-target",
            ),
            (
                Dialog::BbmaskEdit {
                    input: "meta-old/.*".into(),
                },
                "Edit effective BBMASK",
            ),
            (
                Dialog::BbmaskConfirmation("meta-old/.*".into()),
                "Confirm BBMASK change",
            ),
            (
                Dialog::RecipeEditor(RecipeEditor {
                    recipe: "busybox".into(),
                    root: "/workspace/busybox".into(),
                    files: vec!["main.c".into()],
                    selection: 0,
                    content: "int main() {}".into(),
                    editing: false,
                    dirty: false,
                }),
                "Workspace file tree",
            ),
            (Dialog::QuitConfirmation, "Confirm quit"),
        ];

        for (dialog, title) in dialogs {
            let mut app = App::new(10, 1_000);
            app.build.status = yoctui_model::BuildStatus::Completed;
            app.dialogs.push_back(dialog);
            let output = rendered_text(&app, 80, 24);
            assert!(output.contains(title), "missing {title} in narrow dialog");
        }
    }
    #[test]
    fn command_palette_renders_search_results_and_disabled_explanations() {
        let mut app = App::new(10, 1_000);
        app.command_palette_open = true;
        app.command_palette_query = "build".into();
        let output = rendered_text(&app, 100, 25);
        assert!(output.contains("Search: build_"));
        assert!(output.contains("Build image"));
        assert!(output.contains("unavailable"));
        assert!(output.contains("Load a Yocto workspace first"));
        assert!(output.contains("Type to search"));

        app.command_palette_query = "nothing matches this".into();
        let output = rendered_text(&app, 80, 24);
        assert!(output.contains("No commands match"));
    }
    #[test]
    fn command_palette_selection_description_and_shortcut_render_in_all_themes() {
        for theme in [
            Theme::Dark,
            Theme::Light,
            Theme::MatrixGreen,
            Theme::HighContrast,
            Theme::Monochrome,
        ] {
            let mut app = App::new(10, 1_000);
            app.theme = theme;
            app.command_palette_open = true;
            app.command_palette_query = "Open Settings".into();
            let output = rendered_text(&app, 80, 24);
            assert!(output.contains("Open Settings"));
            assert!(output.contains("persistent visual"));
            assert!(output.contains("[none]"));
        }
    }
    #[test]
    fn dialog_focus_is_trapped_then_visibly_restored_to_inspector() {
        let mut app = App::new(10, 1_000);
        app.focus = FocusTarget::Inspector;
        let _ = update(&mut app, Action::OpenBuildOptions);

        let dialog = rendered_text(&app, 100, 24);
        assert_eq!(app.focus, FocusTarget::Dialog);
        assert!(dialog.contains("Image build options"));
        assert!(!dialog.contains("Panes:"));

        let _ = update(&mut app, Action::CloseBuildOptions);
        let restored = rendered_text(&app, 100, 24);
        assert_eq!(app.focus, FocusTarget::Inspector);
        assert!(restored.contains("Inspector"));
        assert!(!restored.contains("Image build options"));
    }
    #[test]
    fn formats_error_timestamp_without_panicking() {
        assert_eq!(timestamp_text(UNIX_EPOCH), "0s since Unix epoch");
    }
    #[test]
    fn no_color_selection_uses_reverse_video() {
        let mut app = App::new(10, 1_000);
        app.color_enabled = false;
        assert!(
            selected_style(&app, true)
                .add_modifier
                .contains(Modifier::REVERSED)
        );
        assert_eq!(selected_style(&app, true).bg, None);
    }

    #[test]
    fn renders_notification() {
        let mut app = App::new(1, 1);
        app.notification = Some("Backend unavailable".into());
        let mut terminal = Terminal::new(TestBackend::new(140, 30)).unwrap();
        terminal.draw(|f| render(f, &app)).unwrap();
        let screen = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(screen.contains("Backend unavailable"));
    }
    #[test]
    fn dashboard_renders_backend_and_build_metrics() {
        let mut terminal = Terminal::new(TestBackend::new(160, 32)).unwrap();
        let mut app = App::new(10, 1_000);
        app.backend = "bridge".into();
        app.build.completed = 3;
        app.build.total = Some(7);
        app.build.warnings = 2;
        app.build.errors = 1;
        app.workspace.release = Some("kirkstone".into());
        app.workspace.source_dir = Some("/src/poky".into());
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Backend: bridge"));
        assert!(output.contains("Tasks: 3/7"));
        assert!(output.contains("Warnings: 2  Errors: 1"));
        assert!(output.contains("Yocto: kirkstone @ /src/poky"));
    }
    #[test]
    fn bbmask_footer_shows_its_edit_shortcut() {
        let mut terminal = Terminal::new(TestBackend::new(300, 40)).unwrap();
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Bbmask;
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("e edit BBMASK"));
    }
    #[test]
    fn dashboard_renders_host_cpu_and_build_disk_space() {
        let mut terminal = Terminal::new(TestBackend::new(300, 40)).unwrap();
        let mut app = App::new(10, 1_000);
        app.host_telemetry.cpu_utilization_percent = Some(42);
        app.host_telemetry.disk_available_bytes = Some(8 * 1024 * 1024 * 1024);
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Host CPU: 42%"));
        assert!(output.contains("Disk 8.0 GiB"));
    }
    #[test]
    fn dashboard_renders_parse_progress() {
        let mut terminal = Terminal::new(TestBackend::new(100, 25)).unwrap();
        let mut app = App::new(10, 1_000);
        app.build.parse_current = Some(8);
        app.build.parse_total = Some(20);
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Parse progress: 8/20"));
    }
    #[test]
    fn dashboard_renders_build_exit_code() {
        let mut terminal = Terminal::new(TestBackend::new(100, 25)).unwrap();
        let mut app = App::new(10, 1_000);
        app.build.exit_code = Some(1);
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Exit code: 1"));
    }
    #[test]
    fn build_history_renders_completed_builds() {
        let mut terminal = Terminal::new(TestBackend::new(120, 25)).unwrap();
        let mut app = App::new(10, 1_000);
        app.screen = Screen::BuildHistory;
        app.build_history.push_back(yoctui_model::BuildRecord {
            target: Some("core-image-minimal".into()),
            success: true,
            exit_code: Some(0),
            elapsed: Some(std::time::Duration::from_secs(65)),
            completed_tasks: 42,
            warnings: 1,
            errors: 0,
        });
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Build history"));
        assert!(output.contains("core-image-minimal"));
        assert!(output.contains("Completed package tasks: 42"));
    }
    #[test]
    fn dependency_workspace_renders_typed_partial_graph_paths_and_responsive_states() {
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Dependencies;
        let root = DependencyNodeId::recipe("image");
        let task = DependencyNodeId::task("busybox", "do_compile");
        let orphan = DependencyNodeId::recipe("orphan");
        let (graph, _) = DependencyGraph::normalize(
            root.clone(),
            vec![
                yoctui_model::DependencyNode {
                    id: task.clone(),
                    provider: Some("/layers/meta/busybox.bb".into()),
                    log: Some("/build/tmp/log.do_compile".into()),
                },
                yoctui_model::DependencyNode::identity(orphan.clone()),
            ],
            vec![
                yoctui_model::DependencyEdge {
                    from: root.clone(),
                    to: task.clone(),
                    kind: DependencyEdgeKind::Task,
                },
                yoctui_model::DependencyEdge {
                    from: task.clone(),
                    to: root.clone(),
                    kind: DependencyEdgeKind::Task,
                },
            ],
            100,
            100,
        );
        app.dependency_graph = DependencyGraphState::Partial {
            graph,
            limitations: vec!["runtime edges unavailable".into()],
        };
        app.dependency_graph_selection = Some(task);
        let output = rendered_text(&app, 160, 36);
        assert!(output.contains("Dependency graph"));
        assert!(output.contains("busybox:do_compile"));
        assert!(output.contains("runtime edges unavailable"));
        assert!(output.contains("--task-->"));
        assert!(output.contains("/layers/meta/busybox.bb"));
        assert!(output.contains("Reverse / incoming"));
        for width in [129, 100, 80] {
            let output = rendered_text(&app, width, 24);
            assert!(output.contains("Dependency graph"));
        }

        app.focus = FocusTarget::Inspector;
        app.dependency_graph_selection = Some(orphan);
        let output = rendered_text(&app, 80, 24);
        assert!(output.contains("unreachable from root"));

        app.focus = FocusTarget::Workspace;
        app.dependency_graph = DependencyGraphState::NotLoaded;
        assert!(rendered_text(&app, 80, 24).contains("not loaded"));
        app.dependency_graph = DependencyGraphState::Loading { root: root.clone() };
        assert!(rendered_text(&app, 80, 24).contains("Stale rows are hidden"));
        app.dependency_graph = DependencyGraphState::AvailableEmpty { root: root.clone() };
        assert!(rendered_text(&app, 80, 24).contains("No dependency edges reported"));
        app.dependency_graph = DependencyGraphState::Failed {
            root,
            message: "server unavailable".into(),
        };
        assert!(rendered_text(&app, 80, 24).contains("server unavailable"));
    }
    #[test]
    fn dependency_workspace_reports_path_bounds_without_panicking() {
        let root = DependencyNodeId::recipe("node-0");
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        for index in 1..=66 {
            let previous = DependencyNodeId::recipe(format!("node-{}", index - 1));
            let current = DependencyNodeId::recipe(format!("node-{index}"));
            nodes.push(yoctui_model::DependencyNode::identity(current.clone()));
            edges.push(yoctui_model::DependencyEdge {
                from: previous,
                to: current,
                kind: DependencyEdgeKind::Build,
            });
        }
        let selected = DependencyNodeId::recipe("node-66");
        let (graph, _) = DependencyGraph::normalize(root, nodes, edges, 100, 100);
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Dependencies;
        app.focus = FocusTarget::Inspector;
        app.dependency_graph = DependencyGraphState::Available(graph);
        app.dependency_graph_selection = Some(selected);
        assert!(rendered_text(&app, 80, 24).contains("path limit reached"));
    }
    #[test]
    fn dashboard_renders_colored_task_progress_labels() {
        let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
        let mut app = App::new(10, 1_000);
        app.tasks.insert(
            yoctui_model::TaskId("busybox:do_compile".into()),
            yoctui_model::TaskInfo {
                id: yoctui_model::TaskId("busybox:do_compile".into()),
                recipe: "busybox".into(),
                task: "do_compile".into(),
                progress: Some(42),
                ..yoctui_model::TaskInfo::default()
            },
        );
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("busybox:do_compile 42%"));
    }
    #[test]
    fn dashboard_renders_completed_and_failed_package_tasks() {
        let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
        let mut app = App::new(10, 1_000);
        app.completed_tasks.push_back(yoctui_model::CompletedTask {
            task: yoctui_model::TaskInfo {
                id: yoctui_model::TaskId("busybox:do_compile".into()),
                recipe: "busybox".into(),
                task: "do_compile".into(),
                progress: Some(100),
                ..yoctui_model::TaskInfo::default()
            },
            success: true,
        });
        app.completed_tasks.push_back(yoctui_model::CompletedTask {
            task: yoctui_model::TaskInfo {
                id: yoctui_model::TaskId("bash:do_install".into()),
                recipe: "bash".into(),
                task: "do_install".into(),
                progress: Some(100),
                ..yoctui_model::TaskInfo::default()
            },
            success: false,
        });
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("busybox:do_compile 100% complete"));
        assert!(output.contains("bash:do_install 100% failed"));
    }
    #[test]
    fn renders_build_target_editor() {
        let mut terminal = Terminal::new(TestBackend::new(140, 30)).unwrap();
        let mut app = App::new(10, 1_000);
        app.dialogs.push_back(Dialog::BuildTarget {
            input: "core-image-minimal".into(),
            task: None,
        });
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Build target"));
        assert!(output.contains("core-image-minimal"));
    }
    #[test]
    fn renders_machine_aware_build_options() {
        let mut terminal = Terminal::new(TestBackend::new(140, 30)).unwrap();
        let mut app = App::new(10, 1_000);
        app.dialogs.push_back(Dialog::BuildOptions);
        app.build.target = Some("core-image-minimal".into());
        app.workspace
            .variables
            .insert("MACHINE".into(), "qemuarm".into());
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Image build options"));
        assert!(output.contains("qemuarm"));
        assert!(output.contains("Clean image"));
    }
    #[test]
    fn logs_identify_evicted_warnings_and_errors() {
        let mut terminal = Terminal::new(TestBackend::new(300, 30)).unwrap();
        let mut app = App::new(1, 1_000);
        app.screen = Screen::Logs;
        app.logs.dropped = 3;
        app.logs.dropped_warnings = 1;
        app.logs.dropped_errors = 2;
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("3 evicted [W 1 E 2]"));
    }
    #[test]
    fn renders_recipe_task_confirmation() {
        let mut terminal = Terminal::new(TestBackend::new(140, 30)).unwrap();
        let mut app = App::new(10, 1_000);
        app.dialogs
            .push_back(Dialog::RecipeTaskConfirmation(BuildRequest {
                targets: vec!["busybox".into()],
                task: Some("cleansstate".into()),
                force: false,
            }));
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Confirm recipe task"));
        assert!(output.contains("cleansstate"));
    }
    #[test]
    fn devtool_target_reset_renders_exact_destructive_identity_confirmation() {
        let mut terminal = Terminal::new(TestBackend::new(120, 25)).unwrap();
        let mut app = App::new(10, 1_000);
        app.dialogs.push_back(Dialog::DevtoolResetConfirmation(
            yoctui_model::DevtoolResetPlan {
                identity: yoctui_model::RecipeIdentity {
                    name: "busybox".into(),
                    file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
                },
                source_path: "/build/workspace/sources/busybox".into(),
            },
        ));
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Confirm Devtool reset"));
        assert!(output.contains("devtool reset busybox"));
        assert!(output.contains("busybox.bb"));
        assert!(output.contains("/build/workspace/sources/busybox"));
    }
    #[test]
    fn devtool_publish_update_renders_exact_identity_confirmation() {
        let mut terminal = Terminal::new(TestBackend::new(120, 25)).unwrap();
        let mut app = App::new(10, 1_000);
        app.dialogs.push_back(Dialog::DevtoolUpdateConfirmation(
            yoctui_model::RecipeIdentity {
                name: "busybox".into(),
                file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
            },
        ));
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Confirm Devtool update-recipe"));
        assert!(output.contains("devtool update-recipe busybox"));
        assert!(output.contains("busybox.bb"));
    }
    #[test]
    fn devtool_publish_finish_renders_configured_picker_and_exact_confirmation() {
        let mut terminal = Terminal::new(TestBackend::new(120, 25)).unwrap();
        let mut app = App::new(10, 1_000);
        let identity = yoctui_model::RecipeIdentity {
            name: "busybox".into(),
            file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
        };
        let layer = yoctui_model::Layer {
            name: "meta-demo".into(),
            path: "/layers/meta-demo".into(),
            priority: Some(7),
        };
        app.dialogs.push_back(Dialog::DevtoolFinishPicker(
            yoctui_model::DevtoolFinishPicker {
                identity: identity.clone(),
                layers: vec![layer.clone()],
                selection: 0,
            },
        ));
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Configured layer"));
        assert!(output.contains("meta-demo"));
        assert!(output.contains("/layers/meta-demo"));

        app.dialogs.clear();
        app.dialogs.push_back(Dialog::DevtoolFinishConfirmation(
            yoctui_model::DevtoolFinishPlan { identity, layer },
        ));
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Confirm Devtool finish"));
        assert!(output.contains("devtool finish busybox /layers/meta-demo"));
        assert!(output.contains("busybox.bb"));
        assert!(output.contains("Configured layer: meta-demo"));
    }
    #[test]
    fn devtool_target_deploy_renders_identity_entry_and_exact_confirmation() {
        let mut terminal = Terminal::new(TestBackend::new(120, 25)).unwrap();
        let mut app = App::new(10, 1_000);
        let identity = yoctui_model::RecipeIdentity {
            name: "busybox".into(),
            file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
        };
        app.dialogs
            .push_back(Dialog::DevtoolDeploy(yoctui_model::DevtoolDeployDraft {
                identity: identity.clone(),
                target: "qemuarm".into(),
            }));
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Devtool deploy target"));
        assert!(output.contains("busybox.bb"));
        assert!(output.contains("qemuarm"));

        app.dialogs.clear();
        app.dialogs.push_back(Dialog::DevtoolDeployConfirmation(
            yoctui_model::DevtoolDeployPlan {
                identity,
                target: "qemuarm".into(),
            },
        ));
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Confirm Devtool deploy-target"));
        assert!(output.contains("devtool deploy-target busybox qemuarm"));
        assert!(output.contains("busybox.bb"));
    }
    #[test]
    fn devtool_modify_renders_confirmation_and_workspace_editor_build_shortcut() {
        let mut confirmation = App::new(10, 1_000);
        confirmation
            .dialogs
            .push_back(Dialog::DevtoolModifyConfirmation(
                yoctui_model::RecipeIdentity {
                    name: "busybox".into(),
                    file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
                },
            ));
        let output = rendered_text(&confirmation, 120, 30);
        assert!(output.contains("Confirm Devtool modify"), "{output}");
        assert!(output.contains("devtool modify busybox"), "{output}");
        assert!(output.contains("busybox.bb"), "{output}");

        let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
        let mut app = App::new(10, 1_000);
        app.dialogs.push_back(Dialog::RecipeEditor(RecipeEditor {
            recipe: "busybox".into(),
            root: "/build/workspace/sources/busybox".into(),
            files: vec!["main.c".into()],
            selection: 0,
            content: "int main() {}".into(),
            editing: false,
            dirty: false,
        }));
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Workspace file tree: busybox"));
        assert!(output.contains("int main() {}"));
        assert!(output.contains("Ctrl+B build recipe"));
    }
    #[test]
    fn layer_tree_renders_configured_layers_git_state_and_numbered_preview() {
        let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Layers;
        app.workspace.layers.push(yoctui_model::Layer {
            name: "meta-demo".into(),
            path: "/layers/meta-demo".into(),
            priority: Some(7),
        });
        app.layer_relationships = Some(yoctui_model::LayerRelationships {
            layers: vec![yoctui_model::LayerRelationship {
                name: "meta-demo".into(),
                compatible: vec!["scarthgap".into()],
                ..yoctui_model::LayerRelationship::default()
            }],
        });
        let mut browser = LayerBrowser::new("meta-demo".into(), "/layers/meta-demo".into());
        browser.entries = vec![yoctui_model::LayerBrowserEntry {
            path: "/layers/meta-demo/conf/layer.conf".into(),
            is_dir: false,
            size: Some(31),
            git: yoctui_model::GitFileState::Modified,
            ..yoctui_model::LayerBrowserEntry::default()
        }];
        browser.preview = "BBFILE_COLLECTIONS += \\\"demo\\\"".into();
        browser.preview_kind = yoctui_model::PreviewKind::Text;
        app.layer_browser = Some(browser);
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Configured layers"));
        assert!(output.contains("meta-demo"));
        assert!(output.contains("hidden off"));
        assert!(output.contains("layer.conf"));
        assert!(output.contains("BBFILE_COLLECTIONS"));
        assert!(output.contains("M"));
        assert!(output.contains("1"));
    }

    #[test]
    fn layer_tree_binary_preview_and_responsive_modes_never_render_bytes() {
        for (width, height) in [(160, 30), (110, 28), (90, 25), (70, 20)] {
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            let mut app = App::new(10, 1_000);
            app.screen = Screen::Layers;
            let mut browser = LayerBrowser::new("meta-binary".into(), "/layers/meta-binary".into());
            browser.entries.push(yoctui_model::LayerBrowserEntry {
                path: "/layers/meta-binary/image.bin".into(),
                size: Some(100_000),
                git: yoctui_model::GitFileState::Unavailable,
                ..yoctui_model::LayerBrowserEntry::default()
            });
            browser.preview = "\0secret".into();
            browser.preview_kind = yoctui_model::PreviewKind::Binary;
            browser.preview_truncated = true;
            app.layer_browser = Some(browser);
            terminal.draw(|frame| render(frame, &app)).unwrap();
            let output = terminal
                .backend()
                .buffer()
                .content
                .iter()
                .map(|cell| cell.symbol())
                .collect::<String>();
            assert!(!output.contains("secret"));
            if width >= 80 && height >= 24 {
                assert!(output.contains("Binary preview unavailable"));
            }
        }
    }
    #[test]
    fn bitbake_preview_highlights_assignments_and_comments() {
        let app = App::new(10, 1_000);
        let preview = source_preview("SUMMARY = \"demo\" # explanation", "demo.bb", &app);
        assert_eq!(preview.lines[0].spans[0].style.fg, Some(Color::Yellow));
        assert_eq!(preview.lines[0].spans[1].style.fg, Some(Color::Magenta));
        assert_eq!(preview.lines[0].spans[2].style.fg, Some(Color::Green));
        assert_eq!(preview.lines[0].spans[3].style.fg, Some(Color::DarkGray));
    }
    #[test]
    fn renders_image_picker_for_active_machine() {
        let mut terminal = Terminal::new(TestBackend::new(100, 25)).unwrap();
        let mut app = App::new(10, 1_000);
        app.workspace
            .variables
            .insert("MACHINE".into(), "qemux86-64".into());
        app.dialogs
            .push_back(Dialog::ImagePicker(yoctui_model::ImagePicker {
                images: vec!["core-image-minimal".into()],
                selection: 0,
            }));
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Available image targets"));
        assert!(output.contains("qemux86-64"));
        assert!(output.contains("core-image-minimal"));
    }
    #[test]
    fn inspector_reflects_selected_recipe_and_layer_preview() {
        let mut terminal = Terminal::new(TestBackend::new(160, 30)).unwrap();
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Recipes;
        app.workspace.recipes.push(yoctui_model::Recipe {
            name: "busybox".into(),
            version: Some("1.36".into()),
            layer: Some("meta".into()),
            ..yoctui_model::Recipe::default()
        });
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Recipe: busybox"));
        assert!(output.contains("Resolved version: 1.36"));

        app.screen = Screen::Layers;
        let mut browser = LayerBrowser::new("meta".into(), "/layers/meta".into());
        browser.directory = "/layers/meta/conf".into();
        browser.entries.push(yoctui_model::LayerBrowserEntry {
            path: "/layers/meta/conf/layer.conf".into(),
            ..yoctui_model::LayerBrowserEntry::default()
        });
        browser.preview = "BBFILE_COLLECTIONS += \"meta\"".into();
        browser.preview_kind = yoctui_model::PreviewKind::Text;
        app.layer_browser = Some(browser);
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Path: /layers/meta/conf/layer.conf"));
        assert!(output.contains("BBFILE_COLLECTIONS"));
    }
    #[test]
    fn recipes_workspace_renders_authoritative_summary_and_inspector_sections() {
        let mut terminal = Terminal::new(TestBackend::new(180, 44)).unwrap();
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Recipes;
        app.workspace.recipes = vec![
            yoctui_model::Recipe {
                name: "alpha".into(),
                ..yoctui_model::Recipe::default()
            },
            yoctui_model::Recipe {
                name: "busybox".into(),
                version: Some("1.36".into()),
                preferred_version: Some("1.36%".into()),
                layer: Some("core".into()),
                file: Some("/layers/meta/recipes-core/busybox/busybox_1.36.bb".into()),
                append_count: Some(2),
            },
        ];
        app.recipe_selection = 1;
        app.metadata_query = "busy".into();
        app.recipe_metadata.insert(
            "busybox".into(),
            yoctui_model::RecipeMetadata {
                recipe: "busybox".into(),
                workspace_status: Some(yoctui_model::RecipeWorkspaceStatus::Modified),
                build_status: None,
                tasks: Some(vec!["do_build".into(), "do_compile".into()]),
                sources: Some(vec![
                    "/layers/meta/recipes-core/busybox/busybox_1.36.bb".into(),
                ]),
                patches: Some(vec!["file://security.patch".into()]),
                packages: Some(vec!["busybox".into(), "busybox-src".into()]),
                history: None,
            },
        );
        let identity = yoctui_model::RecipeIdentity {
            name: "busybox".into(),
            file: "/layers/meta/recipes-core/busybox/busybox_1.36.bb".into(),
        };
        app.devtool_statuses.insert(
            identity.clone(),
            yoctui_model::DevtoolStatus {
                identity,
                capability: yoctui_model::DevtoolCapability::Available,
                workspace: yoctui_model::DevtoolWorkspace::Present {
                    source_path: "/build/workspace/sources/busybox".into(),
                    recipe_file: Some("/layers/meta/recipes-core/busybox/busybox_1.36.bb".into()),
                },
                git: yoctui_model::DevtoolGitState::Available {
                    branch: Some("devtool".into()),
                    head: Some("abc123".into()),
                    modified: 1,
                    untracked: 0,
                    conflicted: 0,
                },
                error: None,
            },
        );
        app.dependencies = Some(yoctui_model::RecipeDependencies {
            recipe: "busybox".into(),
            build: vec!["virtual/libc".into()],
            runtime: vec!["busybox-udhcpc".into()],
        });
        app.tasks.insert(
            yoctui_model::TaskId("busybox:do_compile".into()),
            yoctui_model::TaskInfo {
                id: yoctui_model::TaskId("busybox:do_compile".into()),
                recipe: "busybox".into(),
                task: "do_compile".into(),
                state: TaskState::Active,
                ..yoctui_model::TaskInfo::default()
            },
        );
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Recipes (shown: 1 of 2)"));
        assert!(output.contains("Resolved"));
        assert!(output.contains("Preferred"));
        assert!(output.contains("Provider file"));
        assert!(output.contains("Workspace/Devtool: member at"));
        assert!(output.contains("Git branch devtool"));
        assert!(output.contains("Active tasks: do_compile"));
        assert!(output.contains("virtual/libc"));
        assert!(output.contains("security.patch"));
        assert!(output.contains("busybox-src"));
        assert!(output.contains("History: unavailable"));
    }

    #[test]
    fn recipes_workspace_partial_failure_and_all_responsive_modes_are_safe() {
        for (width, height) in [(160, 30), (110, 28), (90, 25), (70, 20)] {
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            let mut app = App::new(10, 1_000);
            app.screen = Screen::Recipes;
            app.workspace.recipes.push(yoctui_model::Recipe {
                name: "demo".into(),
                ..yoctui_model::Recipe::default()
            });
            app.recipe_metadata_errors
                .insert("demo".into(), "metadata service unavailable".into());
            terminal.draw(|frame| render(frame, &app)).unwrap();
            if width >= 80 && height >= 24 {
                let output = terminal
                    .backend()
                    .buffer()
                    .content
                    .iter()
                    .map(|cell| cell.symbol())
                    .collect::<String>();
                assert!(output.contains("demo"));
                assert!(output.contains("unavailable"));
            }
        }
    }
    #[test]
    fn recipe_bitbake_action_renders_task_picker_and_exact_forced_confirmation() {
        for (width, height) in [(120, 30), (90, 25)] {
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            let mut app = App::new(10, 1_000);
            app.dialogs
                .push_back(Dialog::RecipeTaskPicker(yoctui_model::RecipeTaskPicker {
                    recipe: "busybox".into(),
                    tasks: vec!["clean".into(), "compile".into(), "devshell".into()],
                    selection: 1,
                    force: true,
                }));
            terminal.draw(|frame| render(frame, &app)).unwrap();
            let output = terminal
                .backend()
                .buffer()
                .content
                .iter()
                .map(|cell| cell.symbol())
                .collect::<String>();
            assert!(output.contains("Force task: busybox"));
            assert!(output.contains("Authoritative BitBake tasks"));
            assert!(output.contains("compile"));

            app.dialogs.clear();
            app.dialogs
                .push_back(Dialog::RecipeTaskConfirmation(BuildRequest {
                    targets: vec!["busybox".into()],
                    task: Some("compile".into()),
                    force: true,
                }));
            terminal.draw(|frame| render(frame, &app)).unwrap();
            let output = terminal
                .backend()
                .buffer()
                .content
                .iter()
                .map(|cell| cell.symbol())
                .collect::<String>();
            assert!(output.contains("bitbake -f busybox -c compile"));
        }
    }
    #[test]
    fn recipe_navigation_renders_log_patch_pickers_and_disabled_reasons_responsively() {
        for (width, height) in [(160, 32), (110, 28), (90, 25)] {
            let mut app = App::new(10, 1_000);
            app.dialogs.push_back(Dialog::RecipeTaskLogPicker(
                yoctui_model::RecipeTaskLogPicker {
                    recipe: "busybox".into(),
                    logs: vec![
                        yoctui_model::RecipeTaskLogChoice {
                            task: "do_compile".into(),
                            state: TaskState::Failed,
                            path: "/tmp/log.do_compile".into(),
                        },
                        yoctui_model::RecipeTaskLogChoice {
                            task: "do_install".into(),
                            state: TaskState::Completed,
                            path: "/tmp/log.do_install".into(),
                        },
                    ],
                    selection: 1,
                },
            ));
            let output = rendered_text(&app, width, height);
            assert!(output.contains("busybox retained task logs"), "{output}");
            assert!(output.contains("/tmp/log.do_install"), "{output}");

            app.dialogs.clear();
            app.dialogs
                .push_back(Dialog::RecipePatchPicker(yoctui_model::RecipePatchPicker {
                    recipe: "busybox".into(),
                    patches: vec!["/layers/meta/recipes-core/busybox/files/fix.patch".into()],
                    selection: 0,
                }));
            let output = rendered_text(&app, width, height);
            assert!(output.contains("busybox patch review"), "{output}");
            assert!(output.contains("fix.patch"), "{output}");
        }

        let mut app = App::new(10, 1_000);
        app.screen = Screen::Recipes;
        app.workspace.recipes.push(yoctui_model::Recipe {
            name: "demo".into(),
            ..yoctui_model::Recipe::default()
        });
        app.recipe_metadata.insert(
            "demo".into(),
            yoctui_model::RecipeMetadata {
                recipe: "demo".into(),
                patches: Some(vec!["file://unresolved.patch".into()]),
                ..yoctui_model::RecipeMetadata::default()
            },
        );
        let output = rendered_text(&app, 160, 36);
        assert!(
            output.contains("provider unavailable (provider path not reported)"),
            "{output}"
        );
        assert!(
            output.contains("patches unavailable (remote or")
                && output.contains("unresolved paths)"),
            "{output}"
        );
        assert!(
            output.contains("Devtool availability is authoritative"),
            "{output}"
        );
    }

    #[test]
    fn devtool_metadata_renders_typed_partial_and_disabled_states_responsively() {
        for (width, height) in [(160, 34), (110, 28), (90, 25)] {
            let mut app = App::new(10, 1_000);
            app.screen = Screen::Recipes;
            let file = std::path::PathBuf::from("/layers/core/demo.bb");
            app.workspace.recipes.push(yoctui_model::Recipe {
                name: "demo".into(),
                file: Some(file.clone()),
                ..yoctui_model::Recipe::default()
            });
            let identity = yoctui_model::RecipeIdentity {
                name: "demo".into(),
                file,
            };
            app.devtool_statuses.insert(
                identity.clone(),
                yoctui_model::DevtoolStatus {
                    identity,
                    capability: yoctui_model::DevtoolCapability::Available,
                    workspace: yoctui_model::DevtoolWorkspace::NotMember,
                    git: yoctui_model::DevtoolGitState::NotApplicable,
                    error: None,
                },
            );
            let output = rendered_text(&app, width, height);
            if width >= 80 && height >= 24 {
                assert!(output.contains("not in workspace"), "{output}");
                assert!(output.contains("u update: disabled"), "{output}");
            }
        }
    }
    #[test]
    fn devtool_job_lifecycle_renders_retained_stream_and_outcome() {
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Recipes;
        app.workspace.recipes.push(yoctui_model::Recipe {
            name: "demo".into(),
            ..yoctui_model::Recipe::default()
        });
        let id = yoctui_model::BackgroundJobId(1_u64 << 63);
        let _ = update(
            &mut app,
            Action::QueueBackgroundJob(yoctui_model::BackgroundJobSpec {
                id,
                kind: BackgroundJobKind::Devtool,
                title: "Devtool modify demo".into(),
                context: yoctui_model::BackgroundJobContext {
                    workspace: Some(Screen::Recipes),
                    recipe: Some("demo".into()),
                    ..yoctui_model::BackgroundJobContext::default()
                },
                cancellation_supported: true,
                queued_at: UNIX_EPOCH,
            }),
        );
        let _ = update(
            &mut app,
            Action::StartBackgroundJob {
                id,
                started_at: UNIX_EPOCH,
            },
        );
        let _ = update(&mut app, Action::RunBackgroundJob { id });
        let _ = update(
            &mut app,
            Action::AppendBackgroundJobOutput {
                id,
                entry: yoctui_model::BackgroundJobOutputEntry {
                    severity: Severity::Info,
                    message: "workspace prepared".into(),
                    source: yoctui_model::BackgroundJobOutputSource::Stderr,
                    truncated: true,
                    timestamp: UNIX_EPOCH,
                },
            },
        );
        let _ = update(
            &mut app,
            Action::SucceedBackgroundJob {
                id,
                result: yoctui_model::BackgroundJobResult {
                    summary: "Devtool completed successfully".into(),
                    artifacts: vec![],
                },
                finished_at: UNIX_EPOCH,
            },
        );

        let output = rendered_text(&app, 200, 44);
        assert!(
            output.contains("Devtool modify demo [Succeeded]"),
            "{output}"
        );
        assert!(
            output.contains("Stderr: workspace prepared [truncated]"),
            "{output}"
        );
        assert!(
            output.contains("outcome: Devtool completed") && output.contains("successfully."),
            "{output}"
        );
    }
    #[test]
    fn recipe_qa_action_renders_capabilities_confirmation_and_honest_results() {
        for (width, height) in [(160, 32), (110, 28), (90, 25)] {
            let mut app = App::new(10, 1_000);
            app.dialogs
                .push_back(Dialog::RecipeTaskConfirmation(BuildRequest {
                    targets: vec!["busybox".into()],
                    task: Some("cve_check".into()),
                    force: false,
                }));
            let output = rendered_text(&app, width, height);
            assert!(output.contains("bitbake busybox -c cve_check"), "{output}");
        }

        let mut app = App::new(20, 4_000);
        app.screen = Screen::Recipes;
        app.workspace.recipes.push(yoctui_model::Recipe {
            name: "busybox".into(),
            ..yoctui_model::Recipe::default()
        });
        app.recipe_metadata.insert(
            "busybox".into(),
            yoctui_model::RecipeMetadata {
                recipe: "busybox".into(),
                tasks: Some(vec!["do_cve_check".into(), "do_create_spdx".into()]),
                ..yoctui_model::RecipeMetadata::default()
            },
        );
        let id = yoctui_model::BackgroundJobId(7);
        let _ = update(
            &mut app,
            Action::QueueBackgroundJob(yoctui_model::BackgroundJobSpec {
                id,
                kind: BackgroundJobKind::Spdx,
                title: "SPDX generation busybox".into(),
                context: yoctui_model::BackgroundJobContext {
                    workspace: Some(Screen::Recipes),
                    recipe: Some("busybox".into()),
                    task: Some("create_spdx".into()),
                    ..yoctui_model::BackgroundJobContext::default()
                },
                cancellation_supported: true,
                queued_at: SystemTime::UNIX_EPOCH,
            }),
        );
        let _ = update(
            &mut app,
            Action::StartBackgroundJob {
                id,
                started_at: SystemTime::UNIX_EPOCH,
            },
        );
        let _ = update(&mut app, Action::RunBackgroundJob { id });
        let _ = update(
            &mut app,
            Action::SucceedBackgroundJob {
                id,
                result: yoctui_model::BackgroundJobResult {
                    summary: "SPDX generation completed; BitBake reported no result path".into(),
                    artifacts: vec![],
                },
                finished_at: SystemTime::UNIX_EPOCH,
            },
        );
        let output = rendered_text(&app, 200, 40);
        assert!(output.contains("QA actions: CVE check enabled"), "{output}");
        assert!(output.contains("SPDX generation enabled"), "{output}");
        assert!(
            output.contains("SPDX generation busybox [Succeeded]"),
            "{output}"
        );
        assert!(
            output.contains("artifacts: none") && output.contains("reported."),
            "{output}"
        );
        assert!(output.contains("V CVE"), "{output}");
        assert!(output.contains("X SPDX"), "{output}");

        app.recipe_metadata.get_mut("busybox").unwrap().tasks = Some(vec![]);
        let output = rendered_text(&app, 200, 40);
        assert!(
            output.contains("CVE check unavailable")
                && output.contains("do_cve_check not reported"),
            "{output}"
        );
        assert!(
            output.contains("SPDX generation unavailable")
                && output.contains("do_create_spdx not reported"),
            "{output}"
        );
    }
    #[test]
    fn build_completion_is_modal_but_running_builds_keep_the_shell_visible() {
        let mut terminal = Terminal::new(TestBackend::new(100, 25)).unwrap();
        let mut app = App::new(10, 1_000);
        app.build.target = Some("core-image-minimal".into());
        app.build.status = yoctui_model::BuildStatus::Running;
        app.host_telemetry.cpu_utilization_percent = Some(50);
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Dashboard"));
        assert!(output.contains("Host CPU: 50%"));

        app.build.status = yoctui_model::BuildStatus::Completed;
        app.dialogs.push_back(Dialog::BuildCompletion);
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Build finished"));
        assert!(output.contains("Press any key"));
    }
    #[test]
    fn build_cancellation_completion_is_distinct_from_failure() {
        let mut terminal = Terminal::new(TestBackend::new(100, 25)).unwrap();
        let mut app = App::new(10, 1_000);
        app.build.target = Some("core-image-minimal".into());
        app.build.status = yoctui_model::BuildStatus::Cancelled;
        app.dialogs.push_back(Dialog::BuildCompletion);
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Build was cancelled"));
        assert!(!output.contains("Build failed"));
    }
    #[test]
    fn configuration_renders_bridge_provenance() {
        let mut terminal = Terminal::new(TestBackend::new(100, 25)).unwrap();
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Configuration;
        app.workspace
            .variables
            .insert("MACHINE".into(), "qemuarm".into());
        app.workspace
            .variable_provenance
            .insert("MACHINE".into(), "conf/local.conf:12".into());
        app.workspace.variable_provenance_chain.insert(
            "MACHINE".into(),
            vec![
                "meta/conf/bitbake.conf:1".into(),
                "conf/local.conf:12".into(),
            ],
        );
        let identity = yoctui_model::VariableIdentity {
            name: "MACHINE".into(),
            recipe: None,
        };
        app.variable_details.insert(
            identity.clone(),
            yoctui_model::VariableDetail {
                identity,
                effective_value: Some("qemuarm".into()),
                unexpanded_value: Some("${DEFAULT_MACHINE}".into()),
                provenance: Some("conf/local.conf:12".into()),
                operations: vec![
                    yoctui_model::VariableOperation {
                        operation: "set".into(),
                        file: Some("meta/conf/bitbake.conf".into()),
                        line: Some(1),
                        value: Some("${DEFAULT_MACHINE}".into()),
                    },
                    yoctui_model::VariableOperation {
                        operation: "set".into(),
                        file: Some("conf/local.conf".into()),
                        line: Some(12),
                        value: Some("qemuarm".into()),
                    },
                ],
                active_overrides: vec!["qemuarm".into(), "poky".into()],
            },
        );
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("conf/local.conf:12"));
        assert!(output.contains("meta/conf/bitbake.conf:1"));
    }

    #[test]
    fn config_workspace_renders_lazy_partial_and_error_states_responsively() {
        for (width, height) in [(160, 30), (110, 26), (90, 24), (70, 20)] {
            let mut app = App::new(10, 1_000);
            app.screen = Screen::Configuration;
            app.workspace
                .variables
                .insert("MACHINE".into(), "qemux86-64".into());
            let identity = yoctui_model::VariableIdentity {
                name: "MACHINE".into(),
                recipe: None,
            };
            app.variable_detail_loading.insert(identity.clone());
            let output = rendered_text(&app, width, height);
            if width >= 80 && height >= 24 {
                assert!(output.contains("Loading authoritative detail"), "{output}");
            }

            app.variable_detail_loading.clear();
            app.variable_detail_errors
                .insert(identity, "Tinfoil unavailable".into());
            let output = rendered_text(&app, width, height);
            if width >= 80 && height >= 24 {
                assert!(output.contains("Detail unavailable"), "{output}");
                assert!(output.contains("Tinfoil unavailable"), "{output}");
            }

            app.variable_detail_errors.clear();
            let identity = yoctui_model::VariableIdentity {
                name: "MACHINE".into(),
                recipe: None,
            };
            app.variable_details.insert(
                identity.clone(),
                yoctui_model::VariableDetail {
                    identity,
                    effective_value: Some("qemux86-64".into()),
                    unexpanded_value: None,
                    provenance: None,
                    operations: vec![],
                    active_overrides: vec![],
                },
            );
            let output = rendered_text(&app, width, height);
            if width >= 110 && height >= 24 {
                assert!(output.contains("Unexpanded value: unavailable"), "{output}");
                assert!(output.contains("Operations:"), "{output}");
                assert!(output.contains("none reported"), "{output}");
            }
        }
    }

    #[test]
    fn config_copy_renders_shortcuts_and_exact_availability_responsively() {
        for (width, height) in [(160, 32), (110, 28), (90, 24)] {
            let mut app = App::new(10, 1_000);
            app.screen = Screen::Configuration;
            app.workspace
                .variables
                .insert("MACHINE".into(), "qemux86-64".into());
            let identity = yoctui_model::VariableIdentity {
                name: "MACHINE".into(),
                recipe: None,
            };
            app.variable_details.insert(
                identity.clone(),
                yoctui_model::VariableDetail {
                    identity,
                    effective_value: Some("qemux86-64".into()),
                    unexpanded_value: None,
                    provenance: None,
                    operations: vec![],
                    active_overrides: vec![],
                },
            );
            let output = rendered_text(&app, width, height);
            if width >= 80 && height >= 24 {
                assert!(output.contains("C effective: enabled"), "{output}");
                assert!(output.contains("U unexpanded: disabled"), "{output}");
            }
        }
    }

    #[test]
    fn config_source_renders_typed_picker_and_disabled_reason_responsively() {
        for (width, height) in [(140, 30), (100, 26), (90, 24)] {
            let mut app = App::new(10, 1_000);
            app.screen = Screen::Configuration;
            app.workspace
                .variables
                .insert("MACHINE".into(), "qemux86-64".into());
            let unloaded = rendered_text(&app, width, height);
            if width >= 80 && height >= 24 {
                assert!(unloaded.contains("o source: disabled"), "{unloaded}");
            }
            app.dialogs.push_back(Dialog::ConfigSourcePicker(
                yoctui_model::ConfigSourcePicker {
                    identity: yoctui_model::VariableIdentity {
                        name: "MACHINE".into(),
                        recipe: None,
                    },
                    sources: vec![
                        yoctui_model::ConfigSourceChoice {
                            operation: "set".into(),
                            path: "meta/conf/bitbake.conf".into(),
                            line: Some(10),
                        },
                        yoctui_model::ConfigSourceChoice {
                            operation: "override".into(),
                            path: "conf/local.conf".into(),
                            line: Some(12),
                        },
                    ],
                    selection: 1,
                },
            ));
            let output = rendered_text(&app, width, height);
            assert!(output.contains("MACHINE defining sources"), "{output}");
            assert!(output.contains("local.conf"), "{output}");
            assert!(output.contains("12"), "{output}");
        }
    }

    #[test]
    fn config_scope_renders_picker_active_identity_and_global_fallback() {
        for (width, height) in [(140, 32), (100, 28), (90, 24)] {
            let mut app = App::new(10, 1_000);
            app.screen = Screen::Configuration;
            app.workspace
                .variables
                .insert("MACHINE".into(), "global-summary".into());
            app.workspace.recipes.push(yoctui_model::Recipe {
                name: "base-files".into(),
                ..yoctui_model::Recipe::default()
            });
            app.config_scope = Some("base-files".into());
            let identity = yoctui_model::VariableIdentity {
                name: "MACHINE".into(),
                recipe: Some("base-files".into()),
            };
            app.variable_detail_errors
                .insert(identity, "scoped Tinfoil failure".into());
            let output = rendered_text(&app, width, height);
            if width >= 80 && height >= 24 {
                assert!(output.contains("scoped Tinfoil failure"), "{output}");
                assert!(output.contains("active base-files"), "{output}");
            }

            app.dialogs
                .push_back(Dialog::ConfigScopePicker(yoctui_model::ConfigScopePicker {
                    variable: "MACHINE".into(),
                    scopes: vec![None, Some("base-files".into())],
                    selection: 1,
                }));
            let output = rendered_text(&app, width, height);
            assert!(output.contains("MACHINE scope"), "{output}");
            assert!(output.contains("(global)"), "{output}");
            assert!(output.contains("base-files"), "{output}");
        }

        let mut app = App::new(10, 1_000);
        app.screen = Screen::Configuration;
        app.workspace
            .variables
            .insert("MACHINE".into(), "qemux86-64".into());
        let output = rendered_text(&app, 120, 28);
        assert!(output.contains("global only"), "{output}");
        assert!(output.contains("no recipes reported"), "{output}");
    }

    #[test]
    fn config_compare_renders_typed_outcomes_and_disabled_reason_responsively() {
        for (width, height) in [(140, 32), (100, 28), (90, 24)] {
            let mut app = App::new(10, 1_000);
            app.screen = Screen::Configuration;
            app.workspace
                .variables
                .insert("MACHINE".into(), "qemux86-64".into());
            app.workspace.recipes.push(yoctui_model::Recipe {
                name: "base-files".into(),
                ..yoctui_model::Recipe::default()
            });
            app.config_scope = Some("base-files".into());
            let unloaded = rendered_text(&app, width, height);
            if width >= 80 && height >= 24 {
                assert!(unloaded.contains("c compare: disabled"), "{unloaded}");
            }
            app.dialogs
                .push_back(Dialog::ConfigComparison(yoctui_model::ConfigComparison {
                    variable: "MACHINE".into(),
                    recipe: "base-files".into(),
                    effective: yoctui_model::ConfigComparisonField {
                        global: Some("qemux86-64".into()),
                        recipe: Some("qemux86-64".into()),
                        outcome: yoctui_model::ConfigComparisonOutcome::Equal,
                    },
                    unexpanded: yoctui_model::ConfigComparisonField {
                        global: Some("${DEFAULT_MACHINE}".into()),
                        recipe: None,
                        outcome: yoctui_model::ConfigComparisonOutcome::Unavailable,
                    },
                }));
            let output = rendered_text(&app, width, height);
            assert!(output.contains("Configuration comparison"), "{output}");
            assert!(output.contains("Effective: Equal"), "{output}");
            assert!(output.contains("Unexpanded: Unavailable"), "{output}");
            assert!(output.contains("base-files"), "{output}");
        }
    }

    #[test]
    fn config_edit_preview_renders_availability_editor_and_exact_confirmation_responsively() {
        for (width, height) in [(140, 32), (100, 28), (90, 24)] {
            let mut app = App::new(10, 1_000);
            app.screen = Screen::Configuration;
            app.workspace.build_dir = Some("/build".into());
            app.workspace
                .variables
                .insert("MACHINE".into(), "qemux86-64".into());
            let identity = yoctui_model::VariableIdentity {
                name: "MACHINE".into(),
                recipe: None,
            };
            app.variable_details.insert(
                identity.clone(),
                yoctui_model::VariableDetail {
                    identity: identity.clone(),
                    effective_value: Some("qemux86-64".into()),
                    unexpanded_value: None,
                    provenance: None,
                    operations: vec![],
                    active_overrides: vec![],
                },
            );

            let output = rendered_text(&app, width, height);
            if width >= 80 && height >= 24 {
                assert!(output.contains("E edit:"), "{output}");
                assert!(output.contains("enabled"), "{output}");
            }

            app.dialogs.push_back(Dialog::ConfigEdit {
                identity: identity.clone(),
                input: "qemux86-64".into(),
            });
            let output = rendered_text(&app, width, height);
            assert!(output.contains("Edit global configuration"), "{output}");
            assert!(output.contains("qemux86-64"), "{output}");

            app.dialogs.pop_back();
            app.dialogs.push_back(Dialog::ConfigEditConfirmation(
                yoctui_model::ConfigEditRequest {
                    identity,
                    value: "qemux86-64".into(),
                    destination: "/build/conf/local.conf".into(),
                    assignment: "MACHINE = \"qemux86-64\"".into(),
                },
            ));
            let output = rendered_text(&app, width, height);
            assert!(output.contains("Preview configuration edit"), "{output}");
            assert!(output.contains("/build/conf/local.conf"), "{output}");
            assert!(output.contains("MACHINE = \"qemux86-64\""), "{output}");
        }

        let mut app = App::new(10, 1_000);
        app.screen = Screen::Configuration;
        app.workspace
            .variables
            .insert("BB_NUMBER_THREADS".into(), "8".into());
        let output = rendered_text(&app, 120, 28);
        assert!(output.contains("E edit: disabled"), "{output}");
        assert!(output.contains("read-only"), "{output}");
    }

    #[test]
    fn bbmask_renders_effective_patterns_and_provenance() {
        let mut terminal = Terminal::new(TestBackend::new(160, 30)).unwrap();
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Bbmask;
        app.workspace
            .variables
            .insert("BBMASK".into(), "meta-broken/.* meta-old/.*".into());
        app.workspace
            .variable_provenance
            .insert("BBMASK".into(), "conf/local.conf:42".into());
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Effective BBMASK"));
        assert!(output.contains("meta-broken/.*"));
        assert!(output.contains("conf/local.conf:42"));
    }
    #[test]
    fn bbmask_edit_preview_shows_the_exact_assignment() {
        let mut terminal = Terminal::new(TestBackend::new(160, 30)).unwrap();
        let mut app = App::new(10, 1_000);
        app.dialogs
            .push_back(Dialog::BbmaskConfirmation("meta-broken/.*".into()));
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Confirm BBMASK change"));
        assert!(output.contains("BBMASK = \"meta-broken/.*\""));
    }

    #[test]
    fn live_tasks_renders_summary_states_filters_and_selected_inspector() {
        let mut app = App::new(20, 2_000);
        app.screen = Screen::Tasks;
        app.build.completed = 2;
        app.build.total = Some(5);
        let mut active = yoctui_model::TaskInfo::active(
            yoctui_model::TaskId("busybox:do_compile".into()),
            "busybox".into(),
            "do_compile".into(),
        );
        active.progress = Some(42);
        active.worker = Some("worker-1".into());
        active.pid = Some(4242);
        active.log_path = Some("/build/tmp/work/busybox/temp/log.do_compile".into());
        app.tasks.insert(active.id.clone(), active);
        let mut failed = yoctui_model::TaskInfo::active(
            yoctui_model::TaskId("openssl:do_install".into()),
            "openssl".into(),
            "do_install".into(),
        );
        failed.state = TaskState::Failed;
        failed.progress = Some(100);
        app.completed_tasks.push_back(yoctui_model::CompletedTask {
            task: failed,
            success: false,
        });
        let output = rendered_text(&app, 180, 34);
        assert!(output.contains("40%  2/5"), "{output}");
        assert!(output.contains("active 1"), "{output}");
        assert!(output.contains("waiting 2"), "{output}");
        assert!(output.contains("FAILED"), "{output}");
        assert!(output.contains("State All"), "{output}");
        assert!(output.contains("PID: 4242"), "{output}");
        assert!(output.contains("log.do_compile"), "{output}");
    }

    #[test]
    fn live_tasks_unknown_progress_and_narrow_layout_are_honest_and_safe() {
        let mut app = App::new(20, 2_000);
        app.screen = Screen::Tasks;
        let task = yoctui_model::TaskInfo::active(
            yoctui_model::TaskId("linux-yocto:do_compile".into()),
            "linux-yocto".into(),
            "do_compile".into(),
        );
        app.tasks.insert(task.id.clone(), task);
        let output = rendered_text(&app, 80, 24);
        assert!(output.contains("progress unknown"), "{output}");
        assert!(!output.contains("0%"), "{output}");
        let _ = rendered_text(&app, 50, 16);
    }

    #[test]
    fn log_workspace_selection_drives_full_multiline_inspector_details() {
        let mut app = App::new(20, 4_000);
        app.screen = Screen::Logs;
        app.logs.insert(yoctui_model::LogEntry {
            id: 0,
            severity: Severity::Error,
            message: "compile failed\ncompiler context line".into(),
            recipe: Some("busybox".into()),
            task: Some("do_compile".into()),
            path: Some("/tmp/log.do_compile".into()),
            timestamp: SystemTime::UNIX_EPOCH,
            build: Some("core-image-minimal".into()),
            protected: true,
            diagnostic: None,
        });
        app.logs.insert(yoctui_model::LogEntry {
            id: 0,
            severity: Severity::Info,
            message: "later output".into(),
            recipe: None,
            task: None,
            path: None,
            timestamp: SystemTime::UNIX_EPOCH,
            build: Some("core-image-minimal".into()),
            protected: false,
            diagnostic: None,
        });
        app.logs.follow = false;
        app.logs.selection = 0;
        let output = rendered_text(&app, 180, 34);
        assert!(output.contains("do_compile"), "{output}");
        assert!(output.contains("Source: /tmp/log.do_compile"), "{output}");
        assert!(output.contains("compiler context line"), "{output}");
        assert!(output.contains("Build: core-image-minimal"), "{output}");
    }

    #[test]
    fn log_workspace_exposes_search_filters_pressure_and_narrow_wrap_safely() {
        let mut app = App::new(20, 4_000);
        app.screen = Screen::Logs;
        app.logs.wrap = true;
        app.logs.searching = true;
        app.logs.query = "needle".into();
        app.logs.recipe_filter = Some("busybox".into());
        app.logs.task_filter = Some("do_compile".into());
        app.logs.build_filter = Some("core-image-minimal".into());
        app.logs.coalesced = 7;
        app.logs.dropped = 3;
        app.logs.dropped_warnings = 0;
        app.logs.dropped_errors = 0;
        app.logs.insert(yoctui_model::LogEntry {
            id: 0,
            severity: Severity::Info,
            message: "needle in a long wrapped line".into(),
            recipe: Some("busybox".into()),
            task: Some("do_compile".into()),
            path: None,
            timestamp: SystemTime::UNIX_EPOCH,
            build: Some("core-image-minimal".into()),
            protected: false,
            diagnostic: None,
        });
        let output = rendered_text(&app, 220, 24);
        assert!(output.contains("7 coalesced"), "{output}");
        assert!(output.contains("search: needle_"), "{output}");
        assert!(output.contains("build: core-image-minimal"), "{output}");
        let _ = rendered_text(&app, 50, 16);
    }

    #[test]
    fn error_workspace_renders_structured_columns_inspector_and_related_entries() {
        let mut app = App::new(20, 4_000);
        app.screen = Screen::Errors;
        app.build.target = Some("core-image-minimal".into());
        let mut first = yoctui_model::LogEntry {
            id: 0,
            severity: Severity::Error,
            message: "compile failed\nfull compiler context".into(),
            recipe: Some("busybox".into()),
            task: Some("do_compile".into()),
            path: Some("/tmp/log.do_compile".into()),
            timestamp: SystemTime::UNIX_EPOCH,
            build: None,
            protected: true,
            diagnostic: None,
        };
        first.build = app.build.target.clone();
        app.logs.insert(first);
        app.logs.insert(yoctui_model::LogEntry {
            id: 0,
            severity: Severity::Warning,
            message: "busybox follow-up warning".into(),
            recipe: Some("busybox".into()),
            task: Some("do_package".into()),
            path: None,
            timestamp: SystemTime::UNIX_EPOCH,
            build: Some("core-image-minimal".into()),
            protected: true,
            diagnostic: None,
        });
        let output = rendered_text(&app, 220, 36);
        assert!(output.contains("Time"), "{output}");
        assert!(output.contains("Severity"), "{output}");
        assert!(output.contains("Summary"), "{output}");
        assert!(output.contains("Category: BitBake error"), "{output}");
        assert!(output.contains("full compiler context"), "{output}");
        assert!(output.contains("Suggested actions"), "{output}");
        assert!(output.contains("busybox follow-up warning"), "{output}");
        assert!(output.contains("/tmp/log.do_compile"), "{output}");
    }

    #[test]
    fn error_workspace_and_actionable_failure_completion_are_narrow_safe() {
        let mut app = App::new(20, 4_000);
        app.screen = Screen::Errors;
        app.logs.insert(yoctui_model::LogEntry {
            id: 0,
            severity: Severity::Error,
            message: "backend connection lost".into(),
            recipe: None,
            task: None,
            path: None,
            timestamp: SystemTime::UNIX_EPOCH,
            build: None,
            protected: true,
            diagnostic: None,
        });
        let _ = rendered_text(&app, 50, 16);
        app.build.status = yoctui_model::BuildStatus::Failed;
        app.build.errors = 1;
        app.dialogs.push_back(Dialog::BuildCompletion);
        let output = rendered_text(&app, 100, 24);
        assert!(
            output.contains("Press Enter to investigate Errors"),
            "{output}"
        );
    }

    #[test]
    fn signature_workspace_renders_typed_records_differences_limitations_and_footer() {
        let target = yoctui_model::SignatureTarget {
            recipe: "busybox".into(),
            task: "do_compile".into(),
        };
        let left = yoctui_model::SignatureIdentity {
            target: target.clone(),
            hash: Some("aaa".into()),
            path: Some("/build/tmp/stamps/busybox/do_compile.sigdata.aaa".into()),
        };
        let right = yoctui_model::SignatureIdentity {
            target: target.clone(),
            hash: Some("bbb".into()),
            path: Some("/build/tmp/stamps/busybox/do_compile.sigdata.bbb".into()),
        };
        let mut app = App::new(20, 4_000);
        app.screen = Screen::Signatures;
        app.signature_selection = Some(left.clone());
        app.signature_dump = SignatureDumpState::Partial {
            target,
            records: vec![
                yoctui_model::SignatureRecord {
                    identity: left.clone(),
                    base_hash: Some("base-aaa".into()),
                    task_hash: Some("aaa".into()),
                    variables: vec![yoctui_model::SignatureValue {
                        name: "CC".into(),
                        value: Some("gcc".into()),
                    }],
                    dependencies: vec!["busybox:do_configure=dep-a".into()],
                },
                yoctui_model::SignatureRecord {
                    identity: right.clone(),
                    base_hash: Some("base-bbb".into()),
                    task_hash: Some("bbb".into()),
                    variables: Vec::new(),
                    dependencies: Vec::new(),
                },
            ],
            limitations: vec!["one malformed artifact was omitted".into()],
        };
        app.signature_comparison = SignatureComparisonState::Partial {
            request: yoctui_model::SignatureComparisonRequest { left, right },
            differences: vec![yoctui_model::SignatureDifference {
                category: SignatureDifferenceCategory::ChangedValue,
                key: "CC".into(),
                left: Some("gcc".into()),
                right: Some("clang".into()),
            }],
            limitations: vec!["recursive detail unavailable".into()],
        };

        let wide = rendered_text(&app, 160, 34);
        assert!(wide.contains("Signatures"), "{wide}");
        assert!(wide.contains("busybox:do_compile"), "{wide}");
        assert!(wide.contains("base-aaa"), "{wide}");
        assert!(wide.contains("CC = gcc"), "{wide}");
        assert!(wide.contains("[value] CC: gcc"), "{wide}");
        assert!(wide.contains("one malformed artifact"), "{wide}");
        assert!(wide.contains("recursive detail unavailable"), "{wide}");
        assert!(wide.contains("1/2 sides"), "{wide}");

        let narrow = rendered_text(&app, 90, 30);
        assert!(narrow.contains("Signatures"), "{narrow}");
        assert!(narrow.contains("Selected record"), "{narrow}");
        let tiny = rendered_text(&app, 50, 16);
        assert!(tiny.contains("needs at least 80x24"), "{tiny}");
    }

    #[test]
    fn signature_workspace_renders_explicit_loading_empty_failure_and_picker_states() {
        let target = yoctui_model::SignatureTarget {
            recipe: "busybox".into(),
            task: "do_fetch".into(),
        };
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Signatures;
        app.signature_dump = SignatureDumpState::Loading {
            target: target.clone(),
        };
        assert!(rendered_text(&app, 100, 24).contains("Loading authoritative signature artifacts"));
        app.signature_dump = SignatureDumpState::AvailableEmpty {
            target: target.clone(),
        };
        assert!(rendered_text(&app, 100, 24).contains("no signature artifacts"));
        app.signature_dump = SignatureDumpState::Failed {
            target,
            message: "tool missing".into(),
        };
        assert!(rendered_text(&app, 100, 24).contains("tool missing"));

        app.screen = Screen::Recipes;
        app.dialogs.push_back(Dialog::SignatureTaskPicker(
            yoctui_model::SignatureTaskPicker {
                recipe: RecipeIdentity {
                    name: "busybox".into(),
                    file: "/layers/meta/busybox.bb".into(),
                },
                tasks: vec!["do_fetch".into(), "do_compile".into()],
                selection: 1,
            },
        ));
        let picker = rendered_text(&app, 80, 24);
        assert!(picker.contains("Inspect signatures: busybox"), "{picker}");
        assert!(picker.contains("Authoritative signature tasks"), "{picker}");
    }

    #[test]
    fn pkgdata_workspace_renders_typed_partial_details_footer_and_responsive_modes() {
        let request = yoctui_model::PackageInventoryRequest { generation: 1 };
        let identity = PackageIdentity::new("busybox");
        let package = yoctui_model::PackageSummary {
            identity: identity.clone(),
            recipe: PackageField::Available("busybox".into()),
            provider: PackageField::Available("/layers/meta/recipes-core/busybox.bb".into()),
            version: PackageField::Available("1.37.0-r0".into()),
            installed_size_bytes: PackageField::Available(1_024),
            license: PackageField::Available("GPL-2.0-only".into()),
            image_membership: PackageField::Unavailable,
        };
        let detail_request = yoctui_model::PackageDetailRequest {
            identity: identity.clone(),
            generation: 2,
        };
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Packages;
        app.package_selection = Some(identity.clone());
        app.package_inventory = PackageInventoryState::Partial {
            request,
            packages: vec![package],
            limitations: vec!["image membership unavailable".into()],
        };
        app.package_details.insert(
            identity.clone(),
            PackageDetailState::Partial {
                request: detail_request,
                detail: yoctui_model::PackageDetail {
                    identity,
                    files: PackageField::Available(vec!["/bin/busybox".into()]),
                    runtime_dependencies: PackageField::Available(vec![PackageIdentity::new(
                        "libc6",
                    )]),
                    reverse_dependencies: PackageField::Available(Vec::new()),
                },
                limitations: vec!["reverse scan bounded".into()],
            },
        );

        let wide = rendered_text(&app, 160, 34);
        assert!(wide.contains("Packages"), "{wide}");
        assert!(wide.contains("busybox"), "{wide}");
        assert!(wide.contains("1.37.0-r0"), "{wide}");
        assert!(wide.contains("GPL-2.0-only"), "{wide}");
        assert!(wide.contains("/bin/busybox"), "{wide}");
        assert!(wide.contains("libc6"), "{wide}");
        assert!(wide.contains("Image membership: unavailable"), "{wide}");
        assert!(wide.contains("image membership unavailable"), "{wide}");
        assert!(wide.contains("Enter detail"), "{wide}");

        for (width, height) in [(120, 30), (90, 28)] {
            let output = rendered_text(&app, width, height);
            assert!(output.contains("Packages"), "{width}: {output}");
            assert!(output.contains("busybox"), "{width}: {output}");
        }
        for (theme, color) in [(Theme::Light, true), (Theme::Dark, false)] {
            app.theme = theme;
            app.color_enabled = color;
            let output = rendered_text(&app, 140, 30);
            assert!(output.contains("busybox"), "{output}");
        }
        assert!(rendered_text(&app, 50, 16).contains("needs at least 80x24"));
    }

    #[test]
    fn pkgdata_workspace_renders_loading_empty_failed_and_unavailable_states() {
        let request = yoctui_model::PackageInventoryRequest { generation: 1 };
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Packages;
        app.package_inventory = PackageInventoryState::Loading { request };
        assert!(rendered_text(&app, 100, 25).contains("Loading authoritative package inventory"));
        app.package_inventory = PackageInventoryState::AvailableEmpty { request };
        assert!(rendered_text(&app, 100, 25).contains("No built runtime packages"));
        app.package_inventory = PackageInventoryState::Failed {
            request,
            message: "generated pkgdata is unavailable".into(),
        };
        let failed = rendered_text(&app, 100, 25);
        assert!(
            failed.contains("generated pkgdata is unavailable"),
            "{failed}"
        );
        assert!(failed.contains("do_package"), "{failed}");
    }
}
