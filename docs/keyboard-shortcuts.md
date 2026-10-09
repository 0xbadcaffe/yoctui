# Keyboard shortcuts

The focused dialog/editor/search/terminal owns input before global commands.
F1, contextual Help, and the footer show the current view's exact bindings.

## Global destinations

| Key | Action |
| --- | --- |
| F1 / F2 / F3 / F4 | Help / Tasks / History / Dashboard |
| F5 / F6 / F7 / F8 | Logs / Layers / Recipes / Images |
| F9 or Ctrl+P / F12 | Command palette / application menu |
| Alt+i / Alt+b / Alt+g | Inspector toggle / image build options / GitUI |
| `a` or right-click / `?` | Context actions / contextual Help |
| `!` / `q` or Ctrl+C | Inherited shell / request quit or contextual cancellation |

F5 opens logs, not a build. Recipe `b` builds the selected recipe after review.
Uppercase action aliases generally have Alt+lowercase equivalents.
Tasks lives under OVERVIEW with Dashboard and Insights; F2 is unchanged.

In Images (F8), `i` opens the image picker. On the Artifacts tab (`1`), `b`
reviews the selected image build; Enter confirms it and Esc cancels the review.
Outside that review, Enter (or `o`/`e`) views the selected artifact; DTB opens a
decompilation review. `p` inspects rootfs packages and `3` opens Files.
Use Tab to focus the workspace. These keys do not override editors or terminals.

## Focus and collection movement

| Intent | Keys |
| --- | --- |
| Pane focus / activate / close | Tab, Shift+Tab / Enter / Esc |
| Row / page / first-last | arrows or j/k / PgUp-PgDn / Home-End or gg/G |
| Tree collapse-expand | Left-Right or h/l |
| Search / clear / next-previous match | `/` / Ctrl+U / n or Alt+n |
| Checkbox / editor save | Space / Ctrl+S |
| Editor file/workspace search | Ctrl+F / Alt+f |

Navigator Enter/Right opens and focuses a destination. Zoom preserves selection
and scroll. Dialogs trap focus; writers retain child keys.

Global `/` searches build/config/log/pkgdata/deploy/rootfs file contents using
case-insensitive Rust regex. Alt+n switches between File contents and File names,
keeping the expression. For example, `\.dts$` finds DTS filenames; filename mode
also finds empty, binary and oversized files without reading them. Matches appear
while scanning and append below existing results without changing selection.
Enter opens a hit. At most 500 hits; content mode excludes binary/oversized files.
Both modes exclude symlinks, .git, downloads, sstate and caches. Esc cancels the
scan and retains partial results; typing another query replaces them.
Editors, terminals, and contextual searches keep their own `/` handling.
In Devtool, `/` filters the recipe list; Enter/Esc finishes typing and Ctrl+U clears it.
In its source editor, Ctrl+S saves files and Ctrl+B reviews a recipe build.
Confirming or cancelling the build retains your editor, file and cursor. Building
does not publish patches or run devtool finish/reset. Save and explicitly close
the editor when stopping for now; the Devtool source workspace stays on disk.
Ctrl+V pastes into fields/editors with wl-paste, xclip, or xsel available;
native terminals retain their own paste behavior.

Decompiled DTS opens at the first line with document focus. PgUp/PgDn and the
mouse wheel scroll; Home/End reach the first/last line. Alt+f searches that DTS.
In Errors, Esc closes retained diagnostics back to the selected error list;
a second Esc returns to the navigator.

## Terminal prefix

Press Ctrl+B, then a command within one second:

| Command | Result |
| --- | --- |
| c / n / p | Create build shell / next / previous session |
| % / " / z | Horizontal split / vertical split / zoom |
| x / d | Close pane / detach; keep process |
| : / ? / t | Palette / prefix Help / Terminal Sessions |
| o / Alt+o | Take / release writer |
| e / r | Return to retained editor / rename session |
| `[` / `/` | Copy / search |
| Alt+k | Confirm process-group termination |
| Ctrl+B | Send literal Ctrl+B |

The inherited `!` shell uses Ctrl+] to return. See [Terminal sessions](terminal-sessions.md).

## Customize safely

Preferences → Keybindings: Enter/`c` captures up to three strokes; Ctrl+S saves;
`x` removes, `r` resets one, `R` resets all, `e` exports, `p` retries persistence.
Collisions, ambiguous/reserved prefixes, and loss of critical routes are rejected
without replacing the working map.
