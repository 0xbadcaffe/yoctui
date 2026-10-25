use super::*;
impl App {
    pub fn sstate_cache_rows(&self) -> Vec<String> {
        let mut rows = vec!["SSTATE REUSE · observed BitBake evidence".into()];
        rows.extend(self.cache_status_lines());
        match self.build.cache.summary.filter(|s| s.valid()) {
            Some(s) => {
                rows.push(format!(
                    "Wanted {} · local {} · mirrors {} · missed {} · current {}",
                    s.wanted, s.local, s.mirrors, s.missed, s.current
                ));
                rows.push(s.match_percent().map_or_else(
                    || "No cache lookups needed in this summary".into(),
                    |p| format!("Available from cache: {p}% of wanted tasks"),
                ));
                if s.missed > 0 {
                    rows.push("Cache misses require normal builds. Alt+s opens signatures to investigate rebuilds.".into());
                }
            }
            None => rows.push(
                "No sstate summary reported yet; a new build will supply actual reuse statistics."
                    .into(),
            ),
        }
        rows.extend([
            String::new(),
            "CURRENT CONFIGURATION · latest workspace metadata".into(),
        ]);
        for name in [
            "SSTATE_DIR",
            "DL_DIR",
            "SSTATE_MIRRORS",
            "BB_HASHSERVE",
            "BB_HASHSERVE_UPSTREAM",
            "BB_SIGNATURE_HANDLER",
        ] {
            let value = self.workspace.variables.get(name).filter(|v| !v.is_empty());
            let display = if name == "SSTATE_MIRRORS" || name.contains("HASHSERVE") {
                value.map_or_else(
                    || "not reported/configured".into(),
                    |v| {
                        if name == "BB_HASHSERVE" && v == "auto" {
                            "auto (local hash equivalence)".into()
                        } else {
                            "configured (endpoint/credentials hidden)".into()
                        }
                    },
                )
            } else {
                value.cloned().unwrap_or_else(|| "not reported".into())
            };
            rows.push(format!("{name}: {display}"));
        }
        let hash = self
            .workspace
            .variables
            .get("BB_HASHSERVE")
            .is_some_and(|s| !s.is_empty())
            && self
                .workspace
                .variables
                .get("BB_SIGNATURE_HANDLER")
                .is_some_and(|s| s == "OEEquivHash");
        rows.push(
            if hash {
                "Hash equivalence is already enabled."
            } else {
                "Hash equivalence can avoid downstream rebuilds when task output is unchanged."
            }
            .into(),
        );
        rows.extend([
            String::new(),
            "NEXT CONFIRMED IMAGE/RECIPE BUILD".into(),
            self.sstate_cache_build_summary(),
        ]);
        if let Some(p) = self.sstate_cache_profile() {
            rows.push(format!("Scoped to {}", p.build_directory.display()));
            rows.push(format!(
                "Shared sstate: {}",
                if p.settings.sstate_directory.is_empty() {
                    "keep existing"
                } else {
                    &p.settings.sstate_directory
                }
            ));
            rows.push(format!(
                "Shared downloads: {}",
                if p.settings.downloads_directory.is_empty() {
                    "keep existing"
                } else {
                    &p.settings.downloads_directory
                }
            ));
            rows.push(format!(
                "Mirror: {}",
                if p.settings.mirror_url.is_empty() {
                    "keep existing"
                } else {
                    &p.settings.mirror_url
                }
            ));
        }
        rows.extend([
            "e configures mirrors, shared cache/download paths and hash equivalence.".into(),
            "Blank fields preserve current settings. Changes apply only after a build review."
                .into(),
            "Hash equivalence must correspond to the cache/mirror it serves.".into(),
            "Preserve compatible cache/downloads across builds; avoid cleanall/cleansstate.".into(),
            "Mirror configuration does not prove reachability or matching artifacts.".into(),
            "BitBake validates task signatures and determines what can be reused.".into(),
        ]);
        if let Some(reason) = self.sstate_cache_unavailable() {
            rows.push(format!("Profile unavailable: {reason}"));
        }
        rows.extend([
            String::new(),
            "EXISTING MAINTENANCE TOOLS".into(),
            "r inspects available tools; c reviews oe-check-sstate readiness.".into(),
            "d reviews protected cleanup candidates; it never removes files directly.".into(),
            "t opens the original Sstate tools/capability view; [/] switches Maintenance tabs."
                .into(),
            "p returns from the tools view to this cache performance report.".into(),
        ]);
        if let Some(snapshot) = self.maintenance.capability.snapshot() {
            rows.push(format!(
                "Readiness tool: {} · cleanup tool: {}",
                snapshot.supports(MaintenanceTool::OeCheckSstate),
                snapshot.supports(MaintenanceTool::SstateCacheManagement)
            ));
        } else {
            rows.push("Tool availability has not been inspected; press r when needed.".into());
        }
        rows
    }
}
