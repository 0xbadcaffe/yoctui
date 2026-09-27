# Current Task

**ID:** MENUCONFIG-UBOOT-LIVE-001
**Title:** Validate U-Boot interaction after provider ncurses build repair
**Status:** BLOCKED

M90 Hardware document rendering and v0.1.243 are complete. The reported PDF
passed live fit, zoom, and Navigator access checks. Full workspace tests remain
deferred at the user's request.

External dependency: `u-boot-aspeed-sdk` v2019.04+git fails its ncurses compile
check in `scripts/kconfig/dochecklxdialog`, before an interactive frontend is
available. Repair the provider's compiler/sysroot check, then verify arrows,
Enter, search, Ctrl+G out/back and client restart in Content > U-Boot.

```bash
cargo test -p yoctui-model platform_menuconfig
cargo test -p yoctui --bin yoctui menuconfig
cargo test -p yoctui-ui menuconfig
./scripts/verify-roadmap.sh
```

M67-LIVE-EVIDENCE-001 independently requires a new current-source real-Poky
performance capture. No unblocked implementation task is eligible.
