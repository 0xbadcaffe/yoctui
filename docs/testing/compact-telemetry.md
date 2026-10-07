# Compact telemetry regression contract

Dashboard and Tasks retain CPU, RAM and build-filesystem readings at compact
heights. The fallback reserves four rows for three typed percentages and bars.
Unknown readings remain unavailable; zero is a valid measurement. The fallback
must not add sampling, animation, focus targets or backend operations.

Rendering and mouse selection share panel heights. Dashboard hit testing
accounts for its four summary rows, so logs, history, borders and resources
cannot select hidden task rows.

## Coverage

`compact_resource_meters_remain_visible_across_workspace_sizes` exercises
181x43, 160x48, 130x40, 100x30 and 80x24 layouts. Preserve selection/focus,
visible task and Log Viewer, full-size telemetry, missing/invalid readings,
zero, ASCII, no-color, reduced motion, tiny bounds and mouse-row coverage.

## Delivery boundary

Fixture tests establish presentation behavior only. Source-bound release
performance and genuine live-workspace validation require their separate gates;
a fixture or prior-version measurement does not certify a new release.
