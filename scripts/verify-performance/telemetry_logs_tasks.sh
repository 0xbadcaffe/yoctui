verify_telemetry() {
  python3 - <<'PY'
from pathlib import Path

source = Path("crates/yoctui-cli/src/host_telemetry.rs").read_text(encoding="utf-8")
source += Path("crates/yoctui-cli/src/interactive_runtime.rs").read_text(encoding="utf-8")
source += "".join(
    path.read_text(encoding="utf-8")
    for path in sorted(Path("crates/yoctui-cli/src/interactive_runtime").rglob("*.rs"))
)
scheduler = Path("crates/yoctui-cli/src/telemetry_scheduler.rs").read_text(encoding="utf-8")
sampler = Path("crates/yoctui-cli/src/host_telemetry.rs").read_text(encoding="utf-8")
if "ProcessCommand" in sampler or "Command::new" in sampler:
    raise SystemExit("host telemetry must not spawn a process per sample")
for required in (
    "CLIENT_VISIBLE_INTERVAL", "CLIENT_BACKGROUND_INTERVAL",
    "DAEMON_ACTIVE_INTERVAL", "DAEMON_ATTACHED_IDLE_INTERVAL",
    "connected_clients == 0", "Screen::Dashboard | Screen::Tasks",
):
    if required not in scheduler:
        raise SystemExit(f"telemetry demand/cadence guard is missing: {required}")
for required in (
    "build_disk_device: Option<(u64, u64)>",
    "network_interface: Option<String>",
    "logical_cpu_count: Option<u16>",
    "telemetry_visible && telemetry_changed",
):
    if required not in source:
        raise SystemExit(f"telemetry cache/invalidation contract is missing: {required}")
print("demand-aware telemetry source contracts valid")
PY
  cargo test -q -p yoctui --bin yoctui telemetry_scheduler
  cargo test -q -p yoctui --bin yoctui telemetry_sampling
  cargo test -q -p yoctui-model bounded_telemetry_history_retains_only_the_latest_valid_samples
}

verify_logs() {
  python3 - <<'PY'
from pathlib import Path

model = "\n".join(path.read_text(encoding="utf-8") for path in sorted(Path("crates/yoctui-model/src").rglob("*.rs")) if "tests" not in path.parts and path.name != "tests.rs")
runtime_root = Path("crates/yoctui-cli/src/client_runtime")
runtime = Path("crates/yoctui-cli/src/client_runtime.rs").read_text(encoding="utf-8")
runtime += "".join(path.read_text(encoding="utf-8") for path in sorted(runtime_root.rglob("*.rs")))
app = "\n".join(path.read_text(encoding="utf-8") for path in sorted(Path("crates/yoctui-app/src").rglob("*.rs")) if "tests" not in path.parts and path.name != "tests.rs")
filtered = model.split("pub fn filtered(&self)", 1)[1].split("pub fn diagnostics", 1)[0]
if "e.message.to_lowercase()" in filtered:
    raise SystemExit("log filtering regressed to lowercasing every retained message")
for required in (
    "normalized_messages: VecDeque<String>", "pub fn insert_batch",
    "Action::Logs", "maximum_horizontal_offset", "for entry in self.filtered()",
):
    if required not in model:
        raise SystemExit(f"bounded log projection contract is missing: {required}")
for required in ("pending_logs", "flush_log_events", "MAX_EVENTS_PER_POLL"):
    if required not in runtime:
        raise SystemExit(f"client log batching contract is missing: {required}")
if "apply_log_events_to_app" not in app:
    raise SystemExit("daemon replica lacks ordered batch log reduction")
print("bounded batched log source contracts valid")
PY
  cargo test -q -p yoctui-model tests::coexistence_diagnostic_distinguishes_nominal_busy_and_unknown::log_batches_preserve_critical_order_counts_and_cached_search -- --exact
  cargo test -q -p yoctui-model tests::coexistence_diagnostic_distinguishes_nominal_busy_and_unknown::log_retention_prefers_important_diagnostics_and_reports_coalescing -- --exact
  cargo test -q -p yoctui-model tests::coexistence_diagnostic_distinguishes_nominal_busy_and_unknown::ux_logs_virtualized_window_and_source_time_filters_stay_bounded -- --exact
  cargo test -q -p yoctui-app tests::mouse_runtime_routes_dialog_and_terminal_session_clicks::daemon_client_batches_contiguous_logs_with_one_model_install -- --exact
  cargo test -q -p yoctui-ui tests::log_workspace_exposes_search_filters_pressure_and_narrow_wrap_safely::log_workspace_exposes_search_filters_pressure_and_narrow_wrap_safely -- --exact
}

verify_tasks() {
  python3 - <<'PY'
from pathlib import Path

model = "\n".join(path.read_text(encoding="utf-8") for path in sorted(Path("crates/yoctui-model/src").rglob("*.rs")) if "tests" not in path.parts and path.name != "tests.rs")
runtime_root = Path("crates/yoctui-cli/src/client_runtime")
runtime = Path("crates/yoctui-cli/src/client_runtime.rs").read_text(encoding="utf-8")
runtime += "".join(path.read_text(encoding="utf-8") for path in sorted(runtime_root.rglob("*.rs")))
app = "\n".join(path.read_text(encoding="utf-8") for path in sorted(Path("crates/yoctui-app/src").rglob("*.rs")) if "tests" not in path.parts and path.name != "tests.rs")
for required in (
    "pub enum TaskEvent", "Action::TaskEvents", "apply_task_batch",
    "task_progress_coalesced", "TaskProjectionCache", "MAX_ACTIVE_TASKS",
):
    if required not in model:
        raise SystemExit(f"bounded task update contract is missing: {required}")
for required in ("pending_tasks", "flush_task_events", "MAX_EVENTS_PER_POLL"):
    if required not in runtime:
        raise SystemExit(f"client task batching contract is missing: {required}")
if "apply_task_events_to_app" not in app:
    raise SystemExit("daemon replica lacks ordered batch task reduction")
print("bounded batched task source contracts valid")
PY
  cargo test -q -p yoctui-model tests::navigator_workbench_order_keeps_build_and_validation_groups_contiguous::task_batches_coalesce_progress_and_preserve_terminal_failures -- --exact
  cargo test -q -p yoctui-model tests::navigator_workbench_order_keeps_build_and_validation_groups_contiguous::task_event_flood_bounds_active_and_completed_state_without_losing_terminal_failure -- --exact
  cargo test -q -p yoctui-model tests::navigator_workbench_order_keeps_build_and_validation_groups_contiguous::unchanged_task_projection_reuses_sorted_identity_cache -- --exact
  cargo test -q -p yoctui-app tests::mouse_runtime_routes_dialog_and_terminal_session_clicks::daemon_client_batches_task_progress_without_losing_failure -- --exact
  cargo test -q -p yoctui --bin yoctui client_runtime
}
