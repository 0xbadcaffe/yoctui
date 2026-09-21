use super::*;

#[test]
fn oversized_partial_line_is_rejected_and_cleared() {
    let mut framer = LineFramer::default();
    assert!(matches!(
        framer.push(&vec![b'x'; MAX_LINE_BYTES + 1]),
        Err(ProtocolError::TooLarge)
    ));
    assert_eq!(framer.pending_len(), 0);
}
