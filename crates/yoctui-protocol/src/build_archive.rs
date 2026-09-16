//! Bounded versioned build-history file format; never a live replica.
use serde::{Deserialize, Serialize};
use yoctui_model::SavedBuild;
pub const MAX_ARCHIVE_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_ARCHIVE_BUILDS: usize = 32;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildArchive {
    pub schema_version: u32,
    pub builds: Vec<SavedBuild>,
}
impl Default for BuildArchive {
    fn default() -> Self {
        Self {
            schema_version: 1,
            builds: Vec::new(),
        }
    }
}
impl BuildArchive {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema_version != 1 {
            return Err("unsupported build archive schema");
        }
        if self.builds.len() > MAX_ARCHIVE_BUILDS {
            return Err("too many saved builds");
        }
        let mut ids = std::collections::HashSet::new();
        for build in &self.builds {
            if build.id.is_empty() || !ids.insert(&build.id) {
                return Err("duplicate or empty saved build identity");
            }
            if build.id.len() > 128
                || build.logs.len() > 256
                || build.tasks.len() > 256
                || build.limitations.len() > 16
            {
                return Err("saved build exceeds retention bounds");
            }
            let strings = [&build.id, &build.target]
                .into_iter()
                .chain(build.machine.iter())
                .chain(build.source.iter())
                .chain(build.build_dir.iter())
                .chain(build.limitations.iter())
                .chain(build.logs.iter().map(|l| &l.message))
                .chain(
                    build
                        .tasks
                        .iter()
                        .flat_map(|t| [&t.recipe, &t.task, &t.status]),
                );
            if strings.into_iter().any(|s| {
                s.len() > 4096 || s.chars().any(|c| c.is_control() && c != '\n' && c != '\t')
            }) {
                return Err("invalid saved build text");
            }
        }
        Ok(())
    }
    pub fn remember(&mut self, build: SavedBuild) {
        self.builds.retain(|b| b.id != build.id);
        self.builds.insert(0, build);
        self.builds.truncate(MAX_ARCHIVE_BUILDS);
        while self.builds.len() > 1
            && serde_json::to_vec(self).is_ok_and(|v| v.len() > MAX_ARCHIVE_BYTES)
        {
            self.builds.pop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn archive_rejects_future_schema_and_unbounded_records() {
        assert!(
            BuildArchive {
                schema_version: 2,
                builds: Vec::new()
            }
            .validate()
            .is_err()
        );
        let record = SavedBuild {
            id: "one".into(),
            target: "image".into(),
            machine: None,
            source: None,
            build_dir: None,
            outcome: yoctui_model::SavedBuildOutcome::Incomplete,
            saved_unix_ms: 0,
            started_unix_ms: None,
            finished_unix_ms: None,
            logs: Vec::new(),
            tasks: Vec::new(),
            limitations: Vec::new(),
        };
        let mut archive = BuildArchive {
            schema_version: 1,
            builds: vec![record.clone(); 33],
        };
        assert!(archive.validate().is_err());
        archive.builds = vec![record];
        archive.builds[0].target = "x".repeat(4097);
        assert!(archive.validate().is_err());
        archive.builds[0].target = "unsafe\x1b".into();
        assert!(archive.validate().is_err());
    }
}
