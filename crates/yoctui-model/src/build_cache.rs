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
mod tests {
    use super::*;
    #[test]
    fn cache_policy_accepts_bitbake_booleans_and_keeps_missing_unknown() {
        for (value, expected) in [
            ("YES", Some(true)),
            ("false", Some(false)),
            ("", Some(false)),
            ("invalid", None),
        ] {
            assert_eq!(cache_policy_flag(value), expected);
        }
        let mut app = crate::App::new(16, 4096);
        assert!(app.cache_status_lines()[2].contains("Network: unknown"));
        app.workspace
            .variables
            .insert("BB_NO_NETWORK".into(), "false".into());
        app.workspace
            .variables
            .insert("BB_FETCH_PREMIRRORONLY".into(), "yes".into());
        assert!(app.cache_status_lines()[2].contains("premirrors only"));
        app.workspace
            .variables
            .insert("BB_FETCH_PREMIRRORONLY".into(), "0".into());
        assert!(app.cache_status_lines()[2].contains("Network: allowed"));
    }

    #[test]
    fn cache_summary_rejects_inconsistent_and_overflowing_counts() {
        let summary = SstateSummary {
            wanted: 10,
            local: 3,
            mirrors: 2,
            missed: 5,
            current: 8,
        };
        assert_eq!(summary.match_percent(), Some(50));
        assert!(
            !SstateSummary {
                missed: 6,
                ..summary
            }
            .valid()
        );
        assert!(
            !SstateSummary {
                local: u64::MAX,
                ..summary
            }
            .valid()
        );
        assert_eq!(
            SstateSummary {
                wanted: 0,
                local: 0,
                mirrors: 0,
                missed: 0,
                current: 8
            }
            .match_percent(),
            None
        );
    }

    #[test]
    fn cache_outcomes_are_cumulative_not_offline_readiness() {
        let mut cache = BuildCacheState::default();
        for _ in 0..5000 {
            cache.record_outcome("do_fetch", true);
        }
        cache.record_outcome("do_fetch", false);
        cache.record_outcome("do_compile_setscene", true);
        cache.record_outcome("do_compile", false);
        assert_eq!(cache.fetch_completed, 5000);
        assert_eq!(cache.fetch_failed, 1);
        assert_eq!(cache.setscene_completed, 1);
        assert_eq!(cache.setscene_failed, 0);
        assert_eq!(cache.summary, None);
    }

    #[test]
    fn cache_reducer_retains_evicted_outcomes_deduplicates_and_resets() {
        use crate::{Action, App, TaskId};
        let mut app = App::new(16, 4096);
        for index in 0..crate::MAX_COMPLETED_TASKS + 5 {
            let id = TaskId(format!("recipe-{index}:do_fetch"));
            crate::update(
                &mut app,
                Action::TaskCompleted {
                    id: id.clone(),
                    success: true,
                },
            );
            crate::update(&mut app, Action::TaskCompleted { id, success: true });
        }
        assert_eq!(
            app.build.cache.fetch_completed,
            crate::MAX_COMPLETED_TASKS + 5
        );
        assert_eq!(
            app.overview_cache().fetch_completed,
            crate::MAX_COMPLETED_TASKS + 5
        );
        assert!(app.cache_status_lines()[2].contains("unverified"));
        app.workspace
            .variables
            .insert("BB_NO_NETWORK".into(), "1".into());
        assert!(app.cache_status_lines()[2].contains("disabled"));
        crate::update(
            &mut app,
            Action::BuildRequested {
                target: Some("image".into()),
            },
        );
        assert_eq!(app.build.cache, BuildCacheState::default());
    }
}
