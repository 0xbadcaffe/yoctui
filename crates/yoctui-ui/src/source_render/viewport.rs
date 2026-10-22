/// Reuse multiline syntax state; clone only the rows that can affect this view.
/// Stop skipping at a wrapping row: Ratatui owns its precise Unicode reflow.
pub(crate) fn scrolled_source_preview(
    content: &str,
    file_name: &str,
    app: &App,
    width: u16,
    scroll: u16,
    height: u16,
) -> (Text<'static>, u16) {
    let text = cached_source_preview(content, file_name, app);
    let consumed = if width == 0 {
        0
    } else {
        text.lines
            .iter()
            .take(usize::from(scroll))
            .take_while(|line| line.width() <= usize::from(width))
            .count()
    };
    let remainder = scroll - consumed as u16;
    let mut viewport = Text::from(text.lines.iter().skip(consumed)
        .take(usize::from(remainder) + usize::from(height)).cloned().collect::<Vec<_>>());
    viewport.style = text.style;
    viewport.alignment = text.alignment;
    (viewport, remainder)
}
