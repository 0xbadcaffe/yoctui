# Terminal sessions

## Inherited Yocto shell

`!` suspends Yoctui and opens its inherited shell; `exit` or Ctrl+] returns.
It follows the client lifecycle. After a damaged terminal, use `reset`/`stty sane`.

## Daemon-owned Terminal Sessions

Open Terminal Sessions from Navigator/menu/palette. Create a shell in the build
or selected safe context, then take writer control (Ctrl+B o). Only the writer
can type/resize; clicking focus alone does not grant ownership.

The daemon owns PTYs, emulation, and bounded screen/scrollback; clients receive
styled cells. Navigation, client reconnect, pane close, and detach keep processes.
Ctrl+B Alt+k separately confirms termination. Daemon restart does not restore
process ownership; lost/exited sessions and dropped history are explicit.

[Prefix shortcuts](keyboard-shortcuts.md#terminal-prefix) cover splits, zoom, copy/search,
rename, detach, and writer release. Ctrl+B Ctrl+B sends a literal prefix;
multiline paste requires review. Initialized environment/cwd identity is retained;
stale context offers controlled restart/refresh rather than silent re-sourcing.

## Image consoles

Select a deployed artifact in Images and press `T`:

| Mode | Requirements and behavior |
| --- | --- |
| Boot with QEMU | Inspected runqemu, exact artifact, networking/memory review; nographic/serialstdio. |
| Connect over SSH | Explicit host/user/port and optional absolute identity file; normal host-key policy, no stored password. |

SSH connects to an existing target; it does not boot the image. Images `Q` offers
advanced QEMU arguments/display review. Only final confirmation launches a PTY;
missing tools/stale artifacts keep the form open. Both modes reuse writer leases,
scrollback, split, detach/reconnect, and confirmed kill. Fixture PTYs do not prove
real boot or SSH login. See [Compatibility](compatibility.md).

## Daemon and remote use

```bash
yoctui daemon status
yoctui attach
yoctui sessions
# Submission confirmation is not build completion.
yoctui daemon build core-image-minimal
```

SSH to the build host, then attach there; the daemon uses a per-user local Unix
socket. Restart/stop only after jobs and sessions finish. Reboot cannot resume child
processes; retained interrupted work becomes Lost. After an update, reopen the
client; restart the idle daemon to load the new executable.

Optional: `yoctui daemon service install` and `yoctui daemon service status`.
Arrange its initialized build environment yourself; unit installation alone does
not establish reboot readiness. Diagnose with status and
$XDG_STATE_HOME/yoctui/daemon.log (default ~/.local/state/yoctui/daemon.log).

## Team profiles

Local preferences remain local. Optional .yoctui/project.toml shares favorites,
build presets, and workflows, excluding credentials, host paths, and shell hooks.
`yoctui --build-dir "$BUILDDIR" profile` inspects without execution;
select and review a preset before running it.
