use super::*;

#[test]
fn coexistence_diagnostic_distinguishes_nominal_busy_and_unknown() {
    let mut workspace = Workspace::default();
    workspace
        .variables
        .insert("PARALLEL_MAKE".into(), "-j 8 -l8".into());
    let busy = bitbake_coexistence_diagnostic(
        &workspace,
        &HostTelemetry {
            logical_cpu_count: Some(8),
            load_average_milli: Some([7_200, 7_000, 6_000]),
            ..HostTelemetry::default()
        },
    );
    assert_eq!(busy.pressure, BitBakeCoexistencePressure::Busy);
    workspace.variables.clear();
    let nominal = bitbake_coexistence_diagnostic(
        &workspace,
        &HostTelemetry {
            logical_cpu_count: Some(8),
            load_average_milli: Some([1_000, 1_000, 1_000]),
            ..HostTelemetry::default()
        },
    );
    assert_eq!(nominal.pressure, BitBakeCoexistencePressure::Nominal);
    let unknown = bitbake_coexistence_diagnostic(&workspace, &HostTelemetry::default());
    assert_eq!(unknown.pressure, BitBakeCoexistencePressure::Unknown);
}
