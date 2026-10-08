# Widgets and dependencies

[Architecture](architecture.md) owns state; [Interface behavior](interface-behavior.md) defines
interaction. This guide records widget choices.

## Product outcome

Keep authoritative facts/actions readable while preserving selected context and work.

## Non-negotiable constraints

Model owns focus/selection/scroll/edit/job state; widgets only project it. Unknown
is not zero; charts/colors need text equivalents. Narrow/accessibility modes keep
facts. Consequential operations retain review; PTY input requires writer ownership.

## Interaction architecture

One typed catalog drives all command/input surfaces. Stable identities survive
navigation/resize/zoom; inspector defaults off. See [shortcuts](keyboard-shortcuts.md).

## Built-in widget plan

| Primitive | Use |
| --- | --- |
| `Block`, `Clear` | Borders/overlays/focus. |
| `Paragraph` | Text/help/previews. |
| `List`, `Table`, `Tabs`, `Scrollbar` | Collections/subviews; model-owned selection/scroll. |
| `Gauge`, `LineGauge` | Explicit progress/capacity. |
| `Sparkline`, `Chart`, `BarChart`, `Canvas` | Measurements/composition with exact text. |
| `Calendar` | Authoritative date views. |

## Third-party dependency and license gate

| Crate | License | Decision |
| --- | --- | --- |
| [ratatui-image](https://crates.io/crates/ratatui-image) | MIT | Reject: no artifact raster authority/bounded decode owner. |
| [ratatui-textarea](https://crates.io/crates/ratatui-textarea) | MIT | Reject: retain model-owned editor. |
| [throbber-widgets-tui](https://crates.io/crates/throbber-widgets-tui) | Zlib | Adopt model-owned phase. |
| [tui-big-text](https://crates.io/crates/tui-big-text) | MIT OR Apache-2.0 | Defer pending value/density evidence. |
| [tui-checkbox](https://crates.io/crates/tui-checkbox) | MIT | Reject: native text sufficient. |
| [tui-logger](https://crates.io/crates/tui-logger) | MIT | Adopt bounded viewport only. |
| [tui-menu](https://crates.io/crates/tui-menu) | MIT OR Apache-2.0 | Reject: typed catalog projection. |
| [tui-nodes](https://crates.io/crates/tui-nodes) | MIT | Reject: model graph/tree authority. |
| [tui-piechart](https://crates.io/crates/tui-piechart) | MIT | Adopt wide rootfs with exact text. |
| [tui-scrollview](https://crates.io/crates/tui-scrollview) | MIT OR Apache-2.0 | Reject: model-owned scroll. |
| [tui-term](https://crates.io/crates/tui-term) | MIT | Adopt typed cells; parser/controller disabled. |
| [tui-tree-widget](https://crates.io/crates/tui-tree-widget) | MIT | Reject: stable-ID model trees. |
| [tui-widget-list](https://crates.io/crates/tui-widget-list) | MIT | Reject: bounded model projection. |

[Candidate audit](compliance/widget-candidates.toml) is not the shipped graph.
Dependency changes refresh pins/checksums/features/licenses/MSRV/Ratatui compatibility,
notices/SBOM, admission, cargo deny, and locked/offline checks. See [Compliance](compliance/README.md).

## Test strategy

Reducer/adapter/protocol/input tests cover state, authority, bounds, and lifecycle;
cell/raster fixtures cover layout/styles; PTYs cover native input/restoration.
Live release/boot evidence is separate. See [Testing](testing.md).

## Acceptance criteria

Actions remain discoverable/consistent; state survives resize; partial/missing data
is explicit. Required tests/dependency/performance/evidence checks pass for the scope.
