//! Development session browser and a native multi-folder editor workspace.
use super::*;
mod dialogs;
mod editor;
use yoctui_model::{WORKBENCH_MENU, WorkbenchFocus as Focus, WorkbenchFormKind};

pub(crate) fn workspace(frame: &mut Frame, app: &App, area: Rect) {
    let palette = ThemePalette::for_app(app);
    let Some(session) = app.workbench.current() else {
        let mut rows = vec![
            Line::from("n New workspace   Enter Open   r Rename   Delete Remove saved session"),
            Line::from("Development sessions retain folders, open files and unfinished edits."),
            Line::default(),
        ];
        if app.workbench.recovering {
            rows.push(Line::from("Loading saved workspace sessions…"));
        }
        if let Some(error) = &app.workbench.persistence_error {
            rows.push(Line::styled(
                error.clone(),
                palette.role(palette.warning, Modifier::empty()),
            ));
        }
        if app.workbench.sessions.is_empty() {
            rows.push(Line::from(
                "No workspaces yet. Press n to create one, then add a source folder.",
            ));
        }
        let height = usize::from(area.height.saturating_sub(6)).max(1);
        let first = app
            .workbench
            .selection
            .saturating_sub(height.saturating_sub(1));
        for (index, s) in app
            .workbench
            .sessions
            .iter()
            .enumerate()
            .skip(first)
            .take(height)
        {
            rows.push(Line::styled(
                format!(
                    "{} {} · {} folder(s) · {} tab(s){} · {}",
                    if index == app.workbench.selection {
                        "▶"
                    } else {
                        " "
                    },
                    s.name,
                    s.folders.len(),
                    s.tabs.len(),
                    if s.tabs.iter().any(|t| t.is_dirty()) {
                        " · unsaved"
                    } else {
                        ""
                    },
                    s.recipe.as_deref().unwrap_or("General development")
                ),
                if index == app.workbench.selection {
                    selected_style(app, true)
                } else {
                    Style::default()
                },
            ));
        }
        frame.render_widget(
            Paragraph::new(rows).block(Block::bordered().title("Workbench · Sessions")),
            area,
        );
        overlay(frame, app, area);
        return;
    };
    let layout = yoctui_app::development_workbench_layout(
        area.x,
        area.y,
        area.width,
        area.height,
        session.panel_zoom,
    );
    let rect = |a: [u16; 4]| Rect::new(a[0], a[1], a[2], a[3]);
    let rows = [
        rect(layout.header),
        rect(layout.explorer),
        rect(layout.status),
    ];
    frame.render_widget(Paragraph::new(format!(" {} · {} · Alt+m Actions · Alt+a Folder\n Alt+c Compile  Alt+b Build  Alt+d Modify  Alt+g GDB  Alt+i SDK", session.name, session.recipe.as_deref().unwrap_or("Bind recipe in Alt+m"))).style(palette.role(palette.informational, Modifier::empty())), rows[0]);
    let right = [rect(layout.tabs), rect(layout.editor), rect(layout.output)];
    let tabs = session
        .tabs
        .iter()
        .enumerate()
        .skip(
            session
                .tab
                .saturating_sub(usize::from(right[0].width / 20).max(1) - 1),
        )
        .take(usize::from(right[0].width / 20).max(1))
        .map(|(i, t)| {
            Span::styled(
                tab_label(i, &t.path, t.is_dirty()),
                if i == session.tab {
                    selected_style(app, true)
                } else {
                    Style::default()
                },
            )
        })
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(Line::from(tabs)), right[0]);
    files(frame, app, rect(layout.explorer));
    editor::render(frame, app, right[1]);
    output(frame, app, right[2]);
    let status = app.workbench.persistence_error.as_deref().unwrap_or(
        if app.workbench.pending_compile.is_some() {
            "Loading recipe tasks for Compile… Esc cancels; workspace remains available."
        } else if app.workbench.revision == app.workbench.persisted_revision {
            "Workspace state saved · no patch is published automatically"
        } else {
            "Saving workspace recovery state…"
        },
    );
    frame.render_widget(
        Paragraph::new(status).style(palette.role(
            if app.workbench.persistence_error.is_some() {
                palette.warning
            } else {
                palette.informational
            },
            Modifier::empty(),
        )),
        rows[2],
    );
    overlay(frame, app, area);
}

fn tab_label(index: usize, path: &std::path::Path, dirty: bool) -> String {
    let mut name = String::new();
    let mut width = 0;
    for c in path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .chars()
    {
        let cells = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
        if width + cells > 14 {
            break;
        }
        name.push(c);
        width += cells;
    }
    format!(
        "{:>2} {}{}{}  ",
        index + 1,
        name,
        " ".repeat(14 - width),
        if dirty { "●" } else { " " }
    )
}

fn files(frame: &mut Frame, app: &App, area: Rect) {
    let Some(s) = app.workbench.current() else {
        return;
    };
    let palette = ThemePalette::for_app(app);
    let mut lines = Vec::new();
    let available = usize::from(area.height.saturating_sub(2)).saturating_sub(
        s.folders.len() + usize::from(s.folders.get(s.folder).is_some_and(|f| f.error.is_some())),
    );
    for (index, folder) in s.folders.iter().enumerate() {
        let selected = index == s.folder;
        lines.push(Line::styled(
            format!(
                "{} {}{}",
                if selected { "▾" } else { "▸" },
                folder
                    .browser
                    .root
                    .file_name()
                    .unwrap_or(folder.browser.root.as_os_str())
                    .to_string_lossy(),
                if folder.loading { " · loading…" } else { "" }
            ),
            if selected {
                palette.role(palette.informational, Modifier::BOLD)
            } else {
                Style::default()
            },
        ));
        if !selected {
            continue;
        }
        if let Some(error) = &folder.error {
            lines.push(Line::styled(
                error.clone(),
                palette.role(palette.warning, Modifier::empty()),
            ));
        }
        let first = folder
            .browser
            .selection
            .saturating_sub(available.saturating_sub(1));
        for (row, entry) in folder
            .browser
            .entries
            .iter()
            .enumerate()
            .skip(first)
            .take(available)
        {
            let prefix = if entry.is_dir {
                if folder.browser.expanded.contains(&entry.path) {
                    "▾ "
                } else {
                    "▸ "
                }
            } else {
                "  "
            };
            lines.push(Line::styled(
                format!(
                    "  {}{prefix}{}",
                    "  ".repeat(entry.depth.min(32)),
                    entry.path.file_name().unwrap_or_default().to_string_lossy()
                ),
                if row == folder.browser.selection && s.focus == Focus::Files {
                    selected_style(app, true)
                } else {
                    Style::default()
                },
            ));
        }
    }
    if s.folders.is_empty() {
        lines.push(Line::from("Alt+a Add folder"));
    }
    frame.render_widget(
        Paragraph::new(lines).block(Block::bordered().title("Explorer · Alt+↑/↓ folders")),
        area,
    );
}

fn output(frame: &mut Frame, app: &App, area: Rect) {
    let palette = ThemePalette::for_app(app);
    let Some(s) = app.workbench.current() else {
        return;
    };
    if s.output == yoctui_model::WorkbenchOutput::Terminal
        && s.terminal.is_some()
        && s.terminal_daemon == app.daemon.instance_id
        && app
            .selected_terminal_session()
            .is_some_and(|pty| Some(pty.id) == s.terminal)
    {
        crate::terminal_sessions_workspace(frame, app, area);
        return;
    }
    let mut lines = Vec::new();
    let title = if s.output == yoctui_model::WorkbenchOutput::Changes {
        "Changes · saved file → current buffer"
    } else {
        "Build output · Alt+t Terminal · Alt+m Actions"
    };
    if s.output == yoctui_model::WorkbenchOutput::Changes {
        if let Some(t) = s.tabs.get(s.tab) {
            for row in t
                .diff
                .iter()
                .flat_map(|d| &d.lines)
                .take(usize::from(area.height.saturating_sub(2)))
            {
                let (prefix, color) = match row.kind {
                    yoctui_model::TextAreaDiffKind::Added => ("+", palette.success),
                    yoctui_model::TextAreaDiffKind::Removed => ("-", palette.error),
                    _ => (" ", palette.primary_foreground),
                };
                lines.push(Line::styled(
                    format!("{prefix}{}", row.text),
                    palette.role(color, Modifier::empty()),
                ));
            }
            if lines.is_empty() {
                lines.push(Line::from(if t.is_dirty() {
                    "Changes updated · choose Review changes again to regenerate the preview."
                } else {
                    "No unsaved changes in this tab."
                }));
            }
        }
    } else {
        let visible = usize::from(area.height.saturating_sub(2));
        let logs = app
            .logs
            .entries
            .iter()
            .rev()
            .filter(|line| {
                s.recipe
                    .as_ref()
                    .is_some_and(|recipe| line.recipe.as_ref() == Some(recipe))
            })
            .take(visible)
            .collect::<Vec<_>>();
        for line in logs.into_iter().rev() {
            lines.push(Line::from(line.message.clone()));
        }
        if lines.is_empty() {
            lines.push(Line::from(
                "No build output yet. Bind a recipe, then Alt+b to review a build.",
            ));
        }
    }
    frame.render_widget(
        Paragraph::new(lines)
            .block(Block::bordered().title(title))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn overlay(frame: &mut Frame, app: &App, area: Rect) {
    if dialogs::render(frame, app, area) {
        return;
    }
    if app.workbench.form.is_some() && app.workbench.folder_browser.is_some() {
        folder_dialog(frame, app, area);
        return;
    }
    if let Some(form) = &app.workbench.form {
        let title = match form.kind {
            WorkbenchFormKind::Create => "New workspace · name",
            WorkbenchFormKind::Rename => "Rename workspace",
            WorkbenchFormKind::AddFolder => "Add folder · absolute path",
            WorkbenchFormKind::NewFile => "New file · relative to selected folder",
            WorkbenchFormKind::Recipe => "Bind recipe · name in attached build",
            WorkbenchFormKind::Find => "Find in current file",
        };
        let width = area.width.saturating_sub(4).min(90);
        let popup = Rect::new(
            area.x + area.width.saturating_sub(width) / 2,
            area.y + area.height.saturating_sub(5) / 2,
            width,
            5.min(area.height),
        );
        frame.render_widget(Clear, popup);
        frame.render_widget(
            Paragraph::new(format!(
                "{}▏\nEnter confirm · Ctrl+u clear · Esc cancel",
                form.text
            ))
            .block(Block::bordered().title(title)),
            popup,
        );
    } else if let Some(selection) = app.workbench.menu {
        let width = area.width.saturating_sub(4).min(65);
        let height = (WORKBENCH_MENU.len() as u16 + 3).min(area.height);
        let popup = Rect::new(
            area.x + area.width.saturating_sub(width) / 2,
            area.y + area.height.saturating_sub(height) / 2,
            width,
            height,
        );
        frame.render_widget(Clear, popup);
        let visible = usize::from(height.saturating_sub(2));
        let first = selection.saturating_sub(visible.saturating_sub(1));
        let items = WORKBENCH_MENU
            .iter()
            .enumerate()
            .skip(first)
            .take(visible)
            .map(|(i, label)| {
                Line::styled(
                    format!("{} {label}", if i == selection { "▶" } else { " " }),
                    if i == selection {
                        selected_style(app, true)
                    } else {
                        Style::default()
                    },
                )
            })
            .collect::<Vec<_>>();
        frame.render_widget(
            Paragraph::new(items)
                .block(Block::bordered().title("Workbench · Devtool & workspace actions")),
            popup,
        );
    }
}

fn folder_dialog(frame: &mut Frame, app: &App, area: Rect) {
    let form = app.workbench.form.as_ref().expect("folder form");
    let browser = app
        .workbench
        .folder_browser
        .as_ref()
        .expect("folder browser");
    let [x, y, width, height] =
        yoctui_app::workbench_folder_dialog(area.x, area.y, area.width, area.height);
    let popup = Rect::new(x, y, width, height);
    let palette = ThemePalette::for_app(app);
    frame.render_widget(Clear, popup);
    frame.render_widget(
        Block::bordered().title("Add folder · browse filesystem"),
        popup,
    );
    if width < 4 || height < 6 {
        return;
    }
    let path = format!(
        "{}{}",
        form.text,
        if browser.editing_path { "▏" } else { "" }
    );
    frame.render_widget(
        Paragraph::new(path.clone())
            .scroll((
                0,
                unicode_width::UnicodeWidthStr::width(path.as_str())
                    .saturating_sub(usize::from(width - 3))
                    .min(u16::MAX as usize) as u16,
            ))
            .style(if browser.editing_path {
                palette.role(palette.informational, Modifier::BOLD)
            } else {
                Style::default()
            }),
        Rect::new(x + 1, y + 1, width - 2, 1),
    );
    let help = if browser.editing_path {
        "Enter add path · Tab/↓ browse · Ctrl+u clear\nEsc cancel"
    } else {
        "Enter open · Space add · Tab type path\n←/Backspace parent · Esc cancel"
    };
    frame.render_widget(Paragraph::new(help), Rect::new(x + 1, y + 2, width - 2, 2));
    let visible = usize::from(height.saturating_sub(6));
    let first = browser.selection.saturating_sub(visible.saturating_sub(1));
    let rows = (first..(first + visible).min(browser.entries.len() + 1))
        .map(|index| {
            let label = if index == 0 {
                "./  (add current folder)".to_string()
            } else {
                format!(
                    "{}/",
                    browser.entries[index - 1]
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                )
            };
            Line::styled(
                format!(
                    "{} {label}",
                    if index == browser.selection {
                        "▶"
                    } else {
                        " "
                    }
                ),
                if index == browser.selection && !browser.editing_path {
                    selected_style(app, true)
                } else {
                    Style::default()
                },
            )
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(rows),
        Rect::new(x + 1, y + 4, width - 2, height - 6),
    );
    let status = if browser.loading {
        "Loading directories…".to_string()
    } else if let Some(error) = &browser.error {
        error.clone()
    } else {
        format!(
            "{} folder(s) · {} / {}",
            browser.entries.len(),
            browser.selection + 1,
            browser.entries.len() + 1
        )
    };
    frame.render_widget(
        Paragraph::new(status).style(palette.role(
            if browser.error.is_some() {
                palette.warning
            } else {
                palette.informational
            },
            Modifier::empty(),
        )),
        Rect::new(x + 1, y + height - 2, width - 2, 1),
    );
}
