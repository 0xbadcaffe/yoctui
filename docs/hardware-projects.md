# Hardware projects

Hardware → `p` opens Projects. `n` creates a project/subfolder; `a` imports without
moving or overwriting source files (256 MiB limit). Projects persist under
$XDG_DATA_HOME/yoctui/hardware-projects (default ~/.local/share/yoctui/hardware-projects).

Readable text of any extension uses the Vim-style editor; Ctrl+S saves with
conflict/permission checks (1 MiB limit). Unknown languages remain plain text.
PDF/schematic viewers need graphics; `e` explicitly edits readable source text.
KiCad needs kicad-cli; Altium/Xpedition need a same-stem PDF export. Stored files
are not executed. See [text and PDF controls](user-guide.md#view-and-edit-hardware-text-files).

`s` edits manual 0–100% Bootloader/Kernel/Device tree/Drivers/RootFS/Packages values.
Enter saves, Esc cancels. The overall bar averages these entries, not build progress.
