//! Opt-in build policy and daemon-host observations. No host I/O in the model.
use crate::{App, Effect, FocusTarget, Screen};
mod report;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildPerformanceSettings {
    pub cpu: u32,
    pub io: u32,
    pub memory: u32,
}
impl Default for BuildPerformanceSettings {
    fn default() -> Self {
        Self {
            cpu: 15_000,
            io: 15_000,
            memory: 15_000,
        }
    }
}
impl BuildPerformanceSettings {
    pub fn valid(self) -> bool {
        [self.cpu, self.io, self.memory]
            .into_iter()
            .all(|n| (1..=1_000_000).contains(&n))
    }
    pub fn summary(self) -> String {
        format!(
            "Balanced: PSI CPU={} IO={} memory={}; existing task/make limits retained",
            self.cpu, self.io, self.memory
        )
    }
}

/// Percentage of time stalled over the last ten seconds, in hundredths of a percent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PressureSample {
    pub some_hundredths: u16,
    pub full_hundredths: Option<u16>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipeMemoryPeak {
    pub recipe: String,
    pub task: String,
    pub sampled_peak_rss_bytes: u64,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildResourceTelemetry {
    pub sampled_unix_ms: u64,
    pub host: crate::HostTelemetry,
    pub cached_bytes: Option<u64>,
    pub swap_total_bytes: Option<u64>,
    pub swap_free_bytes: Option<u64>,
    pub swap_in_bytes_per_second: Option<u64>,
    pub swap_out_bytes_per_second: Option<u64>,
    pub cpu_pressure: Option<PressureSample>,
    pub io_pressure: Option<PressureSample>,
    pub memory_pressure: Option<PressureSample>,
    pub active_profile: Option<BuildPerformanceSettings>,
    /// Bounded top 16 sampled task process-tree RSS peaks for the current/last build.
    /// RSS can count shared pages repeatedly and sampling can miss short-lived peaks.
    pub recipe_peaks: Vec<RecipeMemoryPeak>,
    pub process_scan_partial: bool,
}

impl BuildResourceTelemetry {
    pub fn valid(&self) -> bool {
        let pressure = |p: Option<PressureSample>| {
            p.is_none_or(|p| {
                p.some_hundredths <= 10000 && p.full_hundredths.is_none_or(|n| n <= 10000)
            })
        };
        self.sampled_unix_ms > 0
            && self.recipe_peaks.len() <= 16
            && self.recipe_peaks.iter().all(|p| {
                [&p.recipe, &p.task]
                    .iter()
                    .all(|s| !s.is_empty() && s.len() <= 512 && !s.chars().any(char::is_control))
            })
            && self
                .active_profile
                .is_none_or(BuildPerformanceSettings::valid)
            && [self.cpu_pressure, self.io_pressure, self.memory_pressure]
                .into_iter()
                .all(pressure)
            && self
                .host
                .memory_available_bytes
                .zip(self.host.memory_total_bytes)
                .is_none_or(|(free, total)| free <= total)
            && self
                .swap_free_bytes
                .zip(self.swap_total_bytes)
                .is_none_or(|(free, total)| free <= total)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BuildPerformanceState {
    pub scroll: usize,
    pub field: usize,
    pub visible_rows: usize,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildPerformanceAction {
    Balanced,
    Defaults,
    Field(i32),
    Adjust(i32),
    Scroll(i32),
    First,
    Last,
}
pub(crate) fn reduce(app: &mut App, action: BuildPerformanceAction) -> Option<Effect> {
    if app.screen != Screen::BuildPerformance
        || app.focus != FocusTarget::Workspace
        || app.active_dialog().is_some()
        || app.menu.is_open()
        || app.command_palette_open
    {
        return None;
    }
    let maximum = app
        .build_performance_rows()
        .len()
        .saturating_sub(app.build_performance.visible_rows.max(1));
    match action {
        BuildPerformanceAction::Balanced => {
            if let Some(reason) = app.build_performance_unavailable() {
                app.notification = Some(reason);
            } else {
                app.preferences.build_performance = Some(BuildPerformanceSettings::default());
                app.settings_dirty = true;
                app.notification = Some(
                    "Balanced selected for future confirmed builds; running work is unchanged"
                        .into(),
                );
                return Some(Effect::PersistSettings);
            }
        }
        BuildPerformanceAction::Defaults => {
            app.preferences.build_performance = None;
            app.settings_dirty = true;
            app.notification = Some("Future builds use the existing BitBake configuration".into());
            return Some(Effect::PersistSettings);
        }
        BuildPerformanceAction::Field(delta) => {
            app.build_performance.field =
                crate::shifted_index(app.build_performance.field, delta as isize, 3);
            app.reveal_build_performance_field();
        }
        BuildPerformanceAction::Adjust(delta) => {
            if let Some(settings) = app.preferences.build_performance.as_mut() {
                let value = match app.build_performance.field {
                    0 => &mut settings.cpu,
                    1 => &mut settings.io,
                    _ => &mut settings.memory,
                };
                *value = value.saturating_add_signed(delta).clamp(1, 1_000_000);
                app.reveal_build_performance_field();
                app.settings_dirty = true;
                return Some(Effect::PersistSettings);
            }
        }
        BuildPerformanceAction::Scroll(delta) => {
            app.build_performance.scroll =
                crate::shifted_index(app.build_performance.scroll, delta as isize, maximum + 1);
        }
        BuildPerformanceAction::First => app.build_performance.scroll = 0,
        BuildPerformanceAction::Last => app.build_performance.scroll = maximum,
    }
    None
}
impl App {
    fn reveal_build_performance_field(&mut self) {
        let prefix = format!("> {} BB_PRESSURE_", self.build_performance.field + 1);
        if let Some(row) = self
            .build_performance_rows()
            .iter()
            .position(|s| s.starts_with(&prefix))
        {
            let visible = self.build_performance.visible_rows.max(1);
            self.build_performance.scroll = self
                .build_performance
                .scroll
                .min(row)
                .max(row.saturating_add(1).saturating_sub(visible));
        }
    }
    pub fn build_performance_unavailable(&self) -> Option<String> {
        if self.daemon.status != crate::ClientReplicaStatus::Current {
            return Some("Connect to a current daemon session before selecting Balanced".into());
        }
        if !self.build_environment.connected() || self.workspace.build_dir.is_none() {
            return Some("Connect to a configured daemon build environment first".into());
        }
        let supported = self
            .workspace
            .bitbake_version
            .as_deref()
            .is_some_and(|version| {
                let mut parts = version.split('.');
                let major = parts.next().and_then(|n| n.parse::<u32>().ok());
                let minor = parts.next().and_then(|n| n.parse::<u32>().ok());
                major.zip(minor).is_some_and(|version| version >= (2, 2))
            });
        if !supported {
            return Some("Balanced requires a reported BitBake version of 2.2 or newer".into());
        }
        let resource = self
            .daemon
            .telemetry
            .as_ref()
            .and_then(|t| t.build_resources.as_ref());
        match resource {
            Some(r) if r.cpu_pressure.is_some() && r.io_pressure.is_some() && r.memory_pressure.is_some() => None,
            _ => Some("Balanced requires current daemon-host PSI telemetry; update/connect the daemon and wait for a sample".into()),
        }
    }
    pub fn build_performance_profile_label(&self) -> String {
        self.preferences.build_performance.map_or_else(
            || "Existing BitBake configuration".into(),
            BuildPerformanceSettings::summary,
        )
    }
}

#[cfg(test)]
#[path = "tests/build_performance.rs"]
mod tests;
