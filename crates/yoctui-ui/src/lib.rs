//! Rendering only; no backend parsing or mutation lives in widgets.
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Cell, Clear, Gauge, Paragraph, Row, Table, Wrap},
};
use std::time::{SystemTime, UNIX_EPOCH};
use yoctui_model::{
    App, FocusTarget, LayerBrowser, RecipeEditor, Screen, Severity, Theme, format_duration,
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

fn selected_style(app: &App, active: bool) -> Style {
    if !active {
        Style::default()
    } else if app.color_enabled {
        Style::default()
            .bg(theme_selected_background(app.theme))
            .fg(theme_selected_foreground(app.theme))
    } else {
        Style::default().add_modifier(Modifier::REVERSED)
    }
}

fn theme_selected_background(theme: Theme) -> Color {
    match theme {
        Theme::Dark => Color::DarkGray,
        Theme::Light => Color::Gray,
        Theme::MatrixGreen => Color::Green,
        Theme::HighContrast => Color::White,
        Theme::Monochrome => Color::Reset,
    }
}

fn theme_selected_foreground(theme: Theme) -> Color {
    match theme {
        Theme::MatrixGreen | Theme::HighContrast => Color::Black,
        _ => Color::Reset,
    }
}

fn theme_focus_color(app: &App) -> Color {
    if !app.color_enabled {
        return Color::Reset;
    }
    match app.theme {
        Theme::Dark => Color::Cyan,
        Theme::Light => Color::Blue,
        Theme::MatrixGreen => Color::LightGreen,
        Theme::HighContrast => Color::Yellow,
        Theme::Monochrome => Color::Reset,
    }
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

fn source_preview(content: &str, file_name: &str, color_enabled: bool) -> Text<'static> {
    let bitbake_source = ["bb", "bbappend", "inc", "conf"]
        .iter()
        .any(|extension| file_name.ends_with(&format!(".{extension}")));
    let markdown = file_name.ends_with(".md") || file_name.ends_with(".markdown");
    if !color_enabled || (!bitbake_source && !markdown) {
        return Text::from(content.to_owned());
    }
    Text::from(
        content
            .lines()
            .map(|line| {
                if markdown {
                    let style = if line.starts_with('#') {
                        Style::default().fg(Color::LightBlue)
                    } else if line.starts_with("```") {
                        Style::default().fg(Color::Magenta)
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
                        Style::default().fg(Color::LightBlue),
                    ));
                    spans.push(Span::raw(trimmed[keyword_end..].to_owned()));
                } else if let Some(equals) = trimmed.find('=') {
                    let lhs_end = trimmed[..equals]
                        .trim_end_matches([' ', '?', '+', ':'])
                        .len();
                    spans.push(Span::styled(
                        trimmed[..lhs_end].to_owned(),
                        Style::default().fg(Color::Yellow),
                    ));
                    spans.push(Span::styled(
                        trimmed[lhs_end..=equals].to_owned(),
                        Style::default().fg(Color::Magenta),
                    ));
                    spans.push(Span::styled(
                        trimmed[equals + 1..].to_owned(),
                        Style::default().fg(Color::Green),
                    ));
                } else {
                    spans.push(Span::raw(trimmed.to_owned()));
                }
                if let Some(comment) = comment {
                    spans.push(Span::styled(
                        format!("#{comment}"),
                        Style::default().fg(Color::DarkGray),
                    ));
                }
                Line::from(spans)
            })
            .collect::<Vec<_>>(),
    )
}

fn task_activity(app: &App, task_progress: Option<u8>) -> &'static str {
    if task_progress.is_some() || app.reduced_motion {
        return "";
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
    if app.focus == FocusTarget::Navigator {
        return "j/k or ↑/↓ select | Enter open | Tab workspace | Shift+Tab inspector | q quit";
    }
    if app.focus == FocusTarget::Inspector {
        return "Tab navigator | Shift+Tab workspace | ↑/↓ scroll inspector | / search | q quit";
    }
    if app.layer_browser.is_some() {
        return "↑/↓ select | Enter/→ descend | Esc/← up | r refresh | e edit file | Ctrl+S save | ? help | q quit";
    }
    match app.screen {
        Screen::Dashboard => {
            "F5 build | Ctrl+P commands | Tab focus | ↑/↓ package progress | i image | ! shell | c cancel | r recipes | y layers | ? help | q quit"
        }
        Screen::Tasks => "↑/↓ task progress | c cancel | Tab focus | l logs | e errors | q quit",
        Screen::BuildHistory => "↑/↓ select | Esc dashboard | ? help | q quit",
        Screen::Dependencies => {
            "↑/↓ select | Enter recipe | Esc dashboard | r recipes | ? help | q quit"
        }
        Screen::LayerRelationships => "Esc dashboard | y layers | ? help | q quit",
        Screen::Recipes => {
            "↑/↓ select | b build | C clean | M menuconfig | S cleansstate | g graph | d Devtool edit | u update-recipe | F finish | P deploy | D reset | / search | Esc dashboard | ? help | q quit"
        }
        Screen::Images => {
            "↑/↓ select | b build selected image | i image picker | Tab focus | q quit"
        }
        Screen::Layers => {
            "↑/↓ select | Enter browse | i image | R relationships | e in-TUI edit | o external editor | / search | Esc dashboard | ? help | q quit"
        }
        Screen::Configuration => {
            "↑/↓ select | o open provenance | / search | x BBMASK | Esc dashboard | ? help | q quit"
        }
        Screen::Bbmask => {
            "e edit BBMASK | Enter preview/confirm | Esc cancel/dashboard | v configuration | ? help | q quit"
        }
        Screen::Logs => {
            "↑/↓ scroll | ←/→ horizontal | f follow | w wrap | s severity | R/T filters | / search | Esc dashboard | ? help | q quit"
        }
        Screen::Errors => {
            "↑/↓ select | Enter logs | o open source | Esc dashboard | ? help | q quit"
        }
        Screen::Help => "Esc dashboard | q quit",
        Screen::Settings => "Ctrl+P commands | Tab focus | q quit",
    }
}

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();
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
        .block(Block::default().borders(Borders::ALL)),
        chunks[0],
    );
    responsive_shell(frame, app, chunks[1], area.width);
    let footer_style = if app.color_enabled {
        Style::default().fg(theme_focus_color(app))
    } else {
        Style::default()
    };
    frame.render_widget(
        Paragraph::new(footer_shortcuts(app)).style(footer_style),
        chunks[2],
    );
    if let Some(editor) = app.recipe_editor.as_ref() {
        recipe_editor(frame, app, editor, area);
    } else if app.command_palette_open {
        let commands = [
            "Build image",
            "Open Layers",
            "Open Recipes",
            "Open Logs",
            "Open Errors",
            "Open Help",
        ];
        let items = commands
            .iter()
            .enumerate()
            .map(|(index, command)| {
                format!(
                    "{} {}",
                    if index == app.command_palette_selection {
                        ">"
                    } else {
                        " "
                    },
                    command
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        let popup = Rect::new(
            area.width / 4,
            area.height / 4,
            area.width / 2,
            area.height / 2,
        );
        frame.render_widget(Clear, popup);
        frame.render_widget(
            Paragraph::new(format!(
                "Commands\n\n{items}\n\nUp/Down select  Enter run  Esc cancel"
            ))
            .block(
                Block::default()
                    .title("Command palette")
                    .borders(Borders::ALL),
            ),
            popup,
        );
    } else if app.build_completion_open {
        build_completion_popup(frame, app, area);
    } else if app.quit_confirm {
        let popup = Rect::new(area.width / 4, area.height / 3, area.width / 2, 3);
        frame.render_widget(Clear, popup);
        frame.render_widget(
            Paragraph::new("Build is active. Press Y to quit UI, or Esc to continue.")
                .block(Block::default().title("Confirm quit").borders(Borders::ALL)),
            popup,
        )
    } else if let Some(request) = app.recipe_task_confirmation.as_ref() {
        let popup = Rect::new(area.width / 4, area.height / 3, area.width / 2, 5);
        frame.render_widget(Clear, popup);
        frame.render_widget(
            Paragraph::new(format!(
                "Run `bitbake {} {}`?\n\nPress Enter to continue or Esc to cancel.",
                request
                    .task
                    .as_deref()
                    .map_or(String::new(), |task| format!("-c {task}")),
                request.targets.join(" ")
            ))
            .block(
                Block::default()
                    .title("Confirm recipe task")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: true }),
            popup,
        );
    } else if let Some(recipe) = app.devtool_reset_confirmation.as_deref() {
        let popup = Rect::new(area.width / 4, area.height / 3, area.width / 2, 5);
        frame.render_widget(Clear, popup);
        frame.render_widget(
            Paragraph::new(format!(
                "Run `devtool reset {recipe}`?\n\nThis removes the Devtool workspace. Press Enter to continue or Esc to cancel."
            ))
            .block(
                Block::default()
                    .title("Confirm Devtool reset")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: true }),
            popup,
        );
    } else if let Some(recipe) = app.devtool_update_confirmation.as_deref() {
        let popup = Rect::new(area.width / 4, area.height / 3, area.width / 2, 5);
        frame.render_widget(Clear, popup);
        frame.render_widget(
            Paragraph::new(format!(
                "Run `devtool update-recipe {recipe}`?\n\nThis updates recipe metadata from the Devtool workspace. Press Enter to continue or Esc to cancel."
            ))
            .block(
                Block::default()
                    .title("Confirm Devtool update-recipe")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: true }),
            popup,
        );
    } else if let Some(request) = app.devtool_finish_confirmation.as_ref() {
        let width = area.width.saturating_sub(12).clamp(44, 100);
        let popup = Rect::new(
            (area.width.saturating_sub(width)) / 2,
            area.height.saturating_sub(7) / 2,
            width,
            7,
        );
        frame.render_widget(Clear, popup);
        frame.render_widget(
            Paragraph::new(format!(
                "Run `devtool finish {} {}`?\n\nThis exports Devtool changes into the destination layer.\n\nEnter continues; Esc cancels.",
                request.recipe,
                request.destination.display()
            ))
            .block(
                Block::default()
                    .title("Confirm Devtool finish")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: true }),
            popup,
        );
    } else if let Some(request) = app.devtool_deploy_confirmation.as_ref() {
        let width = area.width.saturating_sub(12).clamp(44, 100);
        let popup = Rect::new(
            (area.width.saturating_sub(width)) / 2,
            area.height.saturating_sub(7) / 2,
            width,
            7,
        );
        frame.render_widget(Clear, popup);
        frame.render_widget(
            Paragraph::new(format!(
                "Run `devtool deploy-target {} {}`?\n\nThis deploys the recipe output to the specified target.\n\nEnter continues; Esc cancels.",
                request.recipe, request.target
            ))
            .block(
                Block::default()
                    .title("Confirm Devtool deploy-target")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: true }),
            popup,
        );
    } else if let Some(recipe) = app.devtool_deploy_recipe.as_deref() {
        let width = area.width.saturating_sub(12).clamp(44, 100);
        let popup = Rect::new(
            (area.width.saturating_sub(width)) / 2,
            area.height.saturating_sub(6) / 2,
            width,
            6,
        );
        frame.render_widget(Clear, popup);
        frame.render_widget(
            Paragraph::new(format!(
                "Recipe: {recipe}\nDeployment target: {}_\n\nEnter previews the command; Esc cancels.",
                app.devtool_deploy_target
            ))
            .block(
                Block::default()
                    .title("Devtool deploy target")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false }),
            popup,
        );
    } else if let Some(recipe) = app.devtool_finish_recipe.as_deref() {
        let width = area.width.saturating_sub(12).clamp(44, 100);
        let popup = Rect::new(
            (area.width.saturating_sub(width)) / 2,
            area.height.saturating_sub(6) / 2,
            width,
            6,
        );
        frame.render_widget(Clear, popup);
        frame.render_widget(
            Paragraph::new(format!(
                "Recipe: {recipe}\nDestination layer: {}_\n\nEnter previews the command; Esc cancels.",
                app.devtool_finish_destination
            ))
            .block(
                Block::default()
                    .title("Devtool finish destination")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false }),
            popup,
        );
    } else if let Some(value) = app.bbmask_confirmation.as_deref() {
        let width = area.width.saturating_sub(12).clamp(40, 96);
        let popup = Rect::new(
            (area.width.saturating_sub(width)) / 2,
            area.height.saturating_sub(7) / 2,
            width,
            7,
        );
        frame.render_widget(Clear, popup);
        frame.render_widget(
            Paragraph::new(format!(
                "Append this exact assignment to $BUILDDIR/conf/local.conf:\n\n{}\n\nEnter writes and refreshes configuration; Esc cancels.",
                bbmask_assignment(value)
            ))
            .block(Block::default().title("Confirm BBMASK change").borders(Borders::ALL))
            .wrap(Wrap { trim: false }),
            popup,
        );
    } else if app.bbmask_editing {
        let width = area.width.saturating_sub(12).clamp(40, 96);
        let popup = Rect::new(
            (area.width.saturating_sub(width)) / 2,
            area.height.saturating_sub(6) / 2,
            width,
            6,
        );
        frame.render_widget(Clear, popup);
        frame.render_widget(
            Paragraph::new(format!(
                "BBMASK: {}_\n\nEnter previews the exact local.conf assignment; Esc cancels.",
                app.bbmask_input
            ))
            .block(
                Block::default()
                    .title("Edit effective BBMASK")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false }),
            popup,
        );
    } else if let Some(picker) = app.image_picker.as_ref() {
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
        frame.render_widget(Clear, popup);
        frame.render_widget(
            Paragraph::new(format!(
                "Active MACHINE: {machine}\n\n{images}\n\nUp/Down select  Enter choose image  Esc cancel"
            ))
            .block(Block::default().title("Available image targets").borders(Borders::ALL))
            .wrap(Wrap { trim: false }),
            popup,
        );
    } else if app.build_options_open {
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
        frame.render_widget(Clear, popup);
        frame.render_widget(
            Paragraph::new(format!(
                "Machine: {machine}\nCurrent image target: {}\n\nb  Build image\nc  Clean image\nm  Run menuconfig\ne  Enter a different image target\n\nEsc closes this menu.",
                app.build.target.as_deref().unwrap_or("not selected")
            ))
            .block(Block::default().title("Image build options").borders(Borders::ALL)),
            popup,
        );
    } else if app.build_target_editing {
        let width = area.width.saturating_sub(12).clamp(30, 80);
        let popup = Rect::new(
            (area.width.saturating_sub(width)) / 2,
            area.height.saturating_sub(5) / 2,
            width,
            5,
        );
        frame.render_widget(Clear, popup);
        frame.render_widget(
            Paragraph::new(format!(
                "Target: {}_\nTask: {}\n\nEnter starts the build; Esc cancels.",
                app.build_target_input,
                app.build_task.as_deref().unwrap_or("default")
            ))
            .block(Block::default().title("Build target").borders(Borders::ALL)),
            popup,
        );
    } else if let Some(notification) = app.notification.as_deref() {
        let width = area.width.saturating_sub(8).clamp(24, 80);
        let popup = Rect::new(
            (area.width.saturating_sub(width)) / 2,
            area.height.saturating_sub(5) / 2,
            width,
            5,
        );
        frame.render_widget(Clear, popup);
        frame.render_widget(
            Paragraph::new(format!("{notification}\n\nPress Enter to dismiss."))
                .block(Block::default().title("Notice").borders(Borders::ALL))
                .wrap(Wrap { trim: true }),
            popup,
        );
    }
}

fn responsive_shell(frame: &mut Frame, app: &App, area: Rect, terminal_width: u16) {
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
        .style(if app.color_enabled {
            Style::default().fg(theme_focus_color(app))
        } else {
            Style::default()
        }),
        area,
    );
}

fn pane_block<'a>(app: &App, title: &'a str, focused: bool) -> Block<'a> {
    let style = if focused {
        Style::default().fg(theme_focus_color(app))
    } else {
        Style::default().fg(Color::DarkGray)
    };
    Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(style)
}

fn navigator(frame: &mut Frame, app: &App, area: Rect) {
    let entries = [
        ("Dashboard", Screen::Dashboard),
        ("Layers", Screen::Layers),
        ("Recipes", Screen::Recipes),
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
        Screen::LayerRelationships => layer_relationships(frame, app, area),
        Screen::Logs => logs(frame, app, area),
        Screen::Errors => errors(frame, app, area),
        Screen::Recipes => recipes(frame, app, area),
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
            |recipe| {
                format!(
                    "Recipe: {}\nVersion: {}\nLayer: {}\n\nUse b to build or g for dependencies.",
                    recipe.name,
                    recipe.version.as_deref().unwrap_or("unknown"),
                    recipe.layer.as_deref().unwrap_or("unknown")
                )
            },
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
                format!(
                    "Path: {}\n\n{}",
                    browser.directory.display(),
                    browser.preview
                )
            },
        ),
        Screen::Configuration => app
            .workspace
            .variables
            .iter()
            .nth(app.config_selection)
            .map_or_else(
                || "No configuration variable selected.".into(),
                |(name, value)| {
                    format!(
                        "{name} = {value}\n\n{}",
                        app.workspace
                            .variable_provenance
                            .get(name)
                            .map_or("No provenance available.", String::as_str)
                    )
                },
            ),
        Screen::Logs => app.logs.entries.back().map_or_else(
            || "No logs retained.".into(),
            |entry| format!("{:?}\n{}", entry.severity, entry.message),
        ),
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
    frame.render_widget(Clear, popup);
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
    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(format!(
            "Build {} for {}.\n\nTasks completed: {}\nWarnings: {}    Errors: {}    Exit code: {}\nElapsed: {}\n\nPress any key to return to Yoctui.",
            result,
            app.build.target.as_deref().unwrap_or("unknown target"),
            app.build.completed,
            app.build.warnings,
            app.build.errors,
            app.build.exit_code.map_or_else(|| "unknown".into(), |code| code.to_string()),
            app.elapsed().map(format_duration).unwrap_or_else(|| "unknown".into()),
        ))
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
    frame.render_widget(Clear, popup);
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
        Paragraph::new(source_preview(&content, &selected, app.color_enabled))
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
        Paragraph::new("↑/↓ file  Enter/e edit  Ctrl+S save  Esc return to Yoctui").style(
            if app.color_enabled {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default()
            },
        ),
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
            "Package task progress ({} active, {} complete; use Up/Down to scroll)",
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
            let color = if app.color_enabled {
                if *completed == Some(false) {
                    Color::Red
                } else if progress >= 100 {
                    Color::Green
                } else if progress >= 75 {
                    Color::Yellow
                } else {
                    Color::LightBlue
                }
            } else {
                Color::Reset
            };
            frame.render_widget(
                Gauge::default()
                    .ratio(f64::from(progress) / 100.0)
                    .label(format!(
                        "{}:{}{} {progress}%{}",
                        task.recipe,
                        task.task,
                        task_activity(app, task.progress),
                        match completed {
                            Some(true) => " complete",
                            Some(false) => " failed",
                            None => "",
                        }
                    ))
                    .gauge_style(Style::default().fg(color)),
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
    let mut tasks = app.tasks.values().collect::<Vec<_>>();
    tasks.sort_by(|left, right| {
        (left.recipe.as_str(), left.task.as_str())
            .cmp(&(right.recipe.as_str(), right.task.as_str()))
    });
    let lines = if tasks.is_empty() {
        "Waiting for BitBake task events.".into()
    } else {
        tasks
            .iter()
            .map(|task| {
                format!(
                    "{:<28} {:<24} {}",
                    task.recipe,
                    task.task,
                    task.progress.map_or_else(
                        || task_activity(app, None).into(),
                        |progress| format!("{progress}%")
                    )
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    frame.render_widget(
        Paragraph::new(format!("Overall: {}/{} complete | {} active\n\nRecipe                       Task                     Progress\n{lines}", app.build.completed, app.build.total.map_or_else(|| "?".into(), |total| total.to_string()), tasks.len()))
            .block(Block::default().title("Live Tasks").borders(Borders::ALL))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn images_workspace(frame: &mut Frame, app: &App, area: Rect) {
    let images = app
        .workspace
        .recipes
        .iter()
        .filter(|recipe| recipe.name.contains("image"))
        .collect::<Vec<_>>();
    let text = if images.is_empty() {
        "No image recipes were discovered in the active layers.".into()
    } else {
        images
            .iter()
            .map(|recipe| {
                format!(
                    "{:<36} {:<14} {}",
                    recipe.name,
                    recipe.version.as_deref().unwrap_or("unknown"),
                    recipe.layer.as_deref().unwrap_or("unknown")
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    frame.render_widget(
        Paragraph::new(format!(
            "Image target                         Version        Layer\n{text}"
        ))
        .block(Block::default().title("Images").borders(Borders::ALL))
        .wrap(Wrap { trim: false }),
        area,
    );
}

fn settings_workspace(frame: &mut Frame, app: &App, area: Rect) {
    frame.render_widget(
        Paragraph::new(format!(
            "Theme: {:?}\nColor: {}\nAnimation speed: {:?}\nReduced motion: {}\n\nSettings are loaded from $XDG_CONFIG_HOME/yoctui/config.toml.\nUse Ctrl+P to discover available workspace commands.",
            app.theme,
            if app.color_enabled { "enabled" } else { "disabled" },
            app.animation_speed,
            app.reduced_motion,
        ))
        .block(Block::default().title("Settings").borders(Borders::ALL))
        .wrap(Wrap { trim: false }),
        area,
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

fn dependencies(frame: &mut Frame, app: &App, area: Rect) {
    let Some(dependencies) = app.dependencies.as_ref() else {
        frame.render_widget(
            Paragraph::new("No recipe dependency data is loaded. Select a recipe and press g.")
                .block(
                    Block::default()
                        .title("Dependency graph")
                        .borders(Borders::ALL),
                ),
            area,
        );
        return;
    };
    let rows = dependencies
        .build
        .iter()
        .map(|name| ("build", name.as_str()))
        .chain(
            dependencies
                .runtime
                .iter()
                .map(|name| ("runtime", name.as_str())),
        )
        .collect::<Vec<_>>();
    frame.render_widget(
        Table::new(
            rows.iter().enumerate().map(|(index, (kind, name))| {
                Row::new(vec![Cell::from(*kind), Cell::from(*name)])
                    .style(selected_style(app, index == app.dependency_selection))
            }),
            [Constraint::Length(12), Constraint::Min(1)],
        )
        .header(
            Row::new(["Kind", "Dependency"]).style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(
            Block::default()
                .title(format!(
                    "Dependency graph: {} ({} edges, server supplied)",
                    dependencies.recipe,
                    rows.len()
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
    let mut visible = app
        .logs
        .filtered()
        .filter(|l| {
            app.screen != Screen::Errors
                || matches!(l.severity, Severity::Warning | Severity::Error)
        })
        .collect::<Vec<_>>();
    let height = area.height.saturating_sub(3) as usize;
    let end = visible.len().saturating_sub(app.logs.scroll_offset);
    let start = end.saturating_sub(height);
    visible = visible[start..end].to_vec();
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
    );
    let title = if app.logs.dropped > 0 {
        format!(
            "Logs ({mode}; {} older entries evicted, including {} warnings and {} errors; retained: {}/{})",
            app.logs.dropped,
            app.logs.dropped_warnings,
            app.logs.dropped_errors,
            app.logs.retained_bytes,
            app.logs.max_bytes
        )
    } else {
        format!(
            "Logs ({mode}; retained: {}/{})",
            app.logs.retained_bytes, app.logs.max_bytes
        )
    };
    let title = if app.logs.searching {
        format!("{title} | search: {}_", app.logs.query)
    } else if app.logs.query.is_empty() {
        title
    } else {
        format!("{title} | search: {}", app.logs.query)
    };
    if app.logs.wrap {
        let text = visible
            .iter()
            .rev()
            .take(height)
            .rev()
            .map(|log| {
                format!(
                    "{:?} {} {}",
                    log.severity,
                    log.recipe.as_deref().unwrap_or(""),
                    log.message
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        frame.render_widget(
            Paragraph::new(text)
                .block(Block::default().title(title).borders(Borders::ALL))
                .wrap(Wrap { trim: false }),
            area,
        );
        return;
    }
    let rows = visible.into_iter().rev().take(height).rev().map(|l| {
        Row::new(vec![
            Cell::from(format!("{:?}", l.severity)),
            Cell::from(l.recipe.as_deref().unwrap_or("")),
            Cell::from(
                l.message
                    .chars()
                    .skip(app.logs.horizontal_offset)
                    .collect::<String>(),
            ),
        ])
    });
    frame.render_widget(
        Table::new(
            rows,
            [
                Constraint::Length(9),
                Constraint::Length(18),
                Constraint::Min(10),
            ],
        )
        .header(
            Row::new(["Level", "Recipe", "Message"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(Block::default().title(title).borders(Borders::ALL)),
        area,
    )
}
fn errors(frame: &mut Frame, app: &App, area: Rect) {
    let errors = app
        .logs
        .filtered()
        .filter(|log| matches!(log.severity, Severity::Warning | Severity::Error))
        .collect::<Vec<_>>();
    let selected = errors.get(app.error_selection).copied();
    let chunks = Layout::vertical([Constraint::Min(4), Constraint::Length(8)]).split(area);
    let rows = errors
        .into_iter()
        .rev()
        .take(area.height.saturating_sub(3) as usize)
        .rev()
        .enumerate()
        .map(|(index, log)| {
            Row::new(vec![
                Cell::from(format!("{:?}", log.severity)),
                Cell::from(log.recipe.as_deref().unwrap_or("")),
                Cell::from(log.task.as_deref().unwrap_or("")),
                Cell::from(
                    log.path
                        .as_deref()
                        .map_or_else(String::new, |path| path.display().to_string()),
                ),
                Cell::from(log.message.as_str()),
            ])
            .style(selected_style(app, index == app.error_selection))
        });
    frame.render_widget(
        Table::new(
            rows,
            [
                Constraint::Length(9),
                Constraint::Length(16),
                Constraint::Length(16),
                Constraint::Length(22),
                Constraint::Min(12),
            ],
        )
        .header(
            Row::new(["Level", "Recipe", "Task", "Location", "Message"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(
            Block::default()
                .title("Errors and warnings (from retained logs)")
                .borders(Borders::ALL),
        ),
        chunks[0],
    );
    let detail = selected.map_or_else(
        || "No retained warnings or errors.".into(),
        |log| {
            format!(
                "{}\nrecipe: {}  task: {}\ntimestamp: {}\nlocation: {}",
                log.message,
                log.recipe.as_deref().unwrap_or("unknown"),
                log.task.as_deref().unwrap_or("unknown"),
                timestamp_text(log.timestamp),
                log.path
                    .as_deref()
                    .map_or_else(|| "unknown".into(), |path| path.display().to_string())
            )
        },
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
fn recipes(frame: &mut Frame, app: &App, area: Rect) {
    let mut recipes = app.workspace.recipes.iter().collect::<Vec<_>>();
    recipes.sort_by(|left, right| left.name.cmp(&right.name));
    recipes.retain(|recipe| {
        matches_metadata(
            &app.metadata_query,
            &[
                recipe.name.as_str(),
                recipe.version.as_deref().unwrap_or(""),
                recipe.layer.as_deref().unwrap_or(""),
            ],
        )
    });
    let recipe_count = recipes.len();
    let selected = recipes.get(app.recipe_selection).copied();
    let chunks = Layout::vertical([Constraint::Min(4), Constraint::Length(5)]).split(area);
    frame.render_widget(
        Table::new(
            recipes.into_iter().enumerate().map(|(index, recipe)| {
                Row::new(vec![
                    Cell::from(recipe.name.as_str()),
                    Cell::from(recipe.version.as_deref().unwrap_or("")),
                    Cell::from(recipe.layer.as_deref().unwrap_or("")),
                ])
                .style(selected_style(app, index == app.recipe_selection))
            }),
            [
                Constraint::Percentage(40),
                Constraint::Percentage(25),
                Constraint::Percentage(35),
            ],
        )
        .header(
            Row::new(["Recipe", "Version", "Layer"])
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
        |recipe| {
            format!(
                "Recipe: {}\nVersion: {}\nLayer: {}",
                recipe.name,
                recipe.version.as_deref().unwrap_or("unknown"),
                recipe.layer.as_deref().unwrap_or("unknown")
            )
        },
    );
    frame.render_widget(
        Paragraph::new(format!(
            "{detail}\n\nb builds.  C cleans.  M runs menuconfig.  S requests cleansstate.  d opens a devtool workspace.  D resets it."
        ))
        .block(
            Block::default()
                .title("Selected recipe")
                .borders(Borders::ALL),
        ),
        chunks[1],
    );
}
fn layer_browser(frame: &mut Frame, app: &App, browser: &LayerBrowser, area: Rect) {
    let chunks =
        Layout::horizontal([Constraint::Percentage(42), Constraint::Percentage(58)]).split(area);
    frame.render_widget(
        Table::new(
            browser.entries.iter().enumerate().map(|(index, entry)| {
                let name = entry.path.file_name().map_or_else(
                    || entry.path.display().to_string(),
                    |name| name.to_string_lossy().into_owned(),
                );
                Row::new(vec![Cell::from(if entry.is_dir {
                    format!("▸ {name}/")
                } else {
                    format!("  {name}")
                })])
                .style(selected_style(app, index == browser.selection))
            }),
            [Constraint::Min(1)],
        )
        .block(
            Block::default()
                .title(format!(
                    "{}: {}",
                    browser.layer,
                    browser.directory.strip_prefix(&browser.root).map_or_else(
                        |_| browser.directory.display().to_string(),
                        |path| format!("/{}", path.display())
                    )
                ))
                .borders(Borders::ALL),
        ),
        chunks[0],
    );
    let title = browser.entries.get(browser.selection).map_or_else(
        || "Preview".into(),
        |entry| {
            format!(
                "Preview: {}",
                entry.path.file_name().map_or_else(
                    || entry.path.display().to_string(),
                    |name| name.to_string_lossy().into_owned()
                )
            )
        },
    );
    let preview = if browser
        .entries
        .get(browser.selection)
        .is_some_and(|entry| entry.is_dir)
    {
        "Directory selected. Press Enter to open it.".into()
    } else if browser.preview.is_empty() {
        "Select a readable recipe, configuration, or Markdown file.".into()
    } else {
        browser.preview.clone()
    };
    let selected_name = browser
        .entries
        .get(browser.selection)
        .and_then(|entry| entry.path.file_name())
        .map_or("", |name| name.to_str().unwrap_or(""));
    frame.render_widget(
        Paragraph::new(source_preview(&preview, selected_name, app.color_enabled))
            .block(Block::default().title(title).borders(Borders::ALL))
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
                    if app.color_enabled {
                        style = style.fg(Color::Green);
                    } else {
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
fn config(frame: &mut Frame, app: &App, area: Rect) {
    let mut variables = app.workspace.variables.iter().collect::<Vec<_>>();
    variables.sort_by_key(|(name, _)| *name);
    variables.retain(|(name, value)| {
        matches_metadata(&app.metadata_query, &[name.as_str(), value.as_str()])
    });
    let variable_count = variables.len();
    let selected = variables.get(app.config_selection).copied();
    let chunks = Layout::vertical([Constraint::Min(4), Constraint::Length(9)]).split(area);
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
    let detail = selected.map_or_else(
        || "No configuration variables supplied by the backend.".into(),
        |(name, value)| {
            let chain = app
                .workspace
                .variable_provenance_chain
                .get(name)
                .filter(|chain| !chain.is_empty())
                .map_or_else(
                    || "backend did not provide an original/append/override chain".into(),
                    |chain| chain.join("\n  -> "),
                );
            format!(
                "Variable: {name}\nEffective value: {value}\nProvenance: {}\nSource chain:\n  {chain}",
                app.workspace
                    .variable_provenance
                    .get(name)
                    .map_or("backend did not provide source provenance", String::as_str)
            )
        },
    );
    frame.render_widget(
        Paragraph::new(format!(
            "{detail}\n\no opens the provenance source file when available."
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
    frame.render_widget(Paragraph::new("B Image build options for the effective MACHINE; b build, c clean, m menuconfig, e choose target\n! Open an inherited Yocto shell; exit returns to Yoctui\nb Choose target and start build; h build history; Dashboard Up/Down scrolls observed package task progress\nc Cancel active build\nl Logs   f toggle follow   w toggle wrapping   s cycle severity\nR cycle recipe filter   T cycle task filter   n/N previous/next match\ne Errors   o open selected source log, layer directory, or config provenance\nr Recipes: b build, C clean, M menuconfig, S cleansstate, g server dependency graph, d devtool-edit, u update-recipe, F finish, P deploy, D reset selected recipe\ny Layers: e in-TUI edit, o external editor   v Configuration   x effective BBMASK, e edit with preview\n/ Search recipes, layers, or configuration   Esc Dashboard   q Quit\n\nCleansstate, Devtool reset/update-recipe/finish/deploy, BBMASK changes, and quitting an active build require confirmation.").block(Block::default().title("Help").borders(Borders::ALL)),area)
}
#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};
    use yoctui_model::BuildRequest;

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
        build_options.build_options_open = true;
        build_options.focus = FocusTarget::Dialog;
        let _ = rendered_text(&build_options, 80, 24);

        let mut palette = App::new(10, 1_000);
        palette.command_palette_open = true;
        palette.focus = FocusTarget::CommandPalette;
        let _ = rendered_text(&palette, 80, 24);

        let mut confirmation = App::new(10, 1_000);
        confirmation.recipe_task_confirmation = Some(BuildRequest {
            targets: vec!["base-files".into()],
            task: Some("listtasks".into()),
        });
        confirmation.focus = FocusTarget::Dialog;
        let _ = rendered_text(&confirmation, 80, 24);
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
    fn dependencies_render_server_supplied_values() {
        let mut terminal = Terminal::new(TestBackend::new(120, 25)).unwrap();
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Dependencies;
        app.dependencies = Some(yoctui_model::RecipeDependencies {
            recipe: "busybox".into(),
            build: vec!["virtual/libc".into()],
            runtime: vec!["base-files".into()],
        });
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Dependency graph"));
        assert!(output.contains("virtual/libc"));
        assert!(output.contains("base-files"));
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
            },
            success: true,
        });
        app.completed_tasks.push_back(yoctui_model::CompletedTask {
            task: yoctui_model::TaskInfo {
                id: yoctui_model::TaskId("bash:do_install".into()),
                recipe: "bash".into(),
                task: "do_install".into(),
                progress: Some(100),
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
        app.build_target_editing = true;
        app.build_target_input = "core-image-minimal".into();
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
        app.build_options_open = true;
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
        assert!(output.contains("including 1 warnings and 2 errors"));
    }
    #[test]
    fn renders_recipe_task_confirmation() {
        let mut terminal = Terminal::new(TestBackend::new(140, 30)).unwrap();
        let mut app = App::new(10, 1_000);
        app.recipe_task_confirmation = Some(BuildRequest {
            targets: vec!["busybox".into()],
            task: Some("cleansstate".into()),
        });
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
    fn renders_devtool_reset_confirmation() {
        let mut terminal = Terminal::new(TestBackend::new(100, 25)).unwrap();
        let mut app = App::new(10, 1_000);
        app.devtool_reset_confirmation = Some("busybox".into());
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
    }
    #[test]
    fn renders_devtool_update_recipe_confirmation() {
        let mut terminal = Terminal::new(TestBackend::new(100, 25)).unwrap();
        let mut app = App::new(10, 1_000);
        app.devtool_update_confirmation = Some("busybox".into());
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
    }
    #[test]
    fn renders_devtool_finish_confirmation() {
        let mut terminal = Terminal::new(TestBackend::new(120, 25)).unwrap();
        let mut app = App::new(10, 1_000);
        app.devtool_finish_confirmation = Some(yoctui_model::DevtoolFinishRequest {
            recipe: "busybox".into(),
            destination: "/layers/meta-demo".into(),
        });
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
    }
    #[test]
    fn renders_devtool_deploy_confirmation() {
        let mut terminal = Terminal::new(TestBackend::new(120, 25)).unwrap();
        let mut app = App::new(10, 1_000);
        app.devtool_deploy_confirmation = Some(yoctui_model::DevtoolDeployRequest {
            recipe: "busybox".into(),
            target: "qemuarm".into(),
        });
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
    }
    #[test]
    fn renders_recipe_editor_overlay() {
        let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
        let mut app = App::new(10, 1_000);
        app.recipe_editor = Some(RecipeEditor {
            recipe: "busybox".into(),
            root: "/build/workspace/sources/busybox".into(),
            files: vec!["main.c".into()],
            selection: 0,
            content: "int main() {}".into(),
            editing: false,
            dirty: false,
        });
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
    }
    #[test]
    fn layer_browser_renders_the_selected_file_preview() {
        let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Layers;
        app.layer_browser = Some(LayerBrowser {
            layer: "meta-demo".into(),
            root: "/layers/meta-demo".into(),
            directory: "/layers/meta-demo/conf".into(),
            entries: vec![yoctui_model::LayerBrowserEntry {
                path: "/layers/meta-demo/conf/layer.conf".into(),
                is_dir: false,
            }],
            selection: 0,
            preview: "BBFILE_COLLECTIONS += \\\"demo\\\"".into(),
        });
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("meta-demo: /conf"));
        assert!(output.contains("layer.conf"));
        assert!(output.contains("BBFILE_COLLECTIONS"));
    }
    #[test]
    fn bitbake_preview_highlights_assignments_and_comments() {
        let preview = source_preview("SUMMARY = \"demo\" # explanation", "demo.bb", true);
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
        app.image_picker = Some(yoctui_model::ImagePicker {
            images: vec!["core-image-minimal".into()],
            selection: 0,
        });
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
        assert!(output.contains("Version: 1.36"));

        app.screen = Screen::Layers;
        app.layer_browser = Some(LayerBrowser {
            layer: "meta".into(),
            root: "/layers/meta".into(),
            directory: "/layers/meta/conf".into(),
            entries: vec![],
            selection: 0,
            preview: "BBFILE_COLLECTIONS += \"meta\"".into(),
        });
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Path: /layers/meta/conf"));
        assert!(output.contains("BBFILE_COLLECTIONS"));
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
        app.build_completion_open = true;
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
        app.build_completion_open = true;
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
        app.bbmask_confirmation = Some("meta-broken/.*".into());
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
}
