use super::*;
use crate::test_support::write_executable;
use std::{
    env,
    sync::atomic::{AtomicU64, Ordering},
};
use yoctui_model::{CapabilityCatalog, CapabilityId, IdentityAuthority, ToolIdentity};

static NEXT: AtomicU64 = AtomicU64::new(1);

mod backend_probe_report_requires_exact_identity_and_unique_known_tokens;

mod backend_probe_process_ignores_stderr_and_fails_closed;

struct Fixture {
    root: PathBuf,
    tool: PathBuf,
}

impl Fixture {
    fn new(body: &str) -> Self {
        Self::new_tool(CapabilityToolId::Devtool, body)
    }

    fn new_tool(tool_id: CapabilityToolId, body: &str) -> Self {
        let root = env::temp_dir().join(format!(
            "yoctui-compat-probe-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let tool = root.join(tool_id.executable_name());
        write_executable(&tool, body);
        Self { root, tool }
    }

    fn context(&self) -> CapabilityProbeContext {
        self.context_for(CapabilityToolId::Devtool)
    }

    fn context_for(&self, tool_id: CapabilityToolId) -> CapabilityProbeContext {
        let identity = YoctoEnvironmentIdentity {
            build_directory: AuthoritativeValue::detected(
                self.root.clone(),
                IdentityAuthority::InitializedEnvironment,
            ),
            available_tools: AuthoritativeValue::detected(
                vec![ToolIdentity {
                    id: tool_id.executable_name().into(),
                    executable: self.tool.clone(),
                    version: None,
                }],
                IdentityAuthority::ExecutableProbe,
            ),
            ..YoctoEnvironmentIdentity::default()
        };
        CapabilityProbeContext::new(
            identity,
            self.root.clone(),
            BTreeMap::from([(tool_id, self.tool.clone())]),
            BTreeMap::from([("PATH".into(), "/usr/bin:/bin".into())]),
            Some(BTreeSet::from(["create_spdx".into()])),
            Some(BTreeSet::from(["MACHINE".into()])),
            Some(BTreeSet::from(["getvar".into()])),
            Some(BTreeSet::from(["state_snapshots".into()])),
            Some(BTreeSet::from(["wic".into()])),
            Some(BTreeSet::from(["buildhistory".into()])),
        )
        .unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

mod compatibility_probe_uses_exact_shell_free_help_and_version_argv;

#[cfg(unix)]
mod compatibility_probe_background_priority_reaches_the_tool_process;

mod compatibility_probe_distinguishes_missing_tool_command_and_option;

mod compatibility_probe_bitbake_getvar_uses_exact_initialized_tool_and_options;

mod raw_capability_probe_is_shell_free_and_distinguishes_direct_evidence;

mod compatibility_probe_reports_timeout_oversize_and_stale_executable_as_inconclusive;

mod compatibility_probe_spawn_retry_classifies_only_text_file_busy_as_transient;

#[cfg(unix)]
mod compatibility_probe_accepts_safe_same_directory_utility_symlink_without_losing_argv0;

mod compatibility_probe_uncollected_inventory_is_inconclusive_not_negative;

mod compatibility_probe_maps_typed_non_process_observations;

mod compatibility_probe_context_rejects_environment_and_tool_mismatch;
