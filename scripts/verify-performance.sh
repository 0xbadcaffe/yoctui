#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

mode="${1:-all}"

source "$repo_root/scripts/verify-performance/contract_and_baseline.sh"
source "$repo_root/scripts/verify-performance/profiles_and_wakeups.sh"
source "$repo_root/scripts/verify-performance/event_render_animation.sh"
source "$repo_root/scripts/verify-performance/telemetry_logs_tasks.sh"
source "$repo_root/scripts/verify-performance/ipc_and_runtime.sh"
source "$repo_root/scripts/verify-performance/scheduling_and_affinity.sh"
source "$repo_root/scripts/verify-performance/coexistence_and_real_poky.sh"
source "$repo_root/scripts/verify-performance/regressions_ci_docs.sh"

case "$mode" in
  --contract)
    verify_contract
    ;;
  --baseline)
    verify_contract
    verify_baseline
    ;;
  --profiles)
    verify_contract
    verify_baseline
    verify_profiles
    ;;
  --wakeups)
    verify_contract
    verify_baseline
    verify_profiles
    verify_wakeups
    ;;
  --event-loops)
    verify_contract
    verify_baseline
    verify_profiles
    verify_wakeups
    verify_event_loops
    ;;
  --render)
    verify_contract
    verify_baseline
    verify_profiles
    verify_wakeups
    verify_event_loops
    verify_render
    ;;
  --animations)
    verify_contract
    verify_baseline
    verify_profiles
    verify_wakeups
    verify_event_loops
    verify_render
    verify_animations
    ;;
  --telemetry)
    verify_contract
    verify_baseline
    verify_profiles
    verify_wakeups
    verify_event_loops
    verify_render
    verify_animations
    verify_telemetry
    ;;
  --logs)
    verify_contract
    verify_baseline
    verify_profiles
    verify_wakeups
    verify_event_loops
    verify_render
    verify_animations
    verify_telemetry
    verify_logs
    ;;
  --tasks)
    verify_contract
    verify_baseline
    verify_profiles
    verify_wakeups
    verify_event_loops
    verify_render
    verify_animations
    verify_telemetry
    verify_logs
    verify_tasks
    ;;
  --ipc)
    verify_contract
    verify_baseline
    verify_profiles
    verify_wakeups
    verify_event_loops
    verify_render
    verify_animations
    verify_telemetry
    verify_logs
    verify_tasks
    verify_ipc
    ;;
  --tokio)
    verify_contract
    verify_baseline
    verify_profiles
    verify_wakeups
    verify_event_loops
    verify_render
    verify_animations
    verify_telemetry
    verify_logs
    verify_tasks
    verify_ipc
    verify_tokio
    ;;
  --scheduling)
    verify_contract
    verify_baseline
    verify_profiles
    verify_wakeups
    verify_event_loops
    verify_render
    verify_animations
    verify_telemetry
    verify_logs
    verify_tasks
    verify_ipc
    verify_tokio
    verify_scheduling
    ;;
  --affinity)
    verify_contract
    verify_baseline
    verify_profiles
    verify_wakeups
    verify_event_loops
    verify_render
    verify_animations
    verify_telemetry
    verify_logs
    verify_tasks
    verify_ipc
    verify_tokio
    verify_scheduling
    verify_affinity
    ;;
  --coexistence)
    verify_contract
    verify_baseline
    verify_profiles
    verify_wakeups
    verify_event_loops
    verify_render
    verify_animations
    verify_telemetry
    verify_logs
    verify_tasks
    verify_ipc
    verify_tokio
    verify_scheduling
    verify_affinity
    verify_coexistence
    ;;
  --real-poky-evidence)
    verify_contract
    verify_baseline
    verify_profiles
    verify_wakeups
    verify_event_loops
    verify_render
    verify_animations
    verify_telemetry
    verify_logs
    verify_tasks
    verify_ipc
    verify_tokio
    verify_scheduling
    verify_affinity
    verify_coexistence
    verify_real_poky
    ;;
  --regressions)
    verify_contract
    verify_baseline
    verify_profiles
    verify_wakeups
    verify_event_loops
    verify_render
    verify_animations
    verify_telemetry
    verify_logs
    verify_tasks
    verify_ipc
    verify_tokio
    verify_scheduling
    verify_affinity
    verify_coexistence
    verify_real_poky
    verify_regressions
    ;;
  --ci)
    "$0" --regressions
    verify_ci
    ;;
  --docs)
    verify_contract
    verify_docs
    ;;
  all)
    verify_contract
    verify_all_done
    verify_baseline
    verify_profiles
    verify_wakeups
    verify_event_loops
    verify_render
    verify_animations
    verify_telemetry
    verify_logs
    verify_tasks
    verify_ipc
    verify_tokio
    verify_scheduling
    verify_affinity
    verify_coexistence
    verify_real_poky
    verify_regressions
    verify_ci
    verify_docs
    for gate in \
      ./scripts/verify-low-overhead.sh \
      ./scripts/verify-saturation-responsiveness.sh \
      ./scripts/verify-ipc-continuity.sh \
      ./scripts/verify-bounded-memory.sh
    do
      if [[ ! -x "$gate" ]]; then
        printf 'missing executable performance gate: %s\n' "$gate" >&2
        exit 1
      fi
      "$gate"
    done
    ;;
  *)
    printf 'performance verification mode is not implemented yet: %s\n' "$mode" >&2
    exit 1
    ;;
esac
