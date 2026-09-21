//! Observed build cache facts, independent of bounded task history.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SstateSummary {
    pub wanted: u64,
    pub local: u64,
    pub mirrors: u64,
    pub missed: u64,
    pub current: u64,
}

impl SstateSummary {
    pub fn valid(self) -> bool {
        self.local
            .checked_add(self.mirrors)
            .and_then(|v| v.checked_add(self.missed))
            == Some(self.wanted)
    }

    pub fn match_percent(self) -> Option<u8> {
        (self.valid() && self.wanted > 0).then(|| {
            ((u128::from(self.local) + u128::from(self.mirrors)) * 100 / u128::from(self.wanted))
                as u8
        })
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildCacheState {
    pub summary: Option<SstateSummary>,
    pub fetch_completed: usize,
    pub fetch_failed: usize,
    pub setscene_completed: usize,
    pub setscene_failed: usize,
}

impl BuildCacheState {
    pub fn record_outcome(&mut self, task: &str, success: bool) {
        let counter = match (task, success) {
            ("do_fetch", true) => &mut self.fetch_completed,
            ("do_fetch", false) => &mut self.fetch_failed,
            (task, true) if task.ends_with("_setscene") => &mut self.setscene_completed,
            (task, false) if task.ends_with("_setscene") => &mut self.setscene_failed,
            _ => return,
        };
        *counter = counter.saturating_add(1);
    }
}

impl crate::App {
    /// Display observations, never a promise that an offline build can finish.
    pub fn cache_status_lines(&self) -> [String; 3] {
        let stale = if self.is_offline() {
            " (last observed)"
        } else {
            ""
        };
        let sstate = match self.build.cache.summary.filter(|summary| summary.valid()) {
            Some(summary) => format!(
                "Sstate: {} local + {} mirrors / {} wanted; {} current{stale}",
                summary.local, summary.mirrors, summary.wanted, summary.current
            ),
            None => "Sstate: summary not reported".into(),
        };
        let active = self
            .tasks
            .values()
            .filter(|task| task.task == "do_fetch" && task.state == crate::TaskState::Active)
            .count();
        let downloads = format!(
            "Downloads: {} completed, {} failed, {active} active{stale}",
            self.build.cache.fetch_completed, self.build.cache.fetch_failed
        );
        let policy = match (
            self.workspace
                .variables
                .get("BB_NO_NETWORK")
                .and_then(|value| cache_policy_flag(value)),
            self.workspace
                .variables
                .get("BB_FETCH_PREMIRRORONLY")
                .and_then(|value| cache_policy_flag(value)),
        ) {
            (Some(true), _) => "disabled",
            (Some(false), Some(true)) => "premirrors only",
            (Some(false), Some(false)) => "allowed",
            _ => "unknown",
        };
        [
            sstate,
            downloads,
            format!("Network: {policy}; offline readiness: unverified"),
        ]
    }
}

fn cache_policy_flag(value: &str) -> Option<bool> {
    match value.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "y" => Some(true),
        "0" | "false" | "no" | "n" | "" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
#[path = "tests/build_cache/mod.rs"]
mod tests;
