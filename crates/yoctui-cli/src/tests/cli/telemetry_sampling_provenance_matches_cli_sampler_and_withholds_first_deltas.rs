use super::*;

#[test]
fn telemetry_sampling_provenance_matches_cli_sampler_and_withholds_first_deltas() {
    use yoctui_model::{TelemetryMetric as Metric, TelemetrySource as Source};

    let provenance = |metric| {
        yoctui_model::TELEMETRY_PROVENANCE
            .iter()
            .find(|entry| entry.metric == metric)
            .unwrap()
    };
    for (metric, source) in [
        (Metric::HostCpuUtilization, Source::HostProcStat),
        (
            Metric::HostLogicalCpuCount,
            Source::HostAvailableParallelism,
        ),
        (Metric::HostMemoryCapacity, Source::HostProcMeminfo),
        (
            Metric::BuildFilesystemCapacity,
            Source::BuildFilesystemStatvfs,
        ),
        (Metric::HostLoadAverage, Source::HostProcLoadavg),
    ] {
        let entry = provenance(metric);
        assert_eq!(entry.source, source);
        assert_eq!(
            entry.sample_period_seconds,
            Some(yoctui_model::HOST_TELEMETRY_SAMPLE_PERIOD_SECONDS)
        );
        assert!(entry.renderable);
    }
    for (metric, source) in [
        (Metric::DiskReadRate, Source::HostProcDiskstats),
        (Metric::DiskWriteRate, Source::HostProcDiskstats),
        (Metric::NetworkReceiveRate, Source::HostProcNetDev),
        (Metric::NetworkTransmitRate, Source::HostProcNetDev),
    ] {
        let entry = provenance(metric);
        assert_eq!(entry.source, source);
        assert!(entry.requires_delta && entry.renderable);
    }

    let mut sampler = HostTelemetrySampler::default();
    let first = sampler.sample(Path::new("."));
    assert_eq!(first.cpu_utilization_percent, None);
    assert_eq!(first.disk_read_bytes_per_second, None);
    assert_eq!(first.disk_write_bytes_per_second, None);
    assert_eq!(first.network_receive_bytes_per_second, None);
    assert_eq!(first.network_transmit_bytes_per_second, None);
}
