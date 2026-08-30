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
mod tests {
    use super::*;
    use crate::{Action, HostTelemetry, update};

    #[test]
    fn ux_telemetry_projection_preserves_units_roles_zero_and_missing_samples() {
        let mut app = App::new(8, 1_024);
        let _ = update(
            &mut app,
            Action::HostTelemetryUpdated(HostTelemetry {
                cpu_utilization_percent: Some(0),
                memory_total_bytes: Some(8_000),
                memory_available_bytes: Some(2_000),
                disk_read_bytes_per_second: Some(0),
                network_receive_bytes_per_second: Some(4_096),
                ..HostTelemetry::default()
            }),
        );
        let _ = update(
            &mut app,
            Action::HostTelemetryUpdated(HostTelemetry {
                memory_total_bytes: Some(8_000),
                memory_available_bytes: Some(4_000),
                ..HostTelemetry::default()
            }),
        );

        let projection = app.host_telemetry_projection();
        let cpu = projection.series(TelemetryMetric::HostCpuUtilization);
        assert_eq!(cpu.unit, TelemetryUnit::IntegerPercent);
        assert_eq!(cpu.history.role, WidgetRole::Cpu);
        assert_eq!(cpu.history.current, None);
        assert_eq!(cpu.history.points, [0]);
        assert_eq!(cpu.history.state, WidgetState::Partial);

        let memory = projection.series(TelemetryMetric::HostMemoryCapacity);
        assert_eq!(memory.history.current, Some(50));
        assert_eq!(memory.history.points, [75, 50]);

        let read = projection.series(TelemetryMetric::DiskReadRate);
        assert_eq!(read.unit, TelemetryUnit::BytesPerSecond);
        assert_eq!(read.history.role, WidgetRole::DiskRead);
        assert_eq!(read.history.current, None);
        assert_eq!(read.history.points, [0]);
        assert_eq!(read.history.state, WidgetState::Partial);

        let transmit = projection.series(TelemetryMetric::NetworkTransmitRate);
        assert_eq!(transmit.history.state, WidgetState::Unavailable);
        assert!(transmit.history.points.is_empty());
    }

    #[test]
    fn ux_telemetry_projection_bounds_large_inputs_and_rejects_invalid_capacity() {
        let mut app = App::new(8, 1_024);
        app.host_telemetry.cpu_utilization_percent = Some(u8::MAX);
        app.host_telemetry.memory_total_bytes = Some(u64::MAX);
        app.host_telemetry.memory_available_bytes = Some(u64::MAX - 1);
        app.host_telemetry_history.cpu_percent.extend(0..10_000);
        app.host_telemetry_history.memory_percent.extend(0..10_000);

        let projection = app.host_telemetry_projection();
        let cpu = projection.series(TelemetryMetric::HostCpuUtilization);
        assert_eq!(cpu.history.current, Some(100));
        assert_eq!(cpu.history.points.len(), HOST_TELEMETRY_HISTORY_SAMPLES);
        assert_eq!(cpu.history.points[0], 9_940);
        let memory = projection.series(TelemetryMetric::HostMemoryCapacity);
        assert_eq!(memory.history.current, Some(0));
        assert_eq!(memory.history.points.len(), HOST_TELEMETRY_HISTORY_SAMPLES);

        app.host_telemetry.memory_total_bytes = Some(0);
        app.host_telemetry.memory_available_bytes = Some(1);
        let invalid = app.host_telemetry_projection();
        assert_eq!(
            invalid
                .series(TelemetryMetric::HostMemoryCapacity)
                .history
                .state,
            WidgetState::Partial
        );
    }
}
