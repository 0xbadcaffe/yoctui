# Current Task

**ID:** MENUCONFIG-UBOOT-LIVE-001
**Title:** Validate U-Boot interaction after provider ncurses build repair
**Status:** BLOCKED

M89 recovery and v0.1.242 release are complete. Live recovery of the retained
unknown API snapshot passed. A fresh initialized daemon published recipe
metadata/inventory/getvar authority and completed initial inventory. A fresh
client loaded Kernel configuration and opened the menuconfig launch dialog.
Full workspace tests remain deferred at the user's request.

External dependency: `u-boot-aspeed-sdk` v2019.04+git fails its ncurses compile
check in `scripts/kconfig/dochecklxdialog`, before an interactive frontend is
available. Reproduced in daemon session 20 using:

```bash
cd /home/bspguy-dev/src/openbmc
source oe-init-build-env build/romulus
bitbake u-boot-aspeed-sdk -c menuconfig
```

The generated upstream wrapper returns 0 after acknowledging failed make;
that is not successful ncurses validation. Repair the provider's compiler/sysroot
check, then verify arrows, Enter, search, Ctrl+G out/back and client restart in
Content > U-Boot. Shared typed key/resume coverage already passes.

```bash
cargo test -p yoctui-model platform_menuconfig
cargo test -p yoctui --bin yoctui menuconfig
cargo test -p yoctui-ui menuconfig
./scripts/verify-roadmap.sh
```

M67-LIVE-EVIDENCE-001 independently requires a new current-source real-Poky
performance capture. No unblocked implementation task is eligible.
