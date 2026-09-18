use super::*;

#[test]
fn parses_cpu_counters_from_proc_stat() {
    assert_eq!(
        parse_cpu_counters("cpu  100 20 30 400 50 0 0 0 0 0"),
        Some(CpuCounters {
            total: 600,
            idle: 450,
        })
    );
    assert_eq!(parse_cpu_counters("intr 1 2 3"), None);
}
