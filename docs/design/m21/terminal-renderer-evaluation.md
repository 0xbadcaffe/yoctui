# M21 terminal renderer evaluation

## Decision

Admit `tui-term` 0.3.4 only as a stateless generic renderer, with default
features disabled and no enabled features. Yoctui does not use the crate's
optional `vt100` implementation or unstable PTY controller. The daemon remains
the sole PTY, parser, emulator, scrollback, resize, and writer authority.

The pinned source exposes public generic
[`Screen`](https://docs.rs/tui-term/0.3.4/tui_term/widget/trait.Screen.html)
and [`Cell`](https://docs.rs/tui-term/0.3.4/tui_term/widget/trait.Cell.html)
traits. This is the required boundary: `yoctui-ui` can project an already
emulated replica directly instead of giving the widget terminal bytes. The
crate's documented `vt100::Parser` example and optional controller are not
part of Yoctui's feature graph.

## Authority and data flow

```text
daemon PTY bytes
  -> one bounded yoctui-model vt100 emulator
  -> dense typed TerminalSnapshot
  -> validated sparse protocol cells + cursor/scrollback state
  -> one dense client replica
  -> transient tui-term Screen/Cell projection
  -> Ratatui Buffer
```

The wire representation sends only non-default cells with stable row-major
indices. Blank default cells are implicit, preventing large blank screens from
consuming the frame budget. Protocol validation rejects invalid dimensions,
more than 250,000 cells, cell contents above 1,024 bytes, control characters,
unsorted or duplicate indices, out-of-range cursor/cells, invalid scrollback
offsets, and frames above 4 MiB. The client expands a validated event once;
plain rows are derived from those same cells rather than transported as a
second potentially drifting representation.

This renderer wire change advanced the daemon protocol from 1.0 to 1.1. The
Terminal Sessions workbench subsequently advances it to 1.2 for bounded
viewport requests, dropped-history accounting, and explicit terminal removal.
Negotiation
therefore rejects a stale daemon or client instead of accepting a peer that can
only exchange the former lossy plain-row screen.

The UI adapter is rebuilt for a draw and retained nowhere. It maps typed cell
contents, RGB/indexed/default colors, bold, dim, italic, underline, inverse,
wide-cell continuation, cursor visibility and cursor position. Scrollback and
horizontal viewport offsets are coordinate projections only. No adapter method
accepts raw bytes, runs a parser, accesses a terminal, starts a process, or
mutates daemon/model state.

## Parity evidence

- Model tests establish bounded complete snapshots, Unicode wide/continuation
  cells, colors, modifiers, cursor visibility, and scrollback coordinates.
- CLI tests establish exact sparse ordered wire cells and absence of escape
  bytes after the daemon parser.
- App tests establish deterministic dense reconstruction and derive fallback
  text from the same cell authority.
- TestBackend tests compare symbols, RGB/indexed colors, modifiers, wide cells,
  visible cursor placement, and no-color behavior.
- The existing real-PTY acceptance fixture now traverses the renderer with
  RGB/bold/italic/underline output and still proves resize, split-pane
  placement, focus ownership, prefix behavior, and terminal restoration.
- All attached clients consume the same journaled `PtyScreenSnapshot`; the
  renderer has no per-client emulator or retained terminal state that could
  drift.

Malformed legacy/test-only client fixtures without a complete dense cell grid
retain the existing plain-row fallback. Normal daemon snapshots always use the
validated typed path.

## Dependency and size evidence

| Item | Result |
|---|---|
| Source/checksum | crates.io 0.3.4, `a338ded85dbe7f9ea2298321d126244f54e531e2b2006b97abdab8e47d6f3c88` |
| License | MIT |
| Declared MSRV | Rust 1.86.0 |
| Ratatui API | `ratatui-core ^0.1.0`, `ratatui-widgets ^0.3.0` |
| Features | defaults off; none enabled |
| Audited closure | 46 packages, all already present except `tui-term` itself |
| Optional parser/controller | `vt100` and `portable-pty` absent from the `tui-term` edge |
| Stripped size-optimized reference link | 288,280 bytes baseline; 288,440 bytes candidate; +160 bytes (+0.1%) |

The size comparison used the same two-cell Ratatui buffer workload in both
builds with `opt-level="z"`, fat LTO, one codegen unit, abort-on-panic, stripped
symbols, and an offline locked build. The candidate variant rendered through
the generic traits; the baseline wrote the same two symbols directly.

Generated notices and the shipped CycloneDX SBOM include the admitted crate.
The candidate graph records the narrower feature-free closure. `cargo deny`,
the locked offline workspace build, TestBackend tests, and the real PTY fixture
are mandatory completion gates.

## Deliberate limits

This decision admits rendering, not terminal-session UX. Session creation,
tabs/list, rename, writer takeover, copy/search, paste, dropped-history UI,
detach/reattach, and termination remain owned by `UX-TERMINAL-UX-001`. Future
use of a `tui-term` optional feature requires a new dependency and authority
review.
