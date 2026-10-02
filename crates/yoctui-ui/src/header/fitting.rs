/// Preserve semantic styles while marking a cell-width-clipped primary context.
pub(super) fn fit_header_spans(spans: Vec<Span<'static>>, width: u16) -> Vec<Span<'static>> {
    if spans.iter().map(Span::width).sum::<usize>() <= usize::from(width) {
        return spans;
    }
    if width == 0 {
        return Vec::new();
    }
    let mut remaining = width.saturating_sub(1);
    let mut fitted = Vec::new();
    for span in spans {
        let span_width = u16::try_from(span.width()).unwrap_or(u16::MAX);
        if span_width <= remaining {
            remaining -= span_width;
            fitted.push(span);
        } else {
            let text = bounded_cell_text(
                &format!("{}…", span.content),
                remaining.saturating_add(1),
            );
            fitted.push(Span::styled(text, span.style));
            return fitted;
        }
    }
    fitted.push(Span::raw("…"));
    fitted
}
