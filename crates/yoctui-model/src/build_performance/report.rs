use super::*;

fn size(value: Option<u64>) -> String {
    value.map_or_else(
        || "unavailable".into(),
        |n| format!("{:.2} GiB", n as f64 / 1_073_741_824.0),
    )
}
fn pressure(value: Option<PressureSample>) -> String {
    value.map_or_else(
        || "unavailable".into(),
        |p| {
            format!(
                "some {:.2}% · full {}",
                f64::from(p.some_hundredths) / 100.0,
                p.full_hundredths
                    .map_or_else(|| "n/a".into(), |f| format!("{:.2}%", f64::from(f) / 100.0))
            )
        },
    )
}
impl App {
    pub fn build_performance_rows(&self) -> Vec<String> {
        let mut rows = vec!["HOST RESOURCES · sampled on the daemon".into()];
        let resources = self
            .daemon
            .telemetry
            .as_ref()
            .and_then(|t| t.build_resources.as_ref());
        if let Some(r) = resources {
            rows.push(format!(
                "Sample Unix ms: {} · connection: {:?}",
                r.sampled_unix_ms, self.daemon.status
            ));
            rows.push(format!(
                "RAM total {} · available {}",
                size(r.host.memory_total_bytes),
                size(r.host.memory_available_bytes)
            ));
            rows.push(format!(
                "Filesystem cache {} · reusable cache is not a leak",
                size(r.cached_bytes)
            ));
            rows.push(format!(
                "Swap total {} · free {}",
                size(r.swap_total_bytes),
                size(r.swap_free_bytes)
            ));
            let rate = |n: Option<u64>| {
                n.map_or_else(
                    || "unavailable".into(),
                    |n| format!("{:.1} KiB/s", n as f64 / 1024.0),
                )
            };
            rows.push(format!(
                "Swap in {} · out {}",
                rate(r.swap_in_bytes_per_second),
                rate(r.swap_out_bytes_per_second)
            ));
            rows.push(format!(
                "CPU {} · disk free {}",
                r.host
                    .cpu_utilization_percent
                    .map_or_else(|| "unavailable".into(), |v| format!("{v}%")),
                size(r.host.disk_available_bytes)
            ));
            rows.push("PRESSURE · time stalled, 10-second average".into());
            rows.push(format!("CPU: {}", pressure(r.cpu_pressure)));
            rows.push(format!("I/O: {}", pressure(r.io_pressure)));
            rows.push(format!("Memory: {}", pressure(r.memory_pressure)));
            rows.push(
                "PSI percentages above are observations, not the raw scheduler thresholds.".into(),
            );
            rows.push(format!(
                "Current/last job requested policy: {}",
                r.active_profile.map_or_else(
                    || "existing BitBake configuration".into(),
                    BuildPerformanceSettings::summary
                )
            ));
        } else {
            rows.push(
                "Daemon-host measurements unavailable; waiting for a compatible daemon sample."
                    .into(),
            );
            rows.push("Client-machine memory is not substituted for build-host memory.".into());
        }
        rows.extend([
            String::new(),
            "NEXT CONFIRMED BUILD".into(),
            self.build_performance_profile_label(),
        ]);
        if self.preferences.build_performance.is_none() {
            rows.push(
                "Balanced defaults below are inactive; existing settings are preserved.".into(),
            );
        }
        let settings = self.preferences.build_performance.unwrap_or_default();
        for (index, (name, value)) in [
            ("CPU", settings.cpu),
            ("IO", settings.io),
            ("MEMORY", settings.memory),
        ]
        .into_iter()
        .enumerate()
        {
            rows.push(format!(
                "{} {} BB_PRESSURE_MAX_{name} = {value}",
                if index == self.build_performance.field {
                    ">"
                } else {
                    " "
                },
                index + 1
            ));
        }
        rows.extend([
            "Balanced delays new tasks under pressure; running tasks keep their limits.".into(),
            "Existing task/make/parser limits remain effective. This is not a RAM cap.".into(),
            "Higher thresholds throttle less. Tune using a measured build.".into(),
            "No local.conf edits; the daemon supplies a reviewed BitBake postread profile.".into(),
            "Profile affects future confirmed builds from this UI; CLI/raw commands retain their own configuration.".into(),
        ]);
        if let Some(reason) = self.build_performance_unavailable() {
            rows.push(format!("Balanced unavailable: {reason}"));
        }
        rows.extend([
            String::new(),
            "CACHE HEALTH · authoritative workspace and build events".into(),
        ]);
        for (name, absent) in [
            ("SSTATE_DIR", "directory not reported"),
            ("DL_DIR", "directory not reported"),
            ("SSTATE_MIRRORS", "no shared mirror configured"),
            ("BB_HASHSERVE", "no hash server reported"),
            (
                "BB_HASHSERVE_UPSTREAM",
                "no upstream hash server configured",
            ),
            ("BB_SIGNATURE_HANDLER", "signature handler not reported"),
        ] {
            let value = self
                .workspace
                .variables
                .get(name)
                .filter(|v| !v.trim().is_empty());
            // Mirror URLs may contain credentials. Only show their presence/count.
            let display = if name == "SSTATE_MIRRORS" || name.contains("HASHSERVE") {
                value.map_or_else(
                    || absent.into(),
                    |_| "configured (credentials/URL hidden)".into(),
                )
            } else {
                value.map_or_else(|| absent.into(), Clone::clone)
            };
            rows.push(format!("{name}: {display}"));
        }
        rows.extend(self.cache_status_lines());
        if let Some(summary) = self.build.cache.summary
            && let Some(percent) = summary.match_percent()
        {
            rows.push(format!(
                "Sstate availability: {percent}% of wanted tasks available locally/on mirrors"
            ));
        }
        let cache = self.overview_cache();
        rows.push(format!(
            "Setscene completed {} · failed {} · active {}",
            cache.sstate_hits, cache.sstate_misses, cache.sstate_active
        ));
        rows.push(
            "Setscene counts are not a cache hit ratio; review Insights for sstate summaries."
                .into(),
        );
        rows.push("Reuse compatible sstate/downloads; avoid cleanall/cleansstate during normal iteration.".into());
        rows.extend([
            String::new(),
            "MEMORY-HEAVY RECIPES · sampled process-tree RSS peaks".into(),
        ]);
        if let Some(r) = resources
            && !r.recipe_peaks.is_empty()
        {
            for peak in r.recipe_peaks.iter().take(16) {
                rows.push(format!(
                    "{} {}: {}",
                    peak.recipe,
                    peak.task,
                    size(Some(peak.sampled_peak_rss_bytes))
                ));
            }
            if r.process_scan_partial {
                rows.push(
                    "Partial process sample: scan limits, permissions or exiting processes.".into(),
                );
            }
        } else {
            rows.push("No task memory observations yet; run a build to collect samples.".into());
        }
        rows.extend([
            "Samples can miss short-lived peaks; RSS can double-count shared pages.".into(),
            "Use these observations to reduce PARALLEL_MAKE for expensive recipes.".into(),
            "Cache checks show configuration, not mirror reachability or compatibility.".into(),
        ]);
        rows
    }
}
