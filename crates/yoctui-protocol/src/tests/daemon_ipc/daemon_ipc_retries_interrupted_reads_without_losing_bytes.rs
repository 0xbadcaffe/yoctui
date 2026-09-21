use super::*;

#[test]
fn daemon_ipc_retries_interrupted_reads_without_losing_bytes() {
    let mut reader = InterruptOnce {
        inner: io::Cursor::new(b"frame"),
        interrupted: false,
    };
    let mut buffer = [0_u8; 5];

    assert_eq!(
        read_retrying_interrupts(&mut reader, &mut buffer).unwrap(),
        buffer.len()
    );
    assert_eq!(&buffer, b"frame");
}
