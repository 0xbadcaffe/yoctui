use super::*;

#[test]
fn terminal_emulation_bounds_and_restores_scrollback_snapshots() {
    let mut terminal = emulator(3, 12, 4);
    terminal
        .process(b"one\r\ntwo\r\nthree\r\nfour\r\nfive")
        .unwrap();
    let current = terminal.snapshot(0).unwrap();
    assert!(current.plain_text.contains("five"));
    assert!(current.max_scrollback_offset > 0);
    let history = terminal.snapshot(usize::MAX).unwrap();
    assert_eq!(history.scrollback_offset, history.max_scrollback_offset);
    assert!(history.plain_text.contains("one") || history.plain_text.contains("two"));
    let current_again = terminal.snapshot(0).unwrap();
    assert_eq!(current_again.plain_text, current.plain_text);

    terminal
        .resize(PtyDimensions {
            columns: 20,
            rows: 5,
        })
        .unwrap();
    assert_eq!(
        terminal.snapshot(0).unwrap().dimensions,
        PtyDimensions {
            columns: 20,
            rows: 5
        }
    );
    assert!(matches!(
        TerminalEmulator::new(
            PtyDimensions {
                columns: 1_000,
                rows: 1_000
            },
            0
        ),
        Err(TerminalEmulationError::ScreenTooLarge { .. })
    ));
    assert_eq!(
        terminal.process(&vec![0; MAX_TERMINAL_FEED_BYTES + 1]),
        Err(TerminalEmulationError::FeedTooLarge(
            MAX_TERMINAL_FEED_BYTES + 1
        ))
    );
}
