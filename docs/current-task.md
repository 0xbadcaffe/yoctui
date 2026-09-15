# Current Task

**ID:** DEVICE-TREE-EDITOR-001
**Title:** Add DTS-aware editing and configurable `dtc` compilation
**Status:** DONE

Recognize DTS and DTSI sources as a first-class editor language, highlight
device-tree directives, labels, property assignments, values and comments, and
retain structural validation in the shared in-app editor. Replace the fixed
compile shortcut with a typed options dialog for symbol generation, stable
sorting, output padding and reserve-map capacity. The dialog must derive its
source, output, working directory and executable from the selected authoritative
inventory, render the exact resulting arguments through the existing terminal
launch preview, refuse existing output files, and run in the daemon-owned PTY.
Kernel and U-Boot workbenches must share the same model and input path.

Verification includes focused model, app and UI tests, the complete workspace
baseline, strict Clippy, bridge tests, documentation, version/layout checks,
roadmap verification and deterministic screenshot checks.

Version 0.1.112 recognizes DTS/DTSI in the shared editor, renders device-tree
syntax roles, and provides one typed compile-options flow for Kernel and U-Boot.
Exact argument construction, collision refusal, terminal preview and daemon PTY
creation are covered. All 1,620 Rust tests pass with four existing live tests
ignored; strict workspace Clippy, 52 bridge tests, documentation, version,
library-layout, roadmap and deterministic image checks pass.
