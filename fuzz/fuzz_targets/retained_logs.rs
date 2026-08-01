#![no_main]

use std::time::{Duration, SystemTime};

use libfuzzer_sys::fuzz_target;
use yoctui_model::{LogEntry, LogState, Severity};

const MAX_OPERATIONS: usize = 256;
const MAX_MESSAGE_BYTES: usize = 256;

fuzz_target!(|data: &[u8]| {
    let max_entries = data.first().map_or(1, |value| usize::from(*value) % 64 + 1);
    let max_bytes = data.get(1).map_or(1, |value| usize::from(*value) * 16 + 1);
    let mut logs = LogState::new(max_entries, max_bytes);

    for (index, chunk) in data
        .get(2..)
        .unwrap_or_default()
        .chunks(MAX_MESSAGE_BYTES)
        .take(MAX_OPERATIONS)
        .enumerate()
    {
        let severity = match chunk.first().copied().unwrap_or_default() % 4 {
            0 => Severity::Trace,
            1 => Severity::Info,
            2 => Severity::Warning,
            _ => Severity::Error,
        };
        logs.insert(LogEntry {
            id: 0,
            severity,
            message: String::from_utf8_lossy(chunk).into_owned(),
            recipe: None,
            task: None,
            path: None,
            timestamp: SystemTime::UNIX_EPOCH + Duration::from_secs(index as u64),
            build: None,
            protected: false,
            diagnostic: None,
        });
        assert!(logs.entries.len() <= max_entries);
        assert!(logs.retained_bytes <= max_bytes || logs.entries.is_empty());
        assert_eq!(
            logs.retained_bytes,
            logs.entries
                .iter()
                .map(|entry| entry.message.len())
                .sum::<usize>()
        );
    }
});
