# Keyboard shortcuts

Build Environment: `1` environment profile, `2` setup catalogue, `w` new BitBake
setup wizard. Catalogue: `o` choose top directory, `r` refresh, arrows/Page/Home/End
select, `Enter` import for review. Wizard: `Tab` field, `Enter` edit/accept,
`Ctrl+U` clear field, `s` review, `Enter` confirm, `Esc` back/cancel.

Selected setup: `f`/`3` fragments, `s` status, `u` update (review required).
Fragments: `e` enable, `d` disable, `n` custom ID, `Enter` show, `r` refresh,
arrows/Page/Home/End scroll. Operation review: `e` edit fragment ID; update
`c` configuration policy, `b` conflict policy, `n` network; Enter confirms,
Escape cancels. Details use arrows/Page/Home/End and Escape returns to the list.

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
Tools → GitUI in the navigator opens the same launcher as Alt+g or F12 → Tools → Open GitUI.
Choose Embedded to run the full GitUI interface inside Yoctui.

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
case-insensitive Rust regex. Tab (or Shift+Tab) switches between File contents and File names;
Alt+n also works when the terminal sends Alt as an escape-prefixed key. If Alt+n
inserts a character because of your keyboard layout/terminal, use Tab instead.
Switching modes keeps the expression. For example, `\.dts$` finds DTS filenames; filename mode
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

## Development Workbench

Open **Build → Workbench** in the navigator or **F12 → Build → Open Workbench**.
`n` creates a named session; Enter reopens one, `r` renames, Delete removes its
saved session metadata only. Folders, source files and processes are not deleted.

In a workspace, `Alt+a` adds an absolute folder, `Alt+m` opens workspace/Devtool
actions, `Alt+f` focuses the explorer, `Alt+e` the editor, `Alt+t` the workspace
terminal, `Alt+z` expands/restores the bottom panel, and `Alt+o` returns to sessions.
Explorer arrows/Enter expand directories or open files; `[`/`]` (or Alt+Up/Down)
switch folders, `.` toggles hidden files, and `r` refreshes. Ctrl+S saves the current
file, Ctrl+W closes a saved tab, Alt+Left/Right switches tabs, Ctrl+Z/Y undo/redo,
and Ctrl+F finds text. Unsaved buffers survive tab/session changes and reopening.
Ctrl+A selects all, Shift+arrows extends selection, Ctrl+C copies and Ctrl+X cuts.
Esc returns editor → explorer → session list → navigator. Inside the terminal,
Esc, Tab, Ctrl+C and ordinary letters belong to the shell; Alt+e/f/o returns control.
Alt+/ searches terminal output; multiline terminal paste uses the existing review.

Bind a recipe in Alt+m before Alt+b reviews its build. Save edited tabs first.
Build confirmation/cancellation stays in Workbench. Modify, devshell, patch
creation, finish and reset use existing capability checks and review dialogs;
nothing publishes a patch automatically. See [Workbench storage and limits](tools-and-workflows.md#development-workbench).

## Diff tool

Open Tools → Diff tool (navigator or F12). Tab/Shift+Tab switches file trees;
Space marks/unmarks one file per side; F5 compares. Enter toggles a directory,
Right expands, Left collapses, Backspace goes to the parent root, `.` toggles
hidden files, and `r` refreshes. Comparison panes scroll together with arrows,
PgUp/PgDn and Home/End; Left/Right pans horizontally. Esc returns to the trees,
then Esc returns to the navigator. F5 still opens Logs outside this workspace.

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

In the Terminal Sessions tree, `/` opens global search without opening a console.
In an open terminal, Alt+/ also opens Yoctui search directly. A writer's plain
`/` is passed through to the shell or application; viewers can use `/` to search.
Search mode supports arrows, PgUp/PgDn and Home/End to inspect retained history;
Ctrl+U clears the query without leaving search. Matches are counted in the current viewport.

The inherited `!` shell uses Ctrl+] to return. See [Terminal sessions](terminal-sessions.md).

## Customize safely

Preferences → Keybindings: Enter/`c` captures up to three strokes; Ctrl+S saves;
`x` removes, `r` resets one, `R` resets all, `e` exports, `p` retries persistence.
Collisions, ambiguous/reserved prefixes, and loss of critical routes are rejected
without replacing the working map.
