"""State-checked management extension shared by fixture and local-upstream PTYs."""

import subprocess
import time


def exercise(
    terminal, press, environment_view, top, template, binary, environment, root
):
    build = top / "demo/build"
    conf = build / "conf/toolcfg.conf"
    target = "core/yocto/monitor-disk-space"

    def wait_file(predicate):
        deadline = time.monotonic() + 60
        while not predicate() and time.monotonic() < deadline:
            terminal.read()
        assert predicate(), terminal.screen.text()

    def wait_idle():
        deadline = time.monotonic() + 60
        while time.monotonic() < deadline:
            status = subprocess.run(
                [binary, "sessions"],
                env=environment,
                cwd=root,
                capture_output=True,
                text=True,
                timeout=5,
                check=True,
            ).stdout
            if "Running" not in status and "Starting" not in status:
                return
            terminal.read()
        raise AssertionError(status)

    def edit_id(identifier=target):
        press(b"e", "Edit fragment ID")
        press(b"\x1b[200~" + identifier.encode() + b"\x1b[201~", identifier[-20:])
        press(b"\r", "Enter confirm", "Edit fragment ID")

    press(b"f", "Inspect configuration fragments")
    press(b"\x1b", "Locally discovered setups", "Inspect configuration fragments")
    press(b"f", "Inspect configuration fragments")
    press(b"\r", "3 Fragments / setup details", "Inspect configuration fragments")
    terminal.wait_for(target, absent="Running reviewed inspection", timeout=60)
    before = conf.read_bytes() if conf.exists() else None
    press(b"n", "Enable fragment")
    edit_id()
    press(b"\x1b", "3 Fragments / setup details", "Enter confirm")
    assert (conf.read_bytes() if conf.exists() else None) == before, (
        "review/cancel wrote config"
    )
    press(b"n", "Enable fragment")
    edit_id()
    press(b"\r", "Terminal Sessions", "Enter confirm")
    wait_file(lambda: conf.exists() and target in conf.read_text())
    wait_idle()
    environment_view()
    terminal.wait_for("[x] " + target, absent="Running reviewed inspection", timeout=60)
    # Edit the exact ID on the disable review; never use prefix-wide deletion.
    press(b"d", "Disable fragment")
    edit_id()
    press(b"\r", "Terminal Sessions", "Enter confirm")
    wait_file(lambda: target not in conf.read_text())
    wait_idle()
    environment_view()
    terminal.wait_for("[ ] " + target, absent="Running reviewed inspection", timeout=60)
    if template:
        for machine in ("qemuarm64", "qemux86-64"):
            press(b"n", "Enable fragment")
            edit_id("machine/" + machine)
            press(b"\r", "Terminal Sessions", "Enter confirm")
            wait_file(lambda: "machine/" + machine in conf.read_text())
            wait_idle()
            environment_view()
            terminal.wait_for(
                "[x] machine/" + machine,
                absent="Running reviewed inspection",
                timeout=60,
            )
        assert "machine/qemuarm64" not in conf.read_text(), (
            "built-in prefix must be replaced, not accumulated"
        )
    press(b"s", "Check setup status")
    press(b"n", "disabled (--no-network)")
    press(b"\r", "3 Fragments / setup details", "Enter confirm")
    terminal.wait_for(
        "SETUP_STATUS" if template is None else "has not changed.",
        absent="Running reviewed inspection",
        timeout=60,
    )
    press(b"\x1b", "e enable", "Check setup status")
    press(b"u", "Update setup")
    assert "abort (default)" in terminal.screen.text()
    press(b"\x1b", "e enable", "Enter confirm")
    assert not (top / "demo/config/update-complete").exists()
    if template:
        # Change only the disposable template; upstream updates local symlinks,
        # never edits the developer's OE-Core/BitBake repositories.
        import json

        configuration = json.loads(template.read_text())
        configuration["description"] = "Yoctui reviewed update validation"
        template.write_text(json.dumps(configuration))
    press(b"u", "Update setup")
    press(b"c", "Configuration policy: no")
    press(b"n", "disabled (--no-network)")
    local = build / "conf/local.conf"
    local.write_text(local.read_text() + "\n# Yoctui keep-config validation\n")
    press(b"\r", "Terminal Sessions", "Enter confirm")
    wait_idle()
    if template:
        wait_file(
            lambda: (
                "Yoctui reviewed update validation"
                in (top / "demo/config/config-upstream.json").read_text()
            )
        )
    else:
        wait_file(
            lambda: (
                (top / "demo/config/update-complete").read_text() == "no"
                if (top / "demo/config/update-complete").exists()
                else False
            )
        )
    assert "# Yoctui keep-config validation" in local.read_text(), (
        "keep policy lost local configuration"
    )
    environment_view()
    terminal.wait_for(target, absent="Running reviewed inspection", timeout=60)
    press(b"2", "Locally discovered setups")
    print(
        "Management: fragment review/cancel, enable/disable + refresh, status and safe update passed"
    )
