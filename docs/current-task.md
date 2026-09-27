# Current Task

**ID:** MENUCONFIG-UBOOT-LIVE-001
**Title:** Validate U-Boot interaction after provider ncurses build repair
**Status:** BLOCKED

M88 implementation and v0.1.241 release are complete. Live Kernel navigation,
search, Ctrl+G out/back and normal client restart recovery passed on Romulus.
Shared Firmware model/input coverage passes. The full workspace suite remains
deferred at the user's request.

External dependency: `u-boot-aspeed-sdk` v2019.04+git fails its ncurses compile
check in `scripts/kconfig/dochecklxdialog`, before an interactive frontend is
available. Reproduced in daemon session 20 using:

```bash
cd /home/bspguy-dev/src/openbmc
source oe-init-build-env build/romulus
bitbake u-boot-aspeed-sdk -c menuconfig
```

The generated upstream wrapper returns 0 after acknowledging the failed make;
do not interpret that as successful ncurses validation. Repair the provider's
compiler/sysroot check, then verify arrows, Enter, search, Ctrl+G out/back and
client quit/restart through Content > U-Boot. Preserve the typed shared flow.

```bash
cargo test -p yoctui-model platform_menuconfig
cargo test -p yoctui --bin yoctui menuconfig
cargo test -p yoctui-ui menuconfig
./scripts/verify-roadmap.sh
```

The other remaining task, M67-LIVE-EVIDENCE-001, independently requires a new
current-source real-Poky performance capture. No unblocked task is eligible.
