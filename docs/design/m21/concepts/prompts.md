# M21 workbench concept prompt set

These are the normalized production prompts used with Codex built-in image
generation. They are retained for design provenance, not deterministic
reproduction: generative rendering can vary between runs.

Every scene used the `ui-mockup` taxonomy, requested a shippable terminal UI
screenshot rather than concept art, and prohibited rounded cards, gradients,
shadows, watermarks, OS chrome, neon HUD styling, and multiple focus owners.

## Shared visual system

- Real raster PNG concept for a Rust Ratatui terminal application.
- One panoramic terminal window at a logical `160x50` cell size.
- Two-row header, `26 / 89 / 45` three-pane body, and two-row footer.
- Compact professional terminal IDE with thin borders and one focus owner.
- `dark-pro`: near-black surfaces, graphite borders, cyan focused border,
  saturated-blue full-row selection, lime progress/success, amber navigation
  and warnings, cyan information, red failures, off-white primary text, and
  muted-gray secondary text.
- Textual status markers and labels accompany every color-coded state.

## 01 — Idle Dashboard

Create the Yoctui idle Dashboard with project `core-image-minimal`, machine
`qemux86-64`, distro `poky`, a connected daemon, idle BitBake, the complete
Navigator grouping, Build Overview, Recent Builds, Project Inspector, and
Quick Actions. Show exact meters for CPU `18%`, RAM `42%`, Build FS `63%`, and
sstate reuse `87%`. Include `F1 Help`, `F4 Dashboard`, `F9 Commands`,
`F10 Menu`, `q Quit`, and clock `19:28:27`.

## 02 — Active build Tasks

Using the Dashboard image only as a visual-system reference, create Tasks while
BitBake is running. Show build progress `7,240 / 10,056 tasks`, `72%`, elapsed
`00:18:42`, eight workers, and ETA `00:07:16`. Select `bash:do_compile` at
`72%` in Active Tasks and agree with the Task Inspector. Include following
bounded logs, Job History, CPU `86%`, RAM `58%`, Build FS `63%`, velocity
`43/min`, contextual actions, and the fixed clock.

## 03 — Failed build Errors

Using the active-build image as visual-system reference, create the failed
`bash:do_compile` outcome after `7,268 / 10,056` tasks. Show a structured
Errors/Warnings table, selected `oe_runmake failed` diagnostic, paused
correlated log at match `3/7`, retained/dropped accounting, non-bottom
scrollbar, explicit filter checkbox states, Error Inspector, recovery actions,
and `Clean & rebuild…` labeled as requiring confirmation.

## 04 — Rootfs composition

Create Images / Rootfs composition while BitBake is idle. Separate
`Installed packages` from `Filesystem tree`. Pair a segmented composition pie
with an exact table totaling `126.4 MiB` and `412` packages. Categories are
base system `46.8 MiB 37%`, libraries `34.1 MiB 27%`, kernel and modules
`18.9 MiB 15%`, locales `11.4 MiB 9%`, utilities `8.8 MiB 7%`, and Other
`6.4 MiB 5%`. Include a package drill-down tree with checked, unchecked,
indeterminate, and disabled text markers plus a scrollbar and Image Inspector.
The accepted output received one targeted edit changing only the secondary
header to `Build: (none)`, `Task: (none)`, `Elapsed: 00:00:00`,
`ETA: --:--:--`, and `Workers: 0`.

## 05 — Editor and F10 application menu

Create Recipes with a two-pane `bash_5.2.bb` editor behind a focus-trapped F10
menu. Show stable menu groups Workspace, Build, Navigate, View, Tools, Help;
open Build with `Build saved recipe` selected and `Cancel active build`
disabled because no build is running. Keep editor line numbers, syntax color,
warning range, Validation / Diff Preview, `INSERT · modified`, cursor position,
UTF-8, Save, Build saved, and external-editor routes visible. The menu border
alone owns focus.

## 06 — Terminal Sessions

Create Terminal Sessions with tabs for shell, devshell, and menuconfig. Show
two split daemon-owned PTYs: focused `shell · WRITER · client local-1` above
and `devshell:busybox · READ-ONLY · writer client ssh-2` below. Include real
looking bounded shell output, lower-pane scrollback search at match `2/5`,
`4,096 retained`, `312 dropped`, Session Inspector, disabled Take control
reason, confirmed close/kill affordances, and prefix help:
`Ctrl+B then ? Help · % Split · z Zoom · [ Copy/Search · d Detach · x Close · Ctrl+B Literal prefix`.
