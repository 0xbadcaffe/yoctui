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

In Images (F8), `i` opens the image picker. On the Artifacts tab (`1`), `b`
reviews the selected image build; Enter confirms it and Esc cancels the review.
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

Global `/` searches commands and source/config/log/pkgdata/deploy/rootfs content
using case-insensitive Rust regex. Enter opens a hit or action. At most 500 hits;
binary/oversized files, symlinks, .git, downloads, sstate, and caches are excluded.
Editors, terminals, and contextual searches keep their own `/` handling.
Ctrl+V pastes into fields/editors with wl-paste, xclip, or xsel available;
native terminals retain their own paste behavior.

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
