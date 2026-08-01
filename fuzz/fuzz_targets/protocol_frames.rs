#![no_main]

use libfuzzer_sys::fuzz_target;
use yoctui_protocol::{Command, Event, LineFramer, MAX_LINE_BYTES, decode_line};

fuzz_target!(|data: &[u8]| {
    let oversized;
    let bytes = if data.first() == Some(&b'O') {
        oversized = vec![b'x'; MAX_LINE_BYTES + 1];
        oversized.as_slice()
    } else {
        data.strip_prefix(b"R").unwrap_or(data)
    };

    let previous = data
        .get(1..9)
        .and_then(|value| value.try_into().ok())
        .map(u64::from_le_bytes);
    let _ = decode_line::<Command>(bytes, previous);
    let _ = decode_line::<Event>(bytes, previous);

    let chunk_size = data
        .get(9)
        .copied()
        .map_or(1, |value| usize::from(value).saturating_add(1));
    let mut framer = LineFramer::default();
    for chunk in bytes.chunks(chunk_size) {
        match framer.push(chunk) {
            Ok(frames) => {
                for frame in frames {
                    let _ = decode_line::<Command>(&frame, previous);
                    let _ = decode_line::<Event>(&frame, previous);
                }
            }
            Err(_) => break,
        }
    }
    assert!(framer.pending_len() <= MAX_LINE_BYTES);
});
