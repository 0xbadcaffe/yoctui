# Current task

## Active task

**ID:** PKG-ADAPTER-001
**Title:** Acquire authoritative package data

## Objective

Acquire package inventories and exact package details through a bounded,
shell-free `oe-pkgdata-util` adapter, returning only typed model data and honest
limitations or correlated failures.

## Required work

1. Inspect the installed `oe-pkgdata-util` interface, current process-adapter
   conventions, build-directory validation, cancellation helpers, and package
   model before adding behavior.
2. Discover the authoritative tool and pkgdata root from the configured Yocto
   workspace without relying on the caller's current directory or an
   unvalidated `PATH` result.
3. Construct every process invocation as an executable plus exact arguments;
   do not use a shell, interpolate package names, or parse output outside
   `yoctui-bitbake`.
4. Validate package identities and canonical build/tool/pkgdata paths before
   execution. Reject symlinks or paths escaping their configured roots.
5. Parse bounded inventory and detail output into typed package summaries,
   files, runtime dependencies, reverse dependencies, installed size, license,
   provider, version, and image membership where the authoritative tool or
   pkgdata records expose them. Mark unsupported or absent fields unavailable
   rather than guessing.
6. Enforce hard limits on stdout, stderr, lines, records, nested collections,
   and execution duration. Support explicit process-group cancellation and
   report truncation as typed limitations.
7. Distinguish a valid empty result, partial result, missing/unbuilt pkgdata,
   missing tool, malformed records, timeout, cancellation, nonzero exit, and
   request mismatch.
8. Add typed response conversions and app-boundary tests; reducer and UI code
   must never receive raw tool output.
9. Add fake executable/process coverage named `pkgdata_adapter` for exact
   arguments, discovery, parsing, bounds, failures, timeout, cancellation,
   invalid paths, and unavailable fields.
10. Add an opt-in live smoke path. Run it when authoritative pkgdata exists;
    otherwise record the exact missing external prerequisite without claiming
    live compatibility.
11. Update `docs/architecture.md` with the completed adapter boundary and
    `docs/compatibility.md` only for evidence produced by a real live run.

## Definition of done

- Package acquisition is authoritative, bounded, shell-free, cancellable, and
  typed.
- Empty, partial, missing, malformed, timed-out, cancelled, and failed outcomes
  remain honest and distinct.
- Fake-process coverage proves construction and parsing; any live claim has
  real generated pkgdata evidence.
- Focused and baseline checks pass.

## Verification

```bash
cargo test -p yoctui-bitbake pkgdata_adapter
cargo test -p yoctui-app pkgdata_adapter
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`PKG-UI-001 — Integrate the package data workspace`
