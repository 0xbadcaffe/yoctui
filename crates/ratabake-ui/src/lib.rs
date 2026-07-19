//! Rendering only; no backend parsing or mutation lives in widgets.
use ratabake_model::{App, Screen, Severity, format_duration};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, Wrap},
};
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
        Screen::Logs | Screen::Errors => logs(frame, app, chunks[1]),
        Screen::Recipes => recipes(frame, app, chunks[1]),
        Screen::Layers => layers(frame, app, chunks[1]),
        Screen::Configuration => config(frame, app, chunks[1]),
        Screen::Help => help(frame, chunks[1]),
    };
    frame.render_widget(Paragraph::new("b build | c cancel | l logs | f follow | w wrap | s severity | e errors | r recipes | y layers | v config | ? help | q quit").style(Style::default().fg(Color::DarkGray)),chunks[2]);
    if app.quit_confirm {
        let popup = Rect::new(area.width / 4, area.height / 3, area.width / 2, 3);
        frame.render_widget(Clear, popup);
        frame.render_widget(
            Paragraph::new("Build is active. Press Y to quit UI, or Esc to continue.")
                .block(Block::default().title("Confirm quit").borders(Borders::ALL)),
            popup,
        )
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
            "Target: {}\nMachine: {}\nDistro: {}\nActive tasks: {}\n{}",
            app.build.target.as_deref().unwrap_or("none"),
            app.workspace
                .variables
                .get("MACHINE")
                .map_or("unknown", String::as_str),
            app.workspace
                .variables
                .get("DISTRO")
                .map_or("unknown", String::as_str),
            app.tasks.len(),
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
    let visible = app
        .logs
        .filtered()
        .filter(|l| {
            app.screen != Screen::Errors
                || matches!(l.severity, Severity::Warning | Severity::Error)
        })
        .collect::<Vec<_>>();
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
    );
    let title = if app.logs.dropped > 0 {
        format!("Logs ({mode}; {} older entries evicted)", app.logs.dropped)
    } else {
        format!("Logs ({mode})")
    };
    if app.logs.wrap {
        let text = visible
            .iter()
            .rev()
            .take(area.height.saturating_sub(3) as usize)
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
    let rows = visible
        .into_iter()
        .rev()
        .take(area.height.saturating_sub(3) as usize)
        .rev()
        .map(|l| {
            Row::new(vec![
                Cell::from(format!("{:?}", l.severity)),
                Cell::from(l.recipe.as_deref().unwrap_or("")),
                Cell::from(l.message.as_str()),
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
fn recipes(frame: &mut Frame, app: &App, area: Rect) {
    frame.render_widget(
        Paragraph::new(
            app.workspace
                .recipes
                .iter()
                .map(|r| {
                    format!(
                        "{} {} {}",
                        r.name,
                        r.version.as_deref().unwrap_or(""),
                        r.layer.as_deref().unwrap_or("")
                    )
                })
                .collect::<Vec<_>>()
                .join("\n"),
        )
        .block(
            Block::default()
                .title("Recipes (BitBake supplied)")
                .borders(Borders::ALL),
        ),
        area,
    )
}
fn layers(frame: &mut Frame, app: &App, area: Rect) {
    frame.render_widget(
        Paragraph::new(
            app.workspace
                .layers
                .iter()
                .map(|l| format!("{} {} priority {:?}", l.name, l.path.display(), l.priority))
                .collect::<Vec<_>>()
                .join("\n"),
        )
        .block(Block::default().title("Layers").borders(Borders::ALL)),
        area,
    )
}
fn config(frame: &mut Frame, app: &App, area: Rect) {
    frame.render_widget(
        Paragraph::new(
            app.workspace
                .variables
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join("\n"),
        )
        .block(
            Block::default()
                .title("Effective configuration (read-only)")
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: false }),
        area,
    )
}
fn help(frame: &mut Frame, area: Rect) {
    frame.render_widget(Paragraph::new("b Start build (available when target supplied)\nc Cancel active build\nl Logs   f toggle follow   w toggle wrapping   s cycle severity\ne Errors   r Recipes   y Layers   v Configuration\n? This help   Esc Dashboard   q Quit\n\nQuit requires confirmation during an active build.").block(Block::default().title("Help").borders(Borders::ALL)),area)
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
}
