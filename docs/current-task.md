# Current Task

## Task

**ID:** COMPAT-TEST-CMDS-001
**Title:** Test version-correlated command generation
**Status:** IN_PROGRESS

## Objective

Use the shared generation fixtures to prove every version-varying command
emits only compatible argv and is rejected before spawn when unavailable.

## Dependencies

- `COMPAT-TEST-FIXTURES-001` — DONE
- `COMPAT-BITBAKE-CMD-001` — DONE
- `COMPAT-DEVTOOL-001` — DONE
- `COMPAT-RECIPETOOL-001` — DONE
- `COMPAT-LAYERS-001` — DONE
- `COMPAT-PKGDATA-001` — DONE

## Relevant files

- `crates/yoctui-bitbake/src/compatibility_fixtures.rs`
- `crates/yoctui-bitbake/src/compatibility_command.rs`
- `crates/yoctui-bitbake/src/compatibility_devtool.rs`
- `crates/yoctui-bitbake/src/compatibility_recipetool.rs`
- `crates/yoctui-bitbake/src/compatibility_layers.rs`
- `crates/yoctui-bitbake/src/package.rs`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- Old fixtures generate only old-compatible argv and new fixtures generate
  only positively evidenced modern argv.
- No planner emits an option or subcommand absent from its fixture authority.
- Maintained fallback implementation IDs select exact alternate argv forms;
  unknown/unavailable capabilities are rejected before process construction.
- BitBake, Devtool, Recipetool, bitbake-layers, and pkgdata version-varying
  command families are covered from the same shared snapshots.
- Command tests inspect typed argv directly and do not spawn external tools.

## Verification

```bash
cargo test --workspace --all-features compatibility_command
./scripts/verify-roadmap.sh
```
