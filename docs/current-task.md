# Current Task

**ID:** MENUCONFIG-UBOOT-LIVE-001
**Title:** Validate U-Boot interaction after provider ncurses build repair
**Status:** BLOCKED

The Romulus `u-boot-aspeed-sdk` v2019.04+git provider fails before producing
`mconf`. Its legacy `scripts/kconfig/lxdialog/check-lxdialog.sh` probe declares
`main() {}`, which the current compiler rejects. A corrected
`int main(void) { return 0; }` probe links successfully against the recipe's
Yocto ncurses sysroot. Repair that external provider, rerun menuconfig, and then
validate arrows, Enter, search, Ctrl+G resume, and client restart in Firmware.

```bash
bitbake u-boot-aspeed-sdk -c menuconfig
```

This is an external OpenBMC provider compatibility blocker. Do not claim live
U-Boot menuconfig navigation until the provider builds its ncurses frontend.
