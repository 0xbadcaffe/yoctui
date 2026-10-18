use super::*;

#[test]
fn changed_lines_match_full_preview_without_mutating_save_or_history() {
    let cases = [
        ("", ""),
        ("", "工程\n"),
        ("delete\n", ""),
        ("same\nold\ntail\n", "same\nnew\ntail\n"),
        ("a\nb", "a\nb\n"),
        ("a\nb\nc\n", "a\nb\nx\ny\nc\n"),
    ];
    for (base, current) in cases {
        check_summary(base, current);
    }
    let old = "old\n".repeat(4200);
    let new = "new\n".repeat(4200);
    check_summary(&old, &new);
    check_summary(
        &("context\n".repeat(10) + &old),
        &("context\n".repeat(10) + &new),
    );
}

fn check_summary(base: &str, current: &str) {
    let mut document = TextAreaState::new(base.into());
    // Also test direct public text changes; no cached generation may go stale.
    document.text = current.into();
    let save = document.save_state().clone();
    let history = document.history_lengths();
    let expected: Vec<_> = document
        .clone()
        .preview_diff()
        .lines
        .iter()
        .filter(|line| line.kind != TextAreaDiffKind::Context)
        .cloned()
        .collect();
    for limit in [0, 1, 2, 8, 4095, 4096, 5000] {
        assert_eq!(
            document.changed_lines(limit),
            expected.iter().take(limit).cloned().collect::<Vec<_>>()
        );
        assert_eq!(document.save_state(), &save);
        assert_eq!(document.history_lengths(), history);
    }
}
