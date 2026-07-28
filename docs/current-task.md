# Current task

## Active task

**ID:** WIC-WRITE-ADAPTER-001
**Title:** Discover and revalidate safe Wic write devices

## Objective

Implement the adapter boundary for bounded removable-device discovery and
immediately-before-spawn safety revalidation for shell-free `wic write`,
without privilege escalation or live-device claims.

## Required work

1. Inspect the existing Wic device/output/write model, exact write preview,
   creation runner contracts, architecture safety rules, and adapter fake
   process seams before editing; do not weaken model validation.
2. Add a bounded typed whole-block-device discovery command using an explicit
   executable candidate or canonical `PATH` resolution and shell-free
   arguments. Parse only the documented machine-readable fields needed by the
   model.
3. Reject malformed, oversized, duplicate, partial, symlinked, partition,
   loop/device-mapper/optical, non-removable, read-only, undersized, mounted, or
   ambiguous records. Record safe non-fatal exclusions as explicit limitations.
4. Determine and exclude the current system/root backing whole device without
   guessing from display names. If that identity cannot be established, fail
   closed rather than exposing uncertain candidates.
5. Build an exact `wic write <image> <device>` command only from the model's
   confirmed preview and independently revalidate the canonical regular image
   plus the exact major/minor, capacity, model/serial/transport, removable,
   writable, whole-device, descendant-mount, and system-device invariants
   immediately before construction/spawn.
6. Never invoke `sudo`, a shell, a partition path, or a stale device identity.
   Surface missing tools, permissions, malformed data, timeouts, and safety
   rejections as typed errors.
7. Add `wic_device_write` adapter and app-boundary tests with fake discovery
   records/processes for safe inventory, every rejection class, stale identity,
   changed mounts/capacity/serial/major-minor, undersized image/device, exact
   argv, nonzero failure, cancellation, output bounds, and loss. Tests must use
   fake device paths and must not claim live hardware safety.
8. Run focused and baseline checks, then mark the child done and hand off to
   `WIC-WRITE-UI-CLI-001`.

## Definition of done

- Discovery returns only bounded typed candidates that satisfy every current
  safety invariant.
- Write construction rechecks image and device identity immediately before use
  and emits only exact shell-free arguments.
- Unsafe, ambiguous, stale, privileged, and unsupported paths fail closed with
  typed reasons.
- Fake-device coverage is comprehensive and no live removable-media claim is
  made.
- Focused and baseline checks pass.

## Verification

```bash
cargo test -p yoctui-bitbake wic_device_write
cargo test -p yoctui-app wic_device_write
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
