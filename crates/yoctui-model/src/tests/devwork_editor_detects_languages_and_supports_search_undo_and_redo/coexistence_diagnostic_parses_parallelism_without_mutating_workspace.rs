use super::*;

#[test]
fn coexistence_diagnostic_parses_parallelism_without_mutating_workspace() {
    let mut workspace = Workspace::default();
    workspace
        .variables
        .insert("BB_NUMBER_THREADS".into(), "24".into());
    workspace
        .variables
        .insert("PARALLEL_MAKE".into(), "-l 8 --jobs=20".into());
    let original = workspace.clone();
    let diagnostic = bitbake_coexistence_diagnostic(
        &workspace,
        &HostTelemetry {
            logical_cpu_count: Some(8),
            load_average_milli: Some([16_001, 8_000, 4_000]),
            ..HostTelemetry::default()
        },
    );
    assert_eq!(
        diagnostic.pressure,
        BitBakeCoexistencePressure::Oversubscribed
    );
    assert_eq!(diagnostic.bitbake_threads, Some(24));
    assert_eq!(diagnostic.parallel_make_jobs, Some(20));
    assert_eq!(diagnostic.review_example_jobs, Some(7));
    assert_eq!(workspace, original);
}
