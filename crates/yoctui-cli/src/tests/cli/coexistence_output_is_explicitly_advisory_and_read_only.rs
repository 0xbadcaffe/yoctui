use super::*;

#[test]
fn coexistence_output_is_explicitly_advisory_and_read_only() {
    let lines = bitbake_coexistence_lines(&BitBakeCoexistenceDiagnostic {
        pressure: BitBakeCoexistencePressure::Oversubscribed,
        logical_cpu_count: Some(8),
        load_one_milli: Some(13_250),
        bitbake_threads: Some(24),
        parallel_make_jobs: Some(16),
        review_example_jobs: Some(7),
        reasons: vec!["measured host load is high".into()],
    });
    assert!(
        lines
            .iter()
            .any(|line| line == "coexistence pressure: oversubscribed")
    );
    assert!(lines.iter().any(|line| line.contains("load1=13.25")));
    assert!(
        lines
            .iter()
            .any(|line| line.contains("BB_NUMBER_THREADS=\"7\""))
    );
    assert_eq!(
        lines.last().map(String::as_str),
        Some("coexistence policy: read-only; Yoctui changed no BitBake configuration")
    );
}
