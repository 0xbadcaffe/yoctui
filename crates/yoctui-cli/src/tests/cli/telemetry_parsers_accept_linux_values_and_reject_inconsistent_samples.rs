use super::*;

#[test]
fn telemetry_parsers_accept_linux_values_and_reject_inconsistent_samples() {
    assert_eq!(
        parse_memory_info("MemTotal:       16384 kB\nMemFree: 1024 kB\nMemAvailable: 4096 kB\n"),
        Some((16 * 1024 * 1024, 4 * 1024 * 1024))
    );
    assert_eq!(
        parse_memory_info("MemTotal: 10 kB\nMemAvailable: 11 kB\n"),
        None
    );
    assert_eq!(
        parse_memory_info("MemTotal: 10 MB\nMemAvailable: 5 MB\n"),
        None
    );
    assert_eq!(
        parse_load_average("1.25 0.50 12.345 2/100 123\n"),
        Some([1_250, 500, 12_345])
    );
    assert_eq!(parse_load_average("nan 0.50 1.00"), None);
    assert_eq!(parse_load_average("1.00 -0.5 1.00"), None);
}
