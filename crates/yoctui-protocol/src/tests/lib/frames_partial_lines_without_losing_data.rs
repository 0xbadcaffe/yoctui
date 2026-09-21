use super::*;

#[test]
fn frames_partial_lines_without_losing_data() {
    let mut framer = LineFramer::default();
    assert!(framer.push(b"one\ntw").unwrap().as_slice() == [b"one".to_vec()]);
    assert_eq!(framer.pending_len(), 2);
    assert_eq!(framer.push(b"o\n").unwrap(), vec![b"two".to_vec()]);
}
