//! Renderer-independent projections of bounded host telemetry histories.

use crate::{
    App, HOST_TELEMETRY_HISTORY_SAMPLES, HistoryProjection, TelemetryMetric, TelemetryUnit,
    WidgetRole, WidgetState,
};
use std::collections::VecDeque;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelemetrySeriesProjection {
    pub metric: TelemetryMetric,
    pub unit: TelemetryUnit,
    pub history: HistoryProjection,
}

impl TelemetrySeriesProjection {
    pub fn is_supported(&self) -> bool {
        self.history.current.is_some() || !self.history.points.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostTelemetryProjection {
    pub series: Vec<TelemetrySeriesProjection>,
}

impl HostTelemetryProjection {
    pub fn series(&self, metric: TelemetryMetric) -> &TelemetrySeriesProjection {
        self.series
            .iter()
            .find(|series| series.metric == metric)
            .expect("the closed host telemetry projection contains every history metric")
    }
}

fn utilization_percent(total: Option<u64>, available: Option<u64>) -> Option<u64> {
    let (Some(total), Some(available)) = (total, available) else {
        return None;
    };
    if total == 0 || available > total {
        return None;
    }
    u64::try_from(
        u128::from(total - available)
            .saturating_mul(100)
            .checked_div(u128::from(total))?,
    )
    .ok()
}

fn series(
    metric: TelemetryMetric,
    unit: TelemetryUnit,
    label: &str,
    role: WidgetRole,
    current: Option<u64>,
    points: &VecDeque<u64>,
) -> TelemetrySeriesProjection {
    let (state, detail) = match (current, points.is_empty()) {
        (Some(_), _) => (WidgetState::Available, "latest valid sample"),
        (None, false) => (
            WidgetState::Partial,
            "current sample unavailable; retained valid history",
        ),
        (None, true) => (
            WidgetState::Unavailable,
            "current sample and history unavailable",
        ),
    };
    TelemetrySeriesProjection {
        metric,
        unit,
        history: HistoryProjection::bounded(
            label,
            state,
            role,
            current,
            points.iter().copied(),
            HOST_TELEMETRY_HISTORY_SAMPLES,
            detail,
        )
        .with_value_suffix(match unit {
            TelemetryUnit::IntegerPercent => "%",
            TelemetryUnit::BytesPerSecond => " B/s",
            _ => "",
        }),
    }
}

impl App {
    pub fn host_telemetry_projection(&self) -> HostTelemetryProjection {
        let telemetry = &self.host_telemetry;
        let history = &self.host_telemetry_history;
        HostTelemetryProjection {
            series: vec![
                series(
                    TelemetryMetric::HostCpuUtilization,
                    TelemetryUnit::IntegerPercent,
                    "CPU",
                    WidgetRole::Cpu,
                    telemetry
                        .cpu_utilization_percent
                        .map(|value| u64::from(value.min(100))),
                    &history.cpu_percent,
                ),
                series(
                    TelemetryMetric::HostMemoryCapacity,
                    TelemetryUnit::IntegerPercent,
                    "RAM",
                    WidgetRole::Memory,
                    utilization_percent(
                        telemetry.memory_total_bytes,
                        telemetry.memory_available_bytes,
                    ),
                    &history.memory_percent,
                ),
                series(
                    TelemetryMetric::BuildFilesystemCapacity,
                    TelemetryUnit::IntegerPercent,
                    "Build FS",
                    WidgetRole::Progress,
                    utilization_percent(telemetry.disk_total_bytes, telemetry.disk_available_bytes),
                    &history.build_filesystem_percent,
                ),
                series(
                    TelemetryMetric::DiskReadRate,
                    TelemetryUnit::BytesPerSecond,
                    "Disk read",
                    WidgetRole::DiskRead,
                    telemetry.disk_read_bytes_per_second,
                    &history.disk_read_bytes_per_second,
                ),
                series(
                    TelemetryMetric::DiskWriteRate,
                    TelemetryUnit::BytesPerSecond,
                    "Disk write",
                    WidgetRole::DiskWrite,
                    telemetry.disk_write_bytes_per_second,
                    &history.disk_write_bytes_per_second,
                ),
                series(
                    TelemetryMetric::NetworkReceiveRate,
                    TelemetryUnit::BytesPerSecond,
                    "Network RX",
                    WidgetRole::NetworkRx,
                    telemetry.network_receive_bytes_per_second,
                    &history.network_receive_bytes_per_second,
                ),
                series(
                    TelemetryMetric::NetworkTransmitRate,
                    TelemetryUnit::BytesPerSecond,
                    "Network TX",
                    WidgetRole::NetworkTx,
                    telemetry.network_transmit_bytes_per_second,
                    &history.network_transmit_bytes_per_second,
                ),
            ],
        }
    }
}

#[cfg(test)]
#[path = "tests/telemetry_projection/mod.rs"]
mod tests;
