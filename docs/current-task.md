# Current Task

**ID:** MENUCONFIG-UBOOT-LIVE-001
**Title:** Validate U-Boot interaction after provider ncurses probe repair
**Status:** BLOCKED

M93 Hardware PDF fallback and v0.1.246 are complete. Full workspace tests remain
deferred at the user's request.

External dependency: `u-boot-aspeed-sdk` v2019.04+git uses the legacy
`scripts/kconfig/lxdialog/check-lxdialog.sh` probe `main() {}`. Current host GCC
rejects that missing return type before linking, but the script hides the
compiler diagnostic and prints its generic ncurses-package message. The same
Yocto native-sysroot include/library flags pass with
`int main(void) { return 0; }`. Add that provider recipe patch, then verify
arrows, Enter, search, Ctrl+G out/back, and client restart in Content > U-Boot.

```bash
cargo test -p yoctui-model platform_menuconfig
cargo test -p yoctui --bin yoctui menuconfig
cargo test -p yoctui-ui menuconfig
```

M67-LIVE-EVIDENCE-001 independently requires a new current-source real-Poky
performance capture. No unblocked implementation task is eligible.
