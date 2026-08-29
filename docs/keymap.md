# Yoctui Keymap Reference

Yoctui derives menus, Help, the command palette, contextual footers, mouse
routes, and configurable bindings from one typed action catalog. The focused
dialog, editor, search field, or terminal owns input before global routes are
considered. Disabled actions stay visible with their exact prerequisite.

## Global destinations

| Key | Destination or action |
|---|---|
| `F1` | Help |
| `F2` | Tasks |
| `F3` | History |
| `F4` | Dashboard |
| `F5` | Logs |
| `F6` | Layers |
| `F7` | Recipes |
| `F8` | Images |
| `F9` or `Ctrl+P` | Command palette |
| `F10` | Workspace/Build/Navigate/View/Tools/Help application menu |
| `B` | Image build options |
| `a` or right-click | Context actions for the current selection |
| `?` | Contextual Help |
| `!` | Suspend Yoctui and open the inherited Yocto shell |
| `q` or `Ctrl+C` | Request quit or contextually cancel; confirmations remain distinct |

`F5` never starts a build. A lower-case workspace binding such as recipe `b`
may build the selected recipe through its typed confirmation, while global `B`
opens image build options.

## Focus and collection movement

`Tab` and `Shift+Tab` move among visible focus targets. `Esc` closes the
innermost transient owner or moves outward. Focus and zoom commands are
discoverable through `F10` → View and the command palette; zoom preserves the
exact selection, scroll, follow, and subfocus state.

| Intent | Keys |
|---|---|
| Move one row | arrows or `j`/`k` |
| Move one page | `PageUp`/`PageDown` |
| First/last row | `Home`/`End` or `gg`/`G` |
| Tree collapse/expand | `h`/`l` or `Left`/`Right` |
| Global regex search | `/` (case-insensitive Rust regex) |
| Edit/clear active search | type or `Backspace` / `Ctrl+U` |
| Next/previous match | `n`/`N` |
| Primary action | `Enter` |
| Toggle checkbox | `Space` |

Mouse wheel movement uses the same bounded scroll route. Dialogs trap focus;
terminal and editor input is not interpreted as workspace navigation.

`/` opens a menuconfig-style unified search. It matches workbench destinations
and operator actions (labels, descriptions, action IDs, menu paths, aliases, and
keywords) together with content from:

- recipes (`.bb`, `.bbappend`, `.inc`), configuration (`.conf`), and classes
  (`.bbclass`);
- configured layer sources and scripts, plus Poky and BitBake sources;
- build configuration, task logs, pkgdata, and deployed generated metadata;
- every retained `tmp/work/.../rootfs/...` tree, including installed systemd
  units and their contents. These hits name the originating image recipe.

Results are case-insensitive Rust regex matches and show source kind,
`path:line`, a bounded preview, and image provenance where applicable. `Enter`
opens a content hit in `$EDITOR` or opens/runs an action. Invalid regex syntax is
shown inline. Searches return at most 500 hits, skip binary/oversized files,
never follow symlinks, and exclude `.git`, downloads, sstate, cache, and other
large generated source caches. Active editors, contextual searches, and
Terminal Sessions retain literal `/`; local workspace searches remain available
from the context action menu (`a`).

## Terminal prefix

Daemon-owned Terminal Sessions reserve `Ctrl+B` as a one-second prefix:

| Sequence | Result |
|---|---|
| `Ctrl+B c` | Create a build shell |
| `Ctrl+B n` / `Ctrl+B p` | Next / previous session |
| `Ctrl+B %` / `Ctrl+B "` | Horizontal / vertical split |
| `Ctrl+B z` | Zoom pane |
| `Ctrl+B x` | Close pane; keep the process |
| `Ctrl+B d` | Detach client; keep the process |
| `Ctrl+B :` | Command palette |
| `Ctrl+B ?` | Prefix help |
| `Ctrl+B o` / `Ctrl+B O` | Take / release writer control |
| `Ctrl+B t` | Terminal Sessions |
| `Ctrl+B [` / `Ctrl+B /` | Copy / search mode |
| `Ctrl+B r` | Rename session |
| `Ctrl+B K` | Confirmed process-group termination |
| `Ctrl+B Ctrl+B` | Send literal `Ctrl+B` |

The inherited shell opened by `!` instead reserves `Ctrl+]` as its emergency
return route. See [Embedded shells and terminal sessions](embedded-shell.md).

## Customize safely

Settings → Keybindings lists effective bindings with exact scope and
default/custom/disabled state. `Enter` or `c` captures up to three strokes;
`Backspace` edits, `Ctrl+S` validates and saves, and `Esc` cancels. Use `x` to
remove, `r` to reset the selected action, `R` to reset all, `e` to export, and
`p` to retry a failed persistence operation.

Yoctui rejects same-scope collisions, ambiguous prefixes, reserved `Ctrl+B`
routes, invalid sequences, and removal of the final Help or Dashboard route.
A rejected candidate never replaces the working keymap. The contextual footer
and Help are the runtime authority when a workspace has additional controls.
