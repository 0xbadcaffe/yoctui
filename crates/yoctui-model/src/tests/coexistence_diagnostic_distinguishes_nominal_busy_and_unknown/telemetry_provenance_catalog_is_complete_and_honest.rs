use super::*;

#[test]
fn telemetry_provenance_catalog_is_complete_and_honest() {
    let metrics = TELEMETRY_PROVENANCE
        .iter()
        .map(|entry| entry.metric)
        .collect::<HashSet<_>>();
    assert_eq!(TELEMETRY_PROVENANCE.len(), 17);
    assert_eq!(metrics.len(), TELEMETRY_PROVENANCE.len());
    for required in [
        TelemetryMetric::HostCpuUtilization,
        TelemetryMetric::HostLogicalCpuCount,
        TelemetryMetric::HostMemoryCapacity,
        TelemetryMetric::BuildFilesystemCapacity,
        TelemetryMetric::DiskReadRate,
        TelemetryMetric::DiskWriteRate,
        TelemetryMetric::NetworkReceiveRate,
        TelemetryMetric::NetworkTransmitRate,
        TelemetryMetric::DaemonUptime,
        TelemetryMetric::BitBakeState,
        TelemetryMetric::ConnectedClients,
        TelemetryMetric::TerminalSessions,
        TelemetryMetric::ActiveJobs,
    ] {
        assert!(metrics.contains(&required), "missing {required:?}");
    }

    let provenance = |metric| {
        TELEMETRY_PROVENANCE
            .iter()
            .find(|entry| entry.metric == metric)
            .unwrap()
    };
    let cpu = provenance(TelemetryMetric::HostCpuUtilization);
    assert_eq!(cpu.source, TelemetrySource::HostProcStat);
    assert_eq!(cpu.sample_period_seconds, Some(1));
    assert_eq!(cpu.history_samples, HOST_TELEMETRY_HISTORY_SAMPLES);
    assert!(cpu.requires_delta && cpu.renderable);

    for metric in [
        TelemetryMetric::DiskReadRate,
        TelemetryMetric::DiskWriteRate,
    ] {
        let entry = provenance(metric);
        assert_eq!(entry.source, TelemetrySource::HostProcDiskstats);
        assert_eq!(entry.host_support, TelemetryHostSupport::Linux);
        assert_eq!(entry.sample_period_seconds, Some(1));
        assert_eq!(entry.history_samples, HOST_TELEMETRY_HISTORY_SAMPLES);
        assert!(entry.requires_delta && entry.renderable);
    }
    for metric in [
        TelemetryMetric::NetworkReceiveRate,
        TelemetryMetric::NetworkTransmitRate,
    ] {
        let entry = provenance(metric);
        assert_eq!(entry.source, TelemetrySource::HostProcNetDev);
        assert_eq!(entry.host_support, TelemetryHostSupport::Linux);
        assert_eq!(entry.sample_period_seconds, Some(1));
        assert_eq!(entry.history_samples, HOST_TELEMETRY_HISTORY_SAMPLES);
        assert!(entry.requires_delta && entry.renderable);
    }
    let queue = provenance(TelemetryMetric::DaemonQueueDepth);
    assert!(!queue.renderable);
    assert!(queue.precision.contains("client count"));
    let daemon_memory = provenance(TelemetryMetric::DaemonResidentMemory);
    assert!(!daemon_memory.renderable);
    assert!(daemon_memory.precision.contains("assumed 4096-byte"));
}
