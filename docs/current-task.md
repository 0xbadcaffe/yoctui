# Current task

## Active task

**ID:** SIG-MODEL-001
**Title:** Add typed signature dump and comparison state

## Objective

Define pure typed recipe/task signature identities, explicit dump/comparison
states, bounded deterministic differences, and reducer lifecycle independent of
raw `bitbake-dumpsig` or `bitbake-diffsigs` output.

## Required work

1. Inventory the existing recipe `diffsigs` task shortcut, build/background-job
   state, available recipe/task metadata, screens/dialogs, and any signature
   paths or parsing already present.
2. Define stable typed signature identity using exact recipe, task, signature
   hash, and authoritative absolute signature path where reported. Distinguish
   unavailable fields instead of inventing them.
3. Represent not-loaded, loading, available-empty, available, partial, and
   failed dump state, plus explicit comparison selection and result states.
4. Define typed bounded signature entries and deterministic difference
   categories for changed values, dependencies, base hashes, and unavailable
   fields. The model must not parse raw tool text.
5. Preserve selected entry/signature identities across refresh and clamp
   safely when data disappears.
6. Add reducer actions for dump request/success/partial/failure, comparison
   selection/request/success/failure, and clearing stale comparison results
   when either input changes.
7. Add model and app tests named `signature_model` for validation, bounds,
   duplicate normalization, every explicit state, stable selection, stale
   correlation, and typed event/effect mapping.
8. Update `docs/architecture.md` for signature state ownership and the future
   adapter boundary.

## Definition of done

- Pure model state owns typed identities, bounded normalized dump entries,
  comparison inputs/results, selection, and lifecycle.
- Missing data and partial/failure outcomes remain explicit.
- Reducers consume only typed data and effects; no raw tool output is parsed.
- Focused and baseline verification pass.

## Verification

```bash
cargo test -p yoctui-model signature_model
cargo test -p yoctui-app signature_model
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`SIG-ADAPTER-001 — Acquire and compare authoritative BitBake signatures`
