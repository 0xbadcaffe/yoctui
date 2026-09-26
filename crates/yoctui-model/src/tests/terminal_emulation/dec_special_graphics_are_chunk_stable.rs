use super::*;

#[test]
fn dec_special_graphics_are_chunk_stable() {
    let mut terminal = emulator(5, 40, 10);
    for chunk in [
        b"\x1b".as_slice(),
        b"(0lq".as_slice(),
        b"qk\r\nx  x\r\nmqqj".as_slice(),
        b"\x1b(B normal qxlm".as_slice(),
    ] {
        terminal.process(chunk).unwrap();
    }

    let snapshot = terminal.snapshot(0).unwrap();
    assert!(snapshot.plain_text.contains("┌──┐"));
    assert!(snapshot.plain_text.contains("│  │"));
    assert!(snapshot.plain_text.contains("└──┘ normal qxlm"));
}

#[test]
fn g1_shift_and_reset_preserve_ordinary_text_and_utf8() {
    let mut terminal = emulator(4, 50, 10);
    terminal
        .process(b"\x1b)0\x0elqk\x0f qxlm \xe7\x95\x8c\r\n")
        .unwrap();
    let snapshot = terminal.snapshot(0).unwrap();
    assert!(snapshot.plain_text.contains("┌─┐ qxlm 界"));

    terminal.process(b"\x1b(0q\x1bcq").unwrap();
    assert_eq!(terminal.snapshot(0).unwrap().plain_text, "q");
}

#[test]
fn complete_dec_special_graphics_table_maps_to_unicode_cells() {
    let mut terminal = emulator(2, 40, 10);
    terminal
        .process(b"\x1b(0_`abcdefghijklmnopqrstuvwxyz{|}~\x1b(B")
        .unwrap();

    let snapshot = terminal.snapshot(0).unwrap();
    assert!(
        snapshot
            .plain_text
            .contains(" ◆▒␉␌␍␊°±␤␋┘┐┌└┼⎺⎻─⎼⎽├┤┴┬│≤≥π≠£·")
    );
}
