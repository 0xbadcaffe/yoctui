# Current Task

**ID:** README-MENUCONFIG-002
**Title:** Document native kernel and U-Boot menuconfig rendering
**Status:** DONE

Update the README gallery and kernel/firmware workflow to state that kernel and
U-Boot menuconfig preserve the original ncurses interface inside Yoctui. Record
the full workspace layout, omitted passive Inspector and exact PTY resizing.
Bump the workspace version, refresh deterministic version-bearing artifacts,
run the documentation, roadmap, release and raster checks, then publish the
reviewed branch to master.

Version 0.1.110 documents the native kernel and U-Boot menuconfig interface in
the README gallery and operator workflow. All workspace tests, strict Clippy,
52 bridge tests, documentation, version/layout policy, roadmap, and
deterministic raster checks pass.
