# Embedded Shells and Terminal Sessions

Yoctui provides two deliberately separate shell routes. The inherited shell is
a compatibility escape from the current client. Terminal Sessions are
daemon-owned persistent PTYs integrated into the workbench.

## Inherited Yocto shell

Press `!` to suspend the TUI and open the configured inherited shell in its
Yocto environment. Ordinary keys belong to that shell. `Ctrl+]` is reserved by
Yoctui as the emergency return route and cannot be consumed by the child; a
normal `exit` also restores Yoctui. This shell follows the client lifecycle and
does not provide reconnectable sessions or splits.

The terminal is restored around the child process. If an abnormal child exit
leaves it damaged, run `reset` and `stty sane`, then use
`./scripts/test-terminal.sh` when diagnosing restoration behavior.

## Daemon-owned Terminal Sessions

Open Terminal Sessions from the Navigator, Dashboard, application menu, or
command palette. Each shell has a stable identity, validated absolute working
directory and environment identity, bounded screen/scrollback, lifecycle
state, unread activity, attachments, and explicit writer ownership. The daemon
owns PTY processes and terminal emulation; clients receive typed bounded cells
and never reparse ANSI.

While a terminal pane has focus, ordinary keys go to the writer-owned PTY.
`Ctrl+B` begins the fixed one-second command prefix. `Ctrl+B ?` shows the full
map; create/navigation, splits, zoom, copy/search, rename, detach, and writer
control are described in the [keymap reference](keymap.md). A literal prefix is
`Ctrl+B Ctrl+B`. Multiline paste requires confirmation.

Closing a pane or detaching a client keeps the process. Process-group
termination is a separate `Ctrl+B K` route with confirmation. Disconnect,
daemon restart, normal terminal exit, and process loss remain distinct
outcomes. A replica reports dropped-history limits rather than implying
complete scrollback.

Shells may be created at a build directory, source directory, selected
layer/file/recipe context, SDK environment, or another validated safe
directory. Yocto variables come from the initialized environment and are not
silently re-sourced into a running session. If cwd or environment becomes
stale, Yoctui marks the session and offers controlled restart or refresh.

The typed utility workbench remains the preferred route for routine operations.
Expert utility forms produce indexed argv previews; commands typed manually in
either shell remain the user's responsibility and are never parsed as Yoctui
actions.

## Image consoles

Select an exact deployed artifact in Images and press `T`. The Image Console
form offers two bounded launch modes that both become daemon-owned Terminal
Sessions rendered through the existing `tui-term` replica.

- **Boot with QEMU** uses the current inspected `runqemu`, exact selected
  rootfs/Wic artifact, explicit networking and memory, and enforced
  `nographic`/`serialstdio` console options.
- **Connect over SSH** uses the initialized host's resolved OpenSSH executable
  and an explicit `user@host`, port, and optional normalized absolute identity
  file. It connects to an already-running target; SSH is not an image boot
  mechanism. Normal host-key verification remains enabled, no password is
  retained, and Yoctui does not append a remote command.

Opening, editing, or cancelling the form starts no process. Missing tools,
invalid fields, and stale artifact authority keep the form open with an exact
reason. After confirmation, Terminal Sessions exposes the normal writer lease,
scrollback, search/copy, split, detach, reconnect, and explicit kill controls.
