/// Avoid Unicode word reflow for already-consumed, nonwrapping rows. Highlight
/// the complete source first so multiline syntax state remains unchanged. Stop
/// at the first wrapping row: its visual height belongs to Ratatui, not us.
pub(crate) fn scrolled_source_preview(
    content: &str,
    file_name: &str,
    app: &App,
    width: u16,
    scroll: u16,
) -> (Text<'static>, u16) {
    let mut text = source_preview(content, file_name, app);
    let consumed = if width == 0 {
        0
    } else {
        text.lines
            .iter()
            .take(usize::from(scroll))
            .take_while(|line| line.width() <= usize::from(width))
            .count()
    };
    text.lines.drain(..consumed);
    (text, scroll - consumed as u16)
}
