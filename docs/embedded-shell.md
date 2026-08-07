# Embedded native shell

Yoctui keeps the child shell inside its alternate-screen application. A shell
session has a stable ID, validated absolute cwd, environment identity, bounded
4,000-line scrollback, lifecycle status, and unread-activity marker. At most
four sessions are retained.

While the shell has foreground focus, ordinary keys are sent to its PTY.
`Ctrl+]` is reserved by Yoctui and always returns input to the application;
the child cannot consume it. `Tab`, `Shift+Tab`, dialogs, and confirmation
overlays retain Yoctui priority. Copy and search modes suspend child input.

The PTY propagates terminal resize and owns the child process group. Shutdown
attempts graceful HUP/TERM handling before forced cleanup. Output is bounded;
failure evidence contains redacted ANSI/text logs and terminal dimensions.
Interactive programs, Unicode, cursor movement, alternate-screen controls,
and bracketed-paste-safe terminal emulation are supported by the embedded
terminal state machine. Multiline paste requires explicit confirmation.

Shells may be opened at a build directory, source directory, selected
layer/file/recipe context, SDK environment, or another validated safe
directory. Yocto variables are inherited from the initialized process and are
not silently re-sourced into a running session. If cwd or environment becomes
stale, Yoctui marks the session and offers controlled restart/refresh.

The typed utility workbench remains the preferred route for common operations.
Expert utility forms produce indexed argv previews; commands typed manually in
the native shell remain the user's responsibility and are never parsed by
Yoctui.
