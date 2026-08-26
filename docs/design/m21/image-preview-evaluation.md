# Terminal Image Preview Evaluation

`UX-IMAGE-PREVIEW-001` evaluated the audited `ratatui-image` 11.0.6 candidate
with default features disabled. The decision is **reject** for the current
Images workbench. Yoctui keeps exact artifact metadata, rootfs composition, and
existing open/QEMU/Wic actions as the deterministic fallback.

## Applicability

The authoritative deploy scanner classifies root filesystems, kernels,
bootloaders, Wic disk images, manifests, license manifests, SPDX documents,
checksums, and unknown records. None of those identities establishes raster
MIME authority. Decoding a filesystem or Wic image as a PNG would be false; an
`Other` suffix is not enough evidence to opt into image decoding.

## Boundary review

The pinned picker source performs terminal discovery by changing stdin termios,
writing escape-sequence queries, reading terminal responses, and allowing a
two-second response timeout. Its tmux detection may spawn `tmux set -p
allow-passthrough on`, which changes external pane state. Its threaded resize
helper sends owned image/protocol state through an unbounded channel, uses a
wrapping local ID, and supplies no input-byte, decoded-pixel, retained-memory,
elapsed-time, or cancellation bounds. SSH is not a distinct policy state.

Those behaviors conflict with Yoctui's existing input owner, typed effect
boundary, bounded background-work contract, and rule that environment changes
need explicit intent. Reimplementing probing, bounded decode, cancellation,
generation correlation, resize coalescing, and transport policy around the
widget would be substantial code for an inventory with no raster use case.

## Measured cost

The audited candidate closure contains 118 packages and would add 71 packages
not present in the shipped graph. A disposable offline release-link benchmark
used Rust 1.97.0, Ratatui 0.30.2, `opt-level = "s"`, fat LTO, one codegen unit,
abort-on-panic, and stripped symbols. The baseline referenced Ratatui `Size`;
the candidate selected all four protocol branches and encoded a bounded
640×480 image into a 40×20-cell target.

| Binary | Bytes |
|---|---:|
| Ratatui baseline | 286,472 |
| `ratatui-image` candidate | 605,520 |
| Increase | 319,048 (111.4%) |

The temporary benchmark directory was removed after measurement. The candidate
also retains the decoded `DynamicImage` while resize/encoding can create another
image and protocol payload, so memory cannot be capped by the widget alone.

## Accepted behavior

- No terminal graphics capability query, escape sequence, termios mutation, or
  tmux command is issued by Yoctui for artifact previews.
- Direct terminals, SSH, tmux, SSH-through-tmux, and TestBackend all select the
  same deterministic typed fallback.
- The Inspector states that probing was skipped, native graphics are not
  offered, names the exact fallback, and explains why the selected artifact is
  not raster-authoritative.
- Root filesystem artifacts route users toward package/filesystem composition;
  other artifacts retain exact metadata and existing workflows.
- The candidate remains outside `Cargo.toml`, `Cargo.lock`, shipped notices, and
  the shipped SBOM. Its audited rejected record remains in the candidate SBOM.

Re-evaluation requires a new typed raster-artifact authority plus bounded,
cancellable probing/decode evidence that does not mutate terminal-multiplexer
state, and must repeat dependency, binary-size, memory, SSH/tmux, accessibility,
license, notice, SBOM, deny, and locked-offline checks.
