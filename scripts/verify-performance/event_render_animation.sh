verify_event_loops() {
  python3 - <<'PY'
from pathlib import Path

daemon = Path("crates/yoctui-cli/src/daemon_server.rs").read_text(encoding="utf-8")
ipc = Path("crates/yoctui-protocol/src/daemon_ipc.rs").read_text(encoding="utf-8")
if "listener.accept(Duration::from_millis(1))" in daemon:
    raise SystemExit("daemon listener regressed to one-millisecond polling")
if "thread::sleep(CONNECT_RETRY_INTERVAL.min(timeout))" in ipc.split("impl DaemonListener", 1)[1].split("impl Drop", 1)[0]:
    raise SystemExit("daemon listener regressed to sleep-based readiness polling")
runtime_root = Path("crates/yoctui-cli/src/interactive_runtime")
client = Path("crates/yoctui-cli/src/interactive_runtime.rs").read_text(encoding="utf-8")
client += "".join(path.read_text(encoding="utf-8") for path in sorted(runtime_root.rglob("*.rs")))
if "runtime.terminal.draw(|f| render(f, &runtime.app))?;\n        if event::poll" in client:
    raise SystemExit("interactive client regressed to unconditional idle rendering")
if "if runtime.build_jobs.active_job_id().is_some()" not in client:
    raise SystemExit("idle client must not poll the inactive local BitBake backend")
print("event-loop source contracts valid")
PY
  cargo build -q -p yoctui --bin yoctui
  cargo test -q -p yoctui-protocol daemon_ipc
  cargo test -q -p yoctui --bin yoctui idle_daemon_waits_on_socket_readiness_without_delaying_active_work
  python3 scripts/test-idle-event-loops.py --binary target/debug/yoctui
}

verify_render() {
  python3 - <<'PY'
from pathlib import Path

runtime_root = Path("crates/yoctui-cli/src/interactive_runtime")
source = Path("crates/yoctui-cli/src/interactive_runtime.rs").read_text(encoding="utf-8")
source += "".join(path.read_text(encoding="utf-8") for path in sorted(runtime_root.rglob("*.rs")))
scheduler = Path("crates/yoctui-cli/src/render_scheduler.rs").read_text(encoding="utf-8")
draw = "runtime.terminal.draw(|f| render(f, &runtime.app))?;"
if source.count(draw) != 1:
    raise SystemExit("interactive runtime must have exactly one centralized render call")
guarded = (
    ".take_frame_with_interval(ordinary_frame_interval(&runtime.app))\n"
    "        {\n            " + draw
)
if guarded not in source:
    raise SystemExit("interactive render call is not guarded by coalesced invalidation")
for required in (
    "RenderCause::Input", "RenderCause::State", "RenderCause::Telemetry",
    "RenderCause::Presentation", "RenderCause::Resize",
    "interactive_frame_interval(refresh)",
):
    if required not in source:
        raise SystemExit(f"render invalidation source is missing: {required}")
for required in ("requests", "frames", "coalesced", "skipped_checks"):
    if required not in scheduler:
        raise SystemExit(f"render scheduler metric is missing: {required}")
print("dirty render source contracts valid")
PY
  cargo test -q -p yoctui --bin yoctui render_scheduler
  cargo test -q -p yoctui --bin yoctui normal_render_interval_is_capped_at_ten_hertz
}

verify_animations() {
  python3 - <<'PY'
from pathlib import Path

runtime_root = Path("crates/yoctui-cli/src/interactive_runtime")
source = Path("crates/yoctui-cli/src/interactive_runtime.rs").read_text(encoding="utf-8")
source += "".join(path.read_text(encoding="utf-8") for path in sorted(runtime_root.rglob("*.rs")))
scheduler = Path("crates/yoctui-cli/src/render_scheduler.rs").read_text(encoding="utf-8")
for required in (
    "has_visible_indeterminate_activity(&runtime.app)",
    "presentation_now + animation_interval(&runtime.app)",
    "presentation_now + ELAPSED_REFRESH_INTERVAL",
):
    if required not in source:
        raise SystemExit(f"animation scheduler contract is missing: {required}")
if "Action::Tick" not in source:
    raise SystemExit("visible animation does not advance the model phase")
for required in (
    "app.reduced_motion", "app.active_dialog().is_some()",
    "Screen::Dashboard | Screen::Tasks", "TaskState::Active",
    "task.progress.is_none()",
    "SATURATED_HOST_CPU_PERCENT", "SATURATED_FRAME_INTERVAL",
):
    if required not in scheduler:
        raise SystemExit(f"animation visibility guard is missing: {required}")
print("visible-only animation source contracts valid")
PY
  cargo test -q -p yoctui --bin yoctui render_scheduler::tests::animation_is_visible_only_indeterminate_and_nonterminal
  cargo test -q -p yoctui --bin yoctui render_scheduler::tests::overlays_and_reduced_motion_freeze_animation_but_not_elapsed_time
  cargo test -q -p yoctui --bin yoctui render_scheduler::tests::presentation_cadences_are_explicitly_bounded
  cargo test -q -p yoctui --bin yoctui render_scheduler::tests::saturated_live_builds_reduce_visual_freshness_without_affecting_input
  cargo test -q -p yoctui-ui active_task_indicator_uses_braille_motion_and_accessible_fallbacks
}
