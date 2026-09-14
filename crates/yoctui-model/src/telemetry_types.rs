//! Telemetry types.
use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HostTelemetry {
    pub cpu_utilization_percent: Option<u8>,
    pub logical_cpu_count: Option<u16>,
    pub memory_total_bytes: Option<u64>,
    pub memory_available_bytes: Option<u64>,
    pub disk_available_bytes: Option<u64>,
    pub disk_total_bytes: Option<u64>,
    pub disk_read_bytes_per_second: Option<u64>,
    pub disk_write_bytes_per_second: Option<u64>,
    pub network_receive_bytes_per_second: Option<u64>,
    pub network_transmit_bytes_per_second: Option<u64>,
    pub load_average_milli: Option<[u32; 3]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitBakeCoexistencePressure {
    Unknown,
    Nominal,
    Busy,
    Oversubscribed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitBakeCoexistenceDiagnostic {
    pub pressure: BitBakeCoexistencePressure,
    pub logical_cpu_count: Option<u16>,
    pub load_one_milli: Option<u32>,
    pub bitbake_threads: Option<u16>,
    pub parallel_make_jobs: Option<u16>,
    pub review_example_jobs: Option<u16>,
    pub reasons: Vec<String>,
}

pub(crate) fn positive_u16(value: &str) -> Option<u16> {
    value.trim().parse::<u16>().ok().filter(|value| *value > 0)
}

pub fn parse_parallel_make_jobs(value: &str) -> Option<u16> {
    let mut fields = value.split_whitespace().peekable();
    let mut jobs = None;
    while let Some(field) = fields.next() {
        if field == "-j" || field == "--jobs" {
            jobs = fields.next().and_then(positive_u16);
        } else if let Some(value) = field.strip_prefix("-j") {
            jobs = positive_u16(value);
        } else if let Some(value) = field.strip_prefix("--jobs=") {
            jobs = positive_u16(value);
        }
    }
    jobs
}

/// Produces a read-only coexistence assessment from already collected state.
///
/// This diagnostic deliberately does not claim that BitBake task threads and
/// each task's make jobs run as a simple product. It flags independently
/// excessive configured limits and sustained host load for user review.
pub fn bitbake_coexistence_diagnostic(
    workspace: &Workspace,
    telemetry: &HostTelemetry,
) -> BitBakeCoexistenceDiagnostic {
    let logical_cpu_count = telemetry.logical_cpu_count;
    let load_one_milli = telemetry.load_average_milli.map(|load| load[0]);
    let bitbake_threads = workspace
        .variables
        .get("BB_NUMBER_THREADS")
        .and_then(|value| positive_u16(value));
    let parallel_make_jobs = workspace
        .variables
        .get("PARALLEL_MAKE")
        .and_then(|value| parse_parallel_make_jobs(value));
    let review_example_jobs = logical_cpu_count.map(|cpus| cpus.saturating_sub(1).max(1));
    let mut reasons = Vec::new();
    let Some(cpus) = logical_cpu_count else {
        reasons.push("logical CPU count is unavailable".into());
        return BitBakeCoexistenceDiagnostic {
            pressure: BitBakeCoexistencePressure::Unknown,
            logical_cpu_count,
            load_one_milli,
            bitbake_threads,
            parallel_make_jobs,
            review_example_jobs,
            reasons,
        };
    };
    let cpus = u32::from(cpus);
    let configured_oversubscription = bitbake_threads
        .is_some_and(|threads| u32::from(threads) > cpus.saturating_mul(2))
        || parallel_make_jobs.is_some_and(|jobs| u32::from(jobs) > cpus.saturating_mul(2));
    let observed_oversubscription =
        load_one_milli.is_some_and(|load| load > cpus.saturating_mul(1_500));
    let configured_busy = bitbake_threads.is_some_and(|threads| u32::from(threads) >= cpus)
        || parallel_make_jobs.is_some_and(|jobs| u32::from(jobs) >= cpus);
    let observed_busy = load_one_milli.is_some_and(|load| load >= cpus.saturating_mul(900));

    if configured_oversubscription {
        reasons.push("a configured parallelism limit exceeds twice the logical CPU count".into());
    }
    if observed_oversubscription {
        reasons
            .push("the one-minute load average exceeds 1.5 runnable tasks per logical CPU".into());
    }
    let pressure = if configured_oversubscription || observed_oversubscription {
        BitBakeCoexistencePressure::Oversubscribed
    } else if configured_busy || observed_busy {
        if configured_busy {
            reasons.push("configured build parallelism can occupy every logical CPU".into());
        }
        if observed_busy {
            reasons
                .push("the one-minute load average is near or above logical CPU capacity".into());
        }
        BitBakeCoexistencePressure::Busy
    } else {
        reasons.push("no sustained or configured oversubscription was detected".into());
        BitBakeCoexistencePressure::Nominal
    };
    BitBakeCoexistenceDiagnostic {
        pressure,
        logical_cpu_count,
        load_one_milli,
        bitbake_threads,
        parallel_make_jobs,
        review_example_jobs,
        reasons,
    }
}

pub const HOST_TELEMETRY_SAMPLE_PERIOD_SECONDS: u16 = 1;
pub const HOST_TELEMETRY_HISTORY_SAMPLES: usize = 60;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TelemetryHistory {
    pub cpu_percent: VecDeque<u64>,
    pub memory_percent: VecDeque<u64>,
    pub build_filesystem_percent: VecDeque<u64>,
    pub disk_read_bytes_per_second: VecDeque<u64>,
    pub disk_write_bytes_per_second: VecDeque<u64>,
    pub network_receive_bytes_per_second: VecDeque<u64>,
    pub network_transmit_bytes_per_second: VecDeque<u64>,
}

impl TelemetryHistory {
    pub(crate) fn push(queue: &mut VecDeque<u64>, sample: u64) {
        queue.push_back(sample);
        while queue.len() > HOST_TELEMETRY_HISTORY_SAMPLES {
            queue.pop_front();
        }
    }

    pub fn record(&mut self, telemetry: &HostTelemetry) {
        if let Some(cpu) = telemetry.cpu_utilization_percent {
            Self::push(&mut self.cpu_percent, u64::from(cpu.min(100)));
        }
        if let (Some(total), Some(available)) = (
            telemetry.memory_total_bytes,
            telemetry.memory_available_bytes,
        ) && total > 0
            && available <= total
        {
            Self::push(
                &mut self.memory_percent,
                u64::try_from(u128::from(total - available) * 100 / u128::from(total))
                    .unwrap_or(100),
            );
        }
        if let (Some(total), Some(available)) =
            (telemetry.disk_total_bytes, telemetry.disk_available_bytes)
            && total > 0
            && available <= total
        {
            Self::push(
                &mut self.build_filesystem_percent,
                u64::try_from(u128::from(total - available) * 100 / u128::from(total))
                    .unwrap_or(100),
            );
        }
        for (queue, sample) in [
            (
                &mut self.disk_read_bytes_per_second,
                telemetry.disk_read_bytes_per_second,
            ),
            (
                &mut self.disk_write_bytes_per_second,
                telemetry.disk_write_bytes_per_second,
            ),
            (
                &mut self.network_receive_bytes_per_second,
                telemetry.network_receive_bytes_per_second,
            ),
            (
                &mut self.network_transmit_bytes_per_second,
                telemetry.network_transmit_bytes_per_second,
            ),
        ] {
            if let Some(sample) = sample {
                Self::push(queue, sample);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TelemetryMetric {
    HostCpuUtilization,
    HostLogicalCpuCount,
    HostMemoryCapacity,
    BuildFilesystemCapacity,
    HostLoadAverage,
    DiskReadRate,
    DiskWriteRate,
    NetworkReceiveRate,
    NetworkTransmitRate,
    DaemonConnectionState,
    DaemonUptime,
    BitBakeState,
    ConnectedClients,
    TerminalSessions,
    ActiveJobs,
    DaemonQueueDepth,
    DaemonResidentMemory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TelemetrySource {
    HostProcStat,
    HostAvailableParallelism,
    HostProcMeminfo,
    BuildFilesystemStatvfs,
    HostProcLoadavg,
    HostProcDiskstats,
    HostProcNetDev,
    ClientReplica,
    DaemonRuntimeClock,
    DaemonSnapshot,
    DaemonTelemetry,
    NotCollected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TelemetryUnit {
    IntegerPercent,
    Count,
    Bytes,
    BytesPerSecond,
    MilliLoad,
    Lifecycle,
    Seconds,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TelemetryHostSupport {
    Portable,
    Unix,
    Linux,
    DaemonProtocol,
    NotCollected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TelemetryProvenance {
    pub metric: TelemetryMetric,
    pub source: TelemetrySource,
    pub unit: TelemetryUnit,
    pub host_support: TelemetryHostSupport,
    pub sample_period_seconds: Option<u16>,
    pub history_samples: usize,
    pub requires_delta: bool,
    pub renderable: bool,
    pub precision: &'static str,
    pub unavailable_behavior: &'static str,
}

pub const TELEMETRY_PROVENANCE: &[TelemetryProvenance] = &[
    TelemetryProvenance {
        metric: TelemetryMetric::HostCpuUtilization,
        source: TelemetrySource::HostProcStat,
        unit: TelemetryUnit::IntegerPercent,
        host_support: TelemetryHostSupport::Linux,
        sample_period_seconds: Some(HOST_TELEMETRY_SAMPLE_PERIOD_SECONDS),
        history_samples: HOST_TELEMETRY_HISTORY_SAMPLES,
        requires_delta: true,
        renderable: true,
        precision: "aggregate non-idle counter delta, truncated to a whole percent",
        unavailable_behavior: "unavailable until two valid active-operation samples exist",
    },
    TelemetryProvenance {
        metric: TelemetryMetric::HostLogicalCpuCount,
        source: TelemetrySource::HostAvailableParallelism,
        unit: TelemetryUnit::Count,
        host_support: TelemetryHostSupport::Portable,
        sample_period_seconds: Some(HOST_TELEMETRY_SAMPLE_PERIOD_SECONDS),
        history_samples: 0,
        requires_delta: false,
        renderable: true,
        precision: "logical parallelism visible to this process",
        unavailable_behavior: "display unknown core count when the runtime query fails or exceeds u16",
    },
    TelemetryProvenance {
        metric: TelemetryMetric::HostMemoryCapacity,
        source: TelemetrySource::HostProcMeminfo,
        unit: TelemetryUnit::Bytes,
        host_support: TelemetryHostSupport::Linux,
        sample_period_seconds: Some(HOST_TELEMETRY_SAMPLE_PERIOD_SECONDS),
        history_samples: HOST_TELEMETRY_HISTORY_SAMPLES,
        requires_delta: false,
        renderable: true,
        precision: "MemTotal and MemAvailable kB converted exactly to bytes",
        unavailable_behavior: "omit the sample unless both values are valid and available does not exceed total",
    },
    TelemetryProvenance {
        metric: TelemetryMetric::BuildFilesystemCapacity,
        source: TelemetrySource::BuildFilesystemStatvfs,
        unit: TelemetryUnit::Bytes,
        host_support: TelemetryHostSupport::Unix,
        sample_period_seconds: Some(HOST_TELEMETRY_SAMPLE_PERIOD_SECONDS),
        history_samples: 0,
        requires_delta: false,
        renderable: true,
        precision: "f_bavail and f_blocks multiplied by filesystem fragment size",
        unavailable_behavior: "display unavailable when the build path or statvfs sample is invalid",
    },
    TelemetryProvenance {
        metric: TelemetryMetric::HostLoadAverage,
        source: TelemetrySource::HostProcLoadavg,
        unit: TelemetryUnit::MilliLoad,
        host_support: TelemetryHostSupport::Linux,
        sample_period_seconds: Some(HOST_TELEMETRY_SAMPLE_PERIOD_SECONDS),
        history_samples: 0,
        requires_delta: false,
        renderable: true,
        precision: "1/5/15-minute kernel values retained to three decimal places",
        unavailable_behavior: "display all three values as unavailable when parsing fails",
    },
    TelemetryProvenance {
        metric: TelemetryMetric::DiskReadRate,
        source: TelemetrySource::HostProcDiskstats,
        unit: TelemetryUnit::BytesPerSecond,
        host_support: TelemetryHostSupport::Linux,
        sample_period_seconds: Some(HOST_TELEMETRY_SAMPLE_PERIOD_SECONDS),
        history_samples: HOST_TELEMETRY_HISTORY_SAMPLES,
        requires_delta: true,
        renderable: true,
        precision: "matched build-filesystem device sectors multiplied by 512, divided by measured interval",
        unavailable_behavior: "omit samples on first observation, reset, device change, overflow, zero interval, or unmatched device",
    },
    TelemetryProvenance {
        metric: TelemetryMetric::DiskWriteRate,
        source: TelemetrySource::HostProcDiskstats,
        unit: TelemetryUnit::BytesPerSecond,
        host_support: TelemetryHostSupport::Linux,
        sample_period_seconds: Some(HOST_TELEMETRY_SAMPLE_PERIOD_SECONDS),
        history_samples: HOST_TELEMETRY_HISTORY_SAMPLES,
        requires_delta: true,
        renderable: true,
        precision: "matched build-filesystem device sectors multiplied by 512, divided by measured interval",
        unavailable_behavior: "omit samples on first observation, reset, device change, overflow, zero interval, or unmatched device",
    },
    TelemetryProvenance {
        metric: TelemetryMetric::NetworkReceiveRate,
        source: TelemetrySource::HostProcNetDev,
        unit: TelemetryUnit::BytesPerSecond,
        host_support: TelemetryHostSupport::Linux,
        sample_period_seconds: Some(HOST_TELEMETRY_SAMPLE_PERIOD_SECONDS),
        history_samples: HOST_TELEMETRY_HISTORY_SAMPLES,
        requires_delta: true,
        renderable: true,
        precision: "IPv4 default-route interface byte counter delta divided by measured interval",
        unavailable_behavior: "omit samples on first observation, reset, interface change/disappearance, overflow, zero interval, or absent IPv4 default route",
    },
    TelemetryProvenance {
        metric: TelemetryMetric::NetworkTransmitRate,
        source: TelemetrySource::HostProcNetDev,
        unit: TelemetryUnit::BytesPerSecond,
        host_support: TelemetryHostSupport::Linux,
        sample_period_seconds: Some(HOST_TELEMETRY_SAMPLE_PERIOD_SECONDS),
        history_samples: HOST_TELEMETRY_HISTORY_SAMPLES,
        requires_delta: true,
        renderable: true,
        precision: "IPv4 default-route interface byte counter delta divided by measured interval",
        unavailable_behavior: "omit samples on first observation, reset, interface change/disappearance, overflow, zero interval, or absent IPv4 default route",
    },
    TelemetryProvenance {
        metric: TelemetryMetric::DaemonConnectionState,
        source: TelemetrySource::ClientReplica,
        unit: TelemetryUnit::Lifecycle,
        host_support: TelemetryHostSupport::DaemonProtocol,
        sample_period_seconds: None,
        history_samples: 0,
        requires_delta: false,
        renderable: true,
        precision: "current client replica lifecycle",
        unavailable_behavior: "render the exact disconnected, synchronizing, stale, or current state",
    },
    TelemetryProvenance {
        metric: TelemetryMetric::DaemonUptime,
        source: TelemetrySource::DaemonRuntimeClock,
        unit: TelemetryUnit::Seconds,
        host_support: TelemetryHostSupport::DaemonProtocol,
        sample_period_seconds: Some(1),
        history_samples: 0,
        requires_delta: false,
        renderable: true,
        precision: "whole seconds from saturating wall-clock difference to daemon start",
        unavailable_behavior: "display unavailable until current daemon telemetry arrives",
    },
    TelemetryProvenance {
        metric: TelemetryMetric::BitBakeState,
        source: TelemetrySource::DaemonSnapshot,
        unit: TelemetryUnit::Lifecycle,
        host_support: TelemetryHostSupport::DaemonProtocol,
        sample_period_seconds: None,
        history_samples: 0,
        requires_delta: false,
        renderable: true,
        precision: "exact lifecycle from the current daemon journal snapshot",
        unavailable_behavior: "render disconnected or stale replica state rather than reuse old authority",
    },
    TelemetryProvenance {
        metric: TelemetryMetric::ConnectedClients,
        source: TelemetrySource::DaemonSnapshot,
        unit: TelemetryUnit::Count,
        host_support: TelemetryHostSupport::DaemonProtocol,
        sample_period_seconds: None,
        history_samples: 0,
        requires_delta: false,
        renderable: true,
        precision: "exact current snapshot client inventory length",
        unavailable_behavior: "display unavailable when the daemon replica is not current",
    },
    TelemetryProvenance {
        metric: TelemetryMetric::TerminalSessions,
        source: TelemetrySource::DaemonSnapshot,
        unit: TelemetryUnit::Count,
        host_support: TelemetryHostSupport::DaemonProtocol,
        sample_period_seconds: None,
        history_samples: 0,
        requires_delta: false,
        renderable: true,
        precision: "exact current snapshot PTY inventory length",
        unavailable_behavior: "display unavailable when the daemon replica is not current",
    },
    TelemetryProvenance {
        metric: TelemetryMetric::ActiveJobs,
        source: TelemetrySource::DaemonTelemetry,
        unit: TelemetryUnit::Count,
        host_support: TelemetryHostSupport::DaemonProtocol,
        sample_period_seconds: Some(1),
        history_samples: 0,
        requires_delta: false,
        renderable: true,
        precision: "Connecting, Running, and Stopping daemon job count",
        unavailable_behavior: "display unavailable until current daemon telemetry arrives",
    },
    TelemetryProvenance {
        metric: TelemetryMetric::DaemonQueueDepth,
        source: TelemetrySource::DaemonTelemetry,
        unit: TelemetryUnit::Count,
        host_support: TelemetryHostSupport::DaemonProtocol,
        sample_period_seconds: Some(1),
        history_samples: 0,
        requires_delta: false,
        renderable: false,
        precision: "wire field currently mirrors connected client count, not queued work",
        unavailable_behavior: "do not render as queue depth",
    },
    TelemetryProvenance {
        metric: TelemetryMetric::DaemonResidentMemory,
        source: TelemetrySource::DaemonTelemetry,
        unit: TelemetryUnit::Bytes,
        host_support: TelemetryHostSupport::Unix,
        sample_period_seconds: Some(1),
        history_samples: 0,
        requires_delta: false,
        renderable: false,
        precision: "Linux resident pages multiplied by an assumed 4096-byte page size",
        unavailable_behavior: "diagnostic only; do not render as precise memory until page size is authoritative",
    },
];
