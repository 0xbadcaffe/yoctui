# Screenshots

<p align="center">
  <a href="https://github.com/0xbadcaffe/yoctui/blob/master/docs/media/screenshots/07-idle-dashboard.png"><img src="https://raw.githubusercontent.com/0xbadcaffe/yoctui/master/docs/media/screenshots/07-idle-dashboard.png" alt="Yoctui dashboard with workspace status, recent builds and CPU, RAM and filesystem usage"></a>
</p>

Screens use fixture values rendered through Yoctui at `160x50`, with the optional
inspector enabled. GitUI panes replay native output from a demo repository.
See [Raster provenance](https://github.com/0xbadcaffe/yoctui/blob/master/docs/media/screenshots/manifest.toml)
for source and image checksums.

### Set up a workspace

<table>
  <tr>
    <td width="50%"><a href="https://github.com/0xbadcaffe/yoctui/blob/master/docs/media/screenshots/13-cloning.png"><img src="https://raw.githubusercontent.com/0xbadcaffe/yoctui/master/docs/media/screenshots/13-cloning.png" alt="Fresh clone"></a><br><strong>Clone sources</strong> — Background cloning with a Braille <code>Cloning…</code> indicator.</td>
    <td width="50%"><a href="https://github.com/0xbadcaffe/yoctui/blob/master/docs/media/screenshots/18-offline-dashboard.png"><img src="https://raw.githubusercontent.com/0xbadcaffe/yoctui/master/docs/media/screenshots/18-offline-dashboard.png" alt="Offline Dashboard with saved builds and setup guidance"></a><br><strong>Offline dashboard</strong> — Configure paths, reconnect the daemon or open saved builds.</td>
  </tr>
</table>

### Build and debug

<table>
  <tr>
    <td width="50%"><a href="https://github.com/0xbadcaffe/yoctui/blob/master/docs/media/screenshots/01-active-build-tasks.png"><img src="https://raw.githubusercontent.com/0xbadcaffe/yoctui/master/docs/media/screenshots/01-active-build-tasks.png" alt="Yoctui active BitBake tasks and correlated build logs"></a><br><strong>Tasks</strong> — Active BitBake tasks, build progress and task logs.</td>
    <td width="50%"><a href="https://github.com/0xbadcaffe/yoctui/blob/master/docs/media/screenshots/08-failed-build-errors.png"><img src="https://raw.githubusercontent.com/0xbadcaffe/yoctui/master/docs/media/screenshots/08-failed-build-errors.png" alt="Yoctui failed BitBake task errors and correlated logs"></a><br><strong>Errors</strong> — Failed tasks, diagnostics, source logs and recovery actions.</td>
  </tr>
  <tr>
    <td width="50%"><a href="https://github.com/0xbadcaffe/yoctui/blob/master/docs/media/screenshots/14-cancelling.png"><img src="https://raw.githubusercontent.com/0xbadcaffe/yoctui/master/docs/media/screenshots/14-cancelling.png" alt="Background cancellation"></a><br><strong>Cancel a build</strong> — Cancellation runs in the background; navigation stays available.</td>
    <td width="50%"><a href="https://github.com/0xbadcaffe/yoctui/blob/master/docs/media/screenshots/15-search-empty.png"><img src="https://raw.githubusercontent.com/0xbadcaffe/yoctui/master/docs/media/screenshots/15-search-empty.png" alt="Content search"></a><br><strong>Search build output</strong> — <code>/</code> opens an empty search of build files, generated rootfs and text image artifacts.</td>
  </tr>
</table>

### Read previous builds

<table>
  <tr>
    <td width="50%"><a href="https://github.com/0xbadcaffe/yoctui/blob/master/docs/media/screenshots/19-saved-build-history.png"><img src="https://raw.githubusercontent.com/0xbadcaffe/yoctui/master/docs/media/screenshots/19-saved-build-history.png" alt="Saved build history without a daemon connection"></a><br><strong>Build history</strong> — Browse saved outcomes without a configured environment or daemon connection.</td>
    <td width="50%"><a href="https://github.com/0xbadcaffe/yoctui/blob/master/docs/media/screenshots/20-saved-build-logs.png"><img src="https://raw.githubusercontent.com/0xbadcaffe/yoctui/master/docs/media/screenshots/20-saved-build-logs.png" alt="Read-only saved build logs with provenance"></a><br><strong>Saved logs and tasks</strong> — Read retained build details and log excerpts; missing records and retention limits are shown.</td>
  </tr>
</table>

### Edit sources and commit changes

<table>
  <tr>
    <td width="50%"><a href="https://github.com/0xbadcaffe/yoctui/blob/master/docs/media/screenshots/09-editor-application-menu.png"><img src="https://raw.githubusercontent.com/0xbadcaffe/yoctui/master/docs/media/screenshots/09-editor-application-menu.png" alt="Yoctui BitBake recipe editor and application action menu"></a><br><strong>Recipe editor and menus</strong> — Syntax highlighting, validation and context actions. Use arrows to navigate menus and Escape to return; unavailable actions show the reason.</td>
    <td width="50%"><a href="https://github.com/0xbadcaffe/yoctui/blob/master/docs/media/screenshots/10-terminal-sessions.png"><img src="https://raw.githubusercontent.com/0xbadcaffe/yoctui/master/docs/media/screenshots/10-terminal-sessions.png" alt="Yoctui split daemon-owned terminal sessions"></a><br><strong>Terminal sessions</strong> — Split build shells and devshells with writer control and scrollback.</td>
  </tr>
  <tr>
    <td width="50%"><a href="https://github.com/0xbadcaffe/yoctui/blob/master/docs/media/screenshots/16-gitui-diff.png"><img src="https://raw.githubusercontent.com/0xbadcaffe/yoctui/master/docs/media/screenshots/16-gitui-diff.png" alt="Source Git and diffs"></a><br><strong>Git status and diffs</strong> — Repository status in the header; native GitUI for reviewing and staging changes.</td>
    <td width="50%"><a href="https://github.com/0xbadcaffe/yoctui/blob/master/docs/media/screenshots/17-gitui-commit.png"><img src="https://raw.githubusercontent.com/0xbadcaffe/yoctui/master/docs/media/screenshots/17-gitui-commit.png" alt="Commit messages"></a><br><strong>Commit changes</strong> — Enter commit messages in GitUI inside a Yoctui terminal.</td>
  </tr>
</table>

### Configure the kernel and bootloader

<table>
  <tr>
    <td width="50%"><a href="https://github.com/0xbadcaffe/yoctui/blob/master/docs/media/screenshots/02-kernel-device-tree.png"><img src="https://raw.githubusercontent.com/0xbadcaffe/yoctui/master/docs/media/screenshots/02-kernel-device-tree.png" alt="Yoctui Kernel device-tree inventory"></a><br><strong>Kernel device trees</strong> — DTS, DTSI and compiled DTB files for the selected provider.</td>
    <td width="50%"><a href="https://github.com/0xbadcaffe/yoctui/blob/master/docs/media/screenshots/03-uboot-device-tree.png"><img src="https://raw.githubusercontent.com/0xbadcaffe/yoctui/master/docs/media/screenshots/03-uboot-device-tree.png" alt="Yoctui U-Boot device-tree inventory"></a><br><strong>U-Boot device trees</strong> — Bootloader sources and generated device-tree artifacts.</td>
  </tr>
  <tr>
    <td width="50%"><a href="https://github.com/0xbadcaffe/yoctui/blob/master/docs/media/screenshots/04-kernel-menuconfig.png"><img src="https://raw.githubusercontent.com/0xbadcaffe/yoctui/master/docs/media/screenshots/04-kernel-menuconfig.png" alt="Linux kernel menuconfig inside a Yoctui terminal session"></a><br><strong>Kernel menuconfig</strong> — Native ncurses controls in a daemon-owned terminal.</td>
    <td width="50%"><a href="https://github.com/0xbadcaffe/yoctui/blob/master/docs/media/screenshots/05-uboot-menuconfig.png"><img src="https://raw.githubusercontent.com/0xbadcaffe/yoctui/master/docs/media/screenshots/05-uboot-menuconfig.png" alt="U-Boot menuconfig inside a Yoctui terminal session"></a><br><strong>U-Boot menuconfig</strong> — The selected provider’s ncurses interface in a reconnectable PTY.</td>
  </tr>
  <tr>
    <td width="50%"><a href="https://github.com/0xbadcaffe/yoctui/blob/master/docs/media/screenshots/11-device-tree-editor.png"><img src="https://raw.githubusercontent.com/0xbadcaffe/yoctui/master/docs/media/screenshots/11-device-tree-editor.png" alt="Yoctui Device Tree source editor with DTS syntax highlighting"></a><br><strong>Device Tree editor</strong> — Linux v6.6 <a href="https://github.com/torvalds/linux/blob/v6.6/arch/arm64/boot/dts/freescale/imx8mp-evk.dts">NXP i.MX8MP EVK DTS</a>, with highlighted directives, nodes, properties, values and comments.</td>
    <td width="50%"><a href="https://github.com/0xbadcaffe/yoctui/blob/master/docs/media/screenshots/12-device-tree-compile-options.png"><img src="https://raw.githubusercontent.com/0xbadcaffe/yoctui/master/docs/media/screenshots/12-device-tree-compile-options.png" alt="Yoctui dtc compile-options dialog for a kernel Device Tree source"></a><br><strong>Device Tree compiler</strong> — Set symbols, sorting, padding and reserve entries, then review the exact <code>dtc</code> command.</td>
  </tr>
</table>

### Inspect the generated image

<table>
  <tr>
    <td width="50%"><a href="https://github.com/0xbadcaffe/yoctui/blob/master/docs/media/screenshots/06-rootfs-composition.png"><img src="https://raw.githubusercontent.com/0xbadcaffe/yoctui/master/docs/media/screenshots/06-rootfs-composition.png" alt="Yoctui root filesystem package composition pie chart and exact size table"></a><br><strong>Rootfs composition</strong> — Braille package-size chart with matching table colors, exact byte totals and filesystem drill-down.</td>
    <td width="50%"><a href="https://github.com/0xbadcaffe/yoctui/blob/master/docs/media/screenshots/21-systemd-services.png"><img src="https://raw.githubusercontent.com/0xbadcaffe/yoctui/master/docs/media/screenshots/21-systemd-services.png" alt="Offline systemd Services view listing unit files, descriptions, BusName and enablement from IMAGE_ROOTFS"></a><br><strong>Offline systemd services</strong> — Inspect unit files, D-Bus names and enablement links in <code>IMAGE_ROOTFS</code>. These are installed files, not live service status.</td>
  </tr>
  <tr>
    <td width="50%"><a href="https://github.com/0xbadcaffe/yoctui/blob/master/docs/media/screenshots/22-system-dbus.png"><img src="https://raw.githubusercontent.com/0xbadcaffe/yoctui/master/docs/media/screenshots/22-system-dbus.png" alt="Offline system D-Bus activation map with bus names, systemd units, users, executables and policy counts"></a><br><strong>System D-Bus</strong> — Inspect activation files, associated systemd units and policy files from the image.</td>
    <td width="50%"><a href="https://github.com/0xbadcaffe/yoctui/blob/master/docs/media/screenshots/23-udev-rules.png"><img src="https://raw.githubusercontent.com/0xbadcaffe/yoctui/master/docs/media/screenshots/23-udev-rules.png" alt="Offline udev rule files showing overrides, masks and selected rule content"></a><br><strong>udev rules</strong> — Check file precedence, overrides and masks, then read the selected rule. Rules are not executed.</td>
  </tr>
</table>
