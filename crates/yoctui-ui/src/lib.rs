//! Rendering only; no backend parsing or mutation lives in widgets.
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, Wrap},
};
use yoctui_model::{App, Screen, Severity, format_duration};

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

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();
    if area.width < 30 || area.height < 8 {
        frame.render_widget(
            Paragraph::new("Terminal too small\nResize to at least 30x8")
                .block(Block::default().borders(Borders::ALL)),
            area,
        );
        return;
    }
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(area);
    let elapsed = app
        .elapsed()
        .map(format_duration)
        .unwrap_or_else(|| "--:--:--".into());
    frame.render_widget(
        Paragraph::new(format!(
            " Yoctui | {:?} | {} | {} | warnings: {} errors: {}",
            app.screen, app.build.status, elapsed, app.build.warnings, app.build.errors
        ))
        .block(Block::default().borders(Borders::ALL)),
        chunks[0],
    );
    match app.screen {
        Screen::Dashboard => dashboard(frame, app, chunks[1]),
        Screen::Logs => logs(frame, app, chunks[1]),
        Screen::Errors => errors(frame, app, chunks[1]),
        Screen::Recipes => recipes(frame, app, chunks[1]),
        Screen::Layers => layers(frame, app, chunks[1]),
        Screen::Configuration => config(frame, app, chunks[1]),
        Screen::Help => help(frame, chunks[1]),
    };
    frame.render_widget(Paragraph::new("b target/build | c cancel | l logs | f follow | w wrap | s severity | R recipe | T task | / search | n/N match | e errors | r recipes | y layers | v config | ? help | q quit").style(Style::default().fg(Color::DarkGray)),chunks[2]);
    if app.quit_confirm {
        let popup = Rect::new(area.width / 4, area.height / 3, area.width / 2, 3);
        frame.render_widget(Clear, popup);
        frame.render_widget(
            Paragraph::new("Build is active. Press Y to quit UI, or Esc to continue.")
                .block(Block::default().title("Confirm quit").borders(Borders::ALL)),
            popup,
        )
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
                "Target: {}_\n\nEnter starts the build; Esc cancels.",
                app.build_target_input
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
fn dashboard(frame: &mut Frame, app: &App, area: Rect) {
    let active = app
        .tasks
        .values()
        .map(|t| format!("{}:{} {}%", t.recipe, t.task, t.progress.unwrap_or(0)))
        .collect::<Vec<_>>()
        .join("\n");
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
    frame.render_widget(
        Paragraph::new(format!(
            "Target: {}\nBackend: {}\nStatus: {}\nMachine: {}\nDistro: {}\nRelease: {}\nTasks: {}/{} (active: {})\nWarnings: {}  Errors: {}\n\nActive tasks:\n{}",
            app.build.target.as_deref().unwrap_or("none"),
            app.backend,
            app.build.status,
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
            active
        ))
        .block(Block::default().title("Build").borders(Borders::ALL)),
        chunks[0],
    );
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
            "Logs ({mode}; {} older entries evicted; retained: {}/{})",
            app.logs.dropped, app.logs.retained_bytes, app.logs.max_bytes
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
    let chunks = Layout::vertical([Constraint::Min(4), Constraint::Length(5)]).split(area);
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
            .style(if index == app.error_selection {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            })
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
                "{}\nrecipe: {}  task: {}\nlocation: {}",
                log.message,
                log.recipe.as_deref().unwrap_or("unknown"),
                log.task.as_deref().unwrap_or("unknown"),
                log.path
                    .as_deref()
                    .map_or_else(|| "unknown".into(), |path| path.display().to_string())
            )
        },
    );
    frame.render_widget(
        Paragraph::new(format!("{detail}\n\nPress Enter to jump to matching logs."))
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
                .style(if index == app.recipe_selection {
                    Style::default().bg(Color::DarkGray)
                } else {
                    Style::default()
                })
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
        Paragraph::new(detail).block(
            Block::default()
                .title("Selected recipe")
                .borders(Borders::ALL),
        ),
        chunks[1],
    );
}
fn layers(frame: &mut Frame, app: &App, area: Rect) {
    let mut layers = app.workspace.layers.iter().collect::<Vec<_>>();
    layers.sort_by(|left, right| left.name.cmp(&right.name));
    layers.retain(|layer| {
        matches_metadata(
            &app.metadata_query,
            &[layer.name.as_str(), layer.path.to_str().unwrap_or("")],
        )
    });
    let layer_count = layers.len();
    let selected = layers.get(app.layer_selection).copied();
    let chunks = Layout::vertical([Constraint::Min(4), Constraint::Length(5)]).split(area);
    frame.render_widget(
        Table::new(
            layers.into_iter().enumerate().map(|(index, layer)| {
                Row::new(vec![
                    Cell::from(layer.name.as_str()),
                    Cell::from(layer.path.display().to_string()),
                    Cell::from(
                        layer
                            .priority
                            .map_or_else(String::new, |priority| priority.to_string()),
                    ),
                ])
                .style(if index == app.layer_selection {
                    Style::default().bg(Color::DarkGray)
                } else {
                    Style::default()
                })
            }),
            [
                Constraint::Percentage(30),
                Constraint::Percentage(55),
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
                        "Layers (shown: {} of {})",
                        layer_count,
                        app.workspace.layers.len()
                    ),
                    app,
                ))
                .borders(Borders::ALL),
        ),
        chunks[0],
    );
    let detail = selected.map_or_else(
        || "No layers supplied by the backend.".into(),
        |layer| {
            format!(
                "Layer: {}\nPath: {}\nPriority: {}",
                layer.name,
                layer.path.display(),
                layer
                    .priority
                    .map_or_else(|| "unknown".into(), |priority| priority.to_string())
            )
        },
    );
    frame.render_widget(
        Paragraph::new(detail).block(
            Block::default()
                .title("Selected layer")
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
    let chunks = Layout::vertical([Constraint::Min(4), Constraint::Length(5)]).split(area);
    frame.render_widget(
        Table::new(
            variables
                .into_iter()
                .enumerate()
                .map(|(index, (name, value))| {
                    Row::new(vec![Cell::from(name.as_str()), Cell::from(value.as_str())]).style(
                        if index == app.config_selection {
                            Style::default().bg(Color::DarkGray)
                        } else {
                            Style::default()
                        },
                    )
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
    let detail = selected.map_or_else(|| "No configuration variables supplied by the backend.".into(), |(name, value)| format!("Variable: {name}\nEffective value: {value}\nProvenance: backend value (provenance unavailable)"));
    frame.render_widget(
        Paragraph::new(detail)
            .block(
                Block::default()
                    .title("Selected variable")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false }),
        chunks[1],
    );
}
fn help(frame: &mut Frame, area: Rect) {
    frame.render_widget(Paragraph::new("b Choose target and start build\nc Cancel active build\nl Logs   f toggle follow   w toggle wrapping   s cycle severity\nR cycle recipe filter   T cycle task filter   n/N previous/next match\ne Errors   r Recipes   y Layers   v Configuration\n/ Search recipes, layers, or configuration   Esc Dashboard   q Quit\n\nQuit requires confirmation during an active build.").block(Block::default().title("Help").borders(Borders::ALL)),area)
}
#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};
    #[test]
    fn renders_small_terminal() {
        let mut terminal = Terminal::new(TestBackend::new(20, 5)).unwrap();
        terminal.draw(|f| render(f, &App::new(1, 1))).unwrap();
        assert!(
            terminal
                .backend()
                .buffer()
                .content
                .iter()
                .any(|c| c.symbol() == "T")
        );
    }

    #[test]
    fn renders_notification() {
        let mut app = App::new(1, 1);
        app.notification = Some("Backend unavailable".into());
        let mut terminal = Terminal::new(TestBackend::new(80, 20)).unwrap();
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
        let mut terminal = Terminal::new(TestBackend::new(100, 25)).unwrap();
        let mut app = App::new(10, 1_000);
        app.backend = "bridge".into();
        app.build.completed = 3;
        app.build.total = Some(7);
        app.build.warnings = 2;
        app.build.errors = 1;
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
    }
    #[test]
    fn renders_build_target_editor() {
        let mut terminal = Terminal::new(TestBackend::new(80, 20)).unwrap();
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
}
