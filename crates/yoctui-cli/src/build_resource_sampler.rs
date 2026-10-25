//! Bounded daemon-host sampling. Run only on a blocking worker, never in rendering.
use crate::host_telemetry::HostTelemetrySampler;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
    time::Instant,
};
use yoctui_model::{
    BuildPerformanceSettings, BuildResourceTelemetry, PressureSample, RecipeMemoryPeak,
};

#[derive(Default)]
pub(crate) struct BuildResourceSampler {
    host: HostTelemetrySampler,
    previous_swap: Option<(u64, u64, Instant)>,
    peaks: BTreeMap<(String, String), u64>,
    build_id: Option<u64>,
    partial: bool,
}
#[derive(Clone)]
pub(crate) struct TaskRoot {
    pub recipe: String,
    pub task: String,
    pub pid: u32,
}
impl BuildResourceSampler {
    pub(crate) fn sample(
        &mut self,
        build_dir: &Path,
        roots: &[TaskRoot],
        build_id: Option<u64>,
        profile: Option<BuildPerformanceSettings>,
    ) -> BuildResourceTelemetry {
        if build_id != self.build_id {
            self.peaks.clear();
            self.partial = false;
            self.build_id = build_id;
        }
        let now = Instant::now();
        let meminfo = fs::read_to_string("/proc/meminfo").unwrap_or_default();
        let value = |name| memory_value(&meminfo, name);
        let swap = fs::read_to_string("/proc/vmstat")
            .ok()
            .and_then(|s| swap_counters(&s));
        let rates = swap.and_then(|(input, output)| {
            let (prev_in, prev_out, time) = self.previous_swap?;
            let elapsed = now.checked_duration_since(time)?;
            Some((
                crate::host_telemetry::bytes_per_second(prev_in, input, elapsed)?,
                crate::host_telemetry::bytes_per_second(prev_out, output, elapsed)?,
            ))
        });
        self.previous_swap = swap.map(|(i, o)| (i, o, now));
        let mut partial = roots.len() > 128;
        let mut budget = 2048;
        for root in roots.iter().take(128) {
            let (rss, incomplete) = task_tree_rss(root.pid, &mut budget);
            partial |= incomplete;
            if let Some(rss) = rss {
                // Limit retained identities; the observation is explicitly sampled/partial.
                let key = (root.recipe.clone(), root.task.clone());
                if self.peaks.len() < 256 || self.peaks.contains_key(&key) {
                    self.peaks
                        .entry(key)
                        .and_modify(|old| *old = (*old).max(rss))
                        .or_insert(rss);
                } else {
                    partial = true;
                    if let Some((smallest, &value)) =
                        self.peaks.iter().min_by_key(|(_, value)| **value)
                        && rss > value
                    {
                        let smallest = smallest.clone();
                        self.peaks.remove(&smallest);
                        self.peaks.insert(key, rss);
                    }
                }
            }
        }
        self.partial |= partial;
        let mut peaks: Vec<_> = self
            .peaks
            .iter()
            .map(
                |((recipe, task), &sampled_peak_rss_bytes)| RecipeMemoryPeak {
                    recipe: recipe.clone(),
                    task: task.clone(),
                    sampled_peak_rss_bytes,
                },
            )
            .collect();
        peaks.sort_by(|a, b| {
            b.sampled_peak_rss_bytes
                .cmp(&a.sampled_peak_rss_bytes)
                .then_with(|| a.recipe.cmp(&b.recipe))
                .then_with(|| a.task.cmp(&b.task))
        });
        peaks.truncate(16);
        BuildResourceTelemetry {
            sampled_unix_ms: crate::unix_ms(),
            host: self.host.sample(build_dir),
            cached_bytes: value("Cached:"),
            swap_total_bytes: value("SwapTotal:"),
            swap_free_bytes: value("SwapFree:"),
            swap_in_bytes_per_second: rates.map(|r| r.0),
            swap_out_bytes_per_second: rates.map(|r| r.1),
            cpu_pressure: pressure("cpu"),
            io_pressure: pressure("io"),
            memory_pressure: pressure("memory"),
            active_profile: profile,
            recipe_peaks: peaks,
            process_scan_partial: self.partial,
        }
    }
}
pub(crate) fn memory_value(input: &str, name: &str) -> Option<u64> {
    let mut fields = input
        .lines()
        .find(|l| l.split_whitespace().next() == Some(name))?
        .split_whitespace();
    fields.next()?;
    crate::host_telemetry::parse_memory_kib(fields.next()?, fields.next()?)?.checked_mul(1024)
}
pub(crate) fn parse_pressure(input: &str) -> Option<PressureSample> {
    let read = |kind: &str| -> Option<u16> {
        let line = input
            .lines()
            .find(|l| l.split_whitespace().next() == Some(kind))?;
        let value = line
            .split_whitespace()
            .find_map(|s| s.strip_prefix("avg10="))?;
        let (whole, fraction) = value.split_once('.')?;
        if fraction.len() != 2 || !fraction.bytes().all(|c| c.is_ascii_digit()) {
            return None;
        }
        let n = whole
            .parse::<u16>()
            .ok()?
            .checked_mul(100)?
            .checked_add(fraction.parse().ok()?)?;
        (n <= 10000).then_some(n)
    };
    Some(PressureSample {
        some_hundredths: read("some")?,
        full_hundredths: read("full"),
    })
}
fn pressure(name: &str) -> Option<PressureSample> {
    parse_pressure(&fs::read_to_string(format!("/proc/pressure/{name}")).ok()?)
}
fn swap_counters(input: &str) -> Option<(u64, u64)> {
    let value = |name: &str| -> Option<u64> {
        input.lines().find_map(|line| {
            let mut fields = line.split_whitespace();
            (fields.next()? == name).then_some(fields.next()?.parse::<u64>().ok()?)
        })
    };
    // sysconf rather than assuming the host uses 4 KiB pages.
    let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    let page = u64::try_from(page).ok().filter(|n| *n > 0)?;
    Some((
        value("pswpin")?.checked_mul(page)?,
        value("pswpout")?.checked_mul(page)?,
    ))
}
fn task_tree_rss(root: u32, budget: &mut usize) -> (Option<u64>, bool) {
    let mut queue = vec![root];
    let mut visited = BTreeSet::new();
    let mut rss = 0u64;
    let mut observed = false;
    let mut partial = false;
    while let Some(pid) = queue.pop() {
        if !visited.insert(pid) {
            continue;
        }
        if *budget == 0 {
            partial = true;
            break;
        }
        *budget -= 1;
        let status = fs::read_to_string(format!("/proc/{pid}/status")).ok();
        if let Some(value) = status.as_deref().and_then(|s| memory_value(s, "VmRSS:")) {
            rss = rss.saturating_add(value);
            observed = true;
        } else {
            partial = true;
        }
        // Kernel-provided child IDs avoid scanning every system process.
        match fs::read_to_string(format!("/proc/{pid}/task/{pid}/children")) {
            Ok(children) => {
                for child in children.split_whitespace().take(2048) {
                    if queue.len() >= 2048 {
                        partial = true;
                        break;
                    }
                    if let Ok(child) = child.parse() {
                        queue.push(child);
                    }
                }
            }
            Err(_) => partial = true,
        }
    }
    (observed.then_some(rss), partial)
}

#[cfg(test)]
#[path = "tests/build_resource_sampler.rs"]
mod tests;
