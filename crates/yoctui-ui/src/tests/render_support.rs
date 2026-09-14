//! Render support.
use super::*;

pub(crate) fn concept_text_capture(terminal: &Terminal<TestBackend>) -> String {
    let buffer = terminal.backend().buffer();
    let mut output = String::new();
    for y in 0..TARGET_GOLDEN_HEIGHT {
        let mut row = (0..TARGET_GOLDEN_WIDTH)
            .map(|x| buffer[(x, y)].symbol())
            .collect::<String>();
        while row.ends_with(' ') {
            row.pop();
        }
        output.push_str(&row);
        output.push('\n');
    }
    output
}

pub(crate) fn rendered_text(app: &App, width: u16, height: u16) -> String {
    rendered_text_at(app, width, height, SystemTime::now())
}

pub(crate) fn rendered_text_at(app: &App, width: u16, height: u16, now: SystemTime) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|frame| render_at(frame, app, now)).unwrap();
    terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}

pub(crate) fn rendered_region_rows(
    width: u16,
    height: u16,
    mut render: impl FnMut(&mut Frame<'_>, Rect),
) -> Vec<String> {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|frame| {
            let area = frame.area();
            render(frame, area);
        })
        .unwrap();
    terminal
        .backend()
        .buffer()
        .content
        .chunks(usize::from(width))
        .map(|row| row.iter().map(|cell| cell.symbol()).collect::<String>())
        .collect()
}

pub(crate) struct SemanticSnapshot<'a> {
    pub(crate) name: &'a str,
    pub(crate) screen: Screen,
    pub(crate) anchors: &'a [&'a str],
    pub(crate) selected: Option<&'a str>,
}

pub(crate) fn assert_semantic_snapshot(app: &App, snapshot: &SemanticSnapshot<'_>) {
    let mut state = app.clone();
    state.screen = snapshot.screen;
    state.focus = FocusTarget::Workspace;
    let mut terminal = Terminal::new(TestBackend::new(160, 50)).unwrap();
    terminal
        .draw(|frame| render_at(frame, &state, literal_now()))
        .unwrap();
    let buffer = terminal.backend().buffer();
    let lines = buffer
        .content
        .chunks(160)
        .map(|row| row.iter().map(|cell| cell.symbol()).collect::<String>())
        .collect::<Vec<_>>();
    for anchor in snapshot.anchors {
        assert!(
            lines.iter().any(|line| line.contains(anchor)),
            "semantic snapshot {} lost anchor {anchor:?}:\n{}",
            snapshot.name,
            lines.join("\n")
        );
    }
    if let Some(selected) = snapshot.selected {
        let palette = ThemePalette::for_app(&state);
        let selected_row = buffer.content.chunks(160).find(|row| {
            row.iter()
                .map(|cell| cell.symbol())
                .collect::<String>()
                .contains(selected)
                && row.iter().any(|cell| {
                    cell.bg == palette.selection_background
                        || cell.modifier.contains(Modifier::REVERSED)
                })
        });
        assert!(
            selected_row.is_some(),
            "semantic snapshot {} selected row {selected:?} lost selection styling",
            snapshot.name
        );
    }
}

pub(crate) fn assert_dialog_semantic_snapshot(name: &str, app: &App, anchors: &[&str]) {
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    terminal
        .draw(|frame| render_at(frame, app, literal_now()))
        .unwrap();
    let output = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    for anchor in anchors {
        assert!(
            output.contains(anchor),
            "semantic dialog snapshot {name} lost anchor {anchor:?}: {output}"
        );
    }
}
