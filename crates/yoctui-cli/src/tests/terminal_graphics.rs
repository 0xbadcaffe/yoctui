use super::*;

#[test]
fn primary_device_attributes_require_complete_exact_sixel_capability() {
    for reply in [
        b"\x1b[?65;1;4;9c".as_slice(),
        b"\x1b[?63;1;2;4;6;9;15;17;22;28c",
    ] {
        assert!(primary_device_attributes_support_sixel(reply));
    }
    for reply in [
        b"\x1b[?65;1;9c".as_slice(),
        b"text 4",
        b"\x1b[?64;14;9c",
        b"\x1b[?65;1;4",
        b"\x1b[?65;1;4;9",
        b"\x1b[?4;6c",
        b"\x1b[?65;;4c",
        b"\x1b[?65;4garbagec",
        b"\x1b[?65;4\x1b[?1c",
    ] {
        assert!(!primary_device_attributes_support_sixel(reply), "{reply:?}");
    }
}

#[cfg(unix)]
#[test]
fn fragmented_terminal_reply_is_collected_until_complete() {
    use std::os::{fd::AsRawFd, unix::net::UnixStream};
    let (input, mut terminal) = UnixStream::pair().unwrap();
    let sender = std::thread::spawn(move || {
        for fragment in [b"\x1b[?63;".as_slice(), b"1;4", b";9c"] {
            terminal.write_all(fragment).unwrap();
            std::thread::sleep(Duration::from_millis(10));
        }
    });
    let reply = read_device_attributes(input.as_raw_fd(), Duration::from_secs(1)).unwrap();
    assert!(primary_device_attributes_support_sixel(&reply));
    sender.join().unwrap();
}

#[cfg(unix)]
#[test]
fn missing_or_truncated_terminal_reply_never_selects_graphics() {
    use std::os::{fd::AsRawFd, unix::net::UnixStream};
    let (input, mut terminal) = UnixStream::pair().unwrap();
    terminal.write_all(b"\x1b[?65;1;4").unwrap();
    assert!(read_device_attributes(input.as_raw_fd(), Duration::from_millis(20)).is_none());
    terminal.write_all(&[b'x'; MAX_REPLY_BYTES]).unwrap();
    assert!(read_device_attributes(input.as_raw_fd(), Duration::from_millis(20)).is_none());
}
