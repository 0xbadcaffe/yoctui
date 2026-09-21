/// Return a bounded, selection-centered viewport without retaining derived UI
/// state. Recomputing this range from authoritative selection and row counts
/// makes query/inventory invalidation immediate and stale-state-free.
pub fn centered_viewport_range(
    selected: Option<usize>,
    total: usize,
    visible: usize,
) -> std::ops::Range<usize> {
    if total == 0 || visible == 0 {
        return 0..0;
    }
    let visible = visible.min(total);
    let selected = selected.unwrap_or(0).min(total - 1);
    let start = selected
        .saturating_sub(visible / 2)
        .min(total.saturating_sub(visible));
    start..start + visible
}
