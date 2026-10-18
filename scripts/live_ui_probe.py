"""State-checked native PTY probe; captures are evidence, not live-operation proof."""

import argparse
import codecs
import fcntl
import json
import os
from pathlib import Path
import pty
import re
import select
import struct
import subprocess
import sys
import termios
import time

from pty_acceptance import TerminalAcceptance, become_session_leader

KEYS = {
    "F1": "\x1bOP",
    "F2": "\x1bOQ",
    "F3": "\x1bOR",
    "F4": "\x1bOS",
    "F5": "\x1b[15~",
    "F6": "\x1b[17~",
    "F7": "\x1b[18~",
    "F8": "\x1b[19~",
    "F9": "\x1b[20~",
    "F12": "\x1b[24~",
    "Esc": "\x1b",
    "Enter": "\r",
    "Tab": "\t",
    "Backtab": "\x1b[Z",
    "Up": "\x1b[A",
    "Down": "\x1b[B",
    "Right": "\x1b[C",
    "Left": "\x1b[D",
    "PgUp": "\x1b[5~",
    "PgDn": "\x1b[6~",
    "Home": "\x1b[H",
    "End": "\x1b[F",
    "Ctrl+p": "\x10",
    "Ctrl+u": "\x15",
    "Ctrl+s": "\x13",
    "Ctrl+v": "\x16",
    "Ctrl+b": "\x02",
    "Ctrl+c": "\x03",
}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", required=True)
    parser.add_argument("--build-dir", required=True)
    parser.add_argument("--evidence", required=True)
    parser.add_argument("--client-root")
    parser.add_argument("--trace", action="store_true")
    parser.add_argument("--interactive", action="store_true")
    parser.add_argument("--steps", default="[]")
    parser.add_argument("--width", type=int, default=160)
    parser.add_argument("--height", type=int, default=48)
    args = parser.parse_args()
    evidence = Path(args.evidence).resolve()
    evidence.mkdir(parents=True, exist_ok=True)
    environment = os.environ.copy()
    environment["TERM"] = "xterm-256color"
    environment["YOCTUI_TERMINAL_GRAPHICS"] = "none"
    # Keep the user's runtime/socket and the initialized daemon environment.
    # Isolate only client preferences/history and hardware project mutations.
    for name in ("CONFIG", "STATE", "DATA"):
        directory = (
            Path(args.client_root).resolve() / name.lower()
            if args.client_root
            else evidence / name.lower()
        )
        directory.mkdir(mode=0o700, exist_ok=True)
        environment[f"XDG_{name}_HOME"] = str(directory)
    master, slave = pty.openpty()
    fcntl.ioctl(
        slave, termios.TIOCSWINSZ, struct.pack("HHHH", args.height, args.width, 0, 0)
    )
    command = [args.binary, "--build-dir", args.build_dir, "attach"]
    if args.trace:
        command = [
            "strace",
            "-f",
            "-o",
            str(evidence / "syscalls.capture"),
            "-s",
            "512",
            "-e",
            "trace=read,write,sendto,recvfrom,sendmsg,recvmsg",
            *command,
        ]
    startup_started = time.monotonic()
    process = subprocess.Popen(
        command,
        stdin=slave,
        stdout=slave,
        stderr=slave,
        env=environment,
        preexec_fn=become_session_leader,
    )
    os.close(slave)
    terminal = TerminalAcceptance(master, process, bytearray(), args.width, args.height)
    results = []

    def capture(label):
        text = terminal.screen.text()
        (evidence / f"{label}.txt").write_text(text)
        (evidence / f"{label}.cells").write_text(terminal.screen.cell_golden())
        return text

    def drain(duration):
        deadline = time.monotonic() + duration
        while time.monotonic() < deadline:
            terminal.read(timeout=min(0.05, max(0, deadline - time.monotonic())))

    def selected_rows():
        # Sidebar widths vary by workspace and terminal width. Do not cut the
        # first filename character off a selected artifact (a false mismatch).
        sidebar = "Navigator" in "\n".join(terminal.screen.text().splitlines()[:10])
        start = 20 if sidebar else 0
        return [
            "".join(cell.symbol for cell in row[start:]).strip()
            for row in terminal.screen.cells[7:-3]
            if sum(cell.style.background == "Rgb(13, 57, 132)" for cell in row[start:])
            > 3
        ]

    def selected_identities():
        # Task timing/progress changes must not impersonate selection movement.
        identities = []
        for row in selected_rows():
            task = re.search(r"\b(do_\w+)\s+(\S+)", row)
            identities.append(task.groups() if task else row)
        return identities

    label = "startup"

    def steps():
        if not args.interactive:
            yield from json.loads(args.steps)
            return
        pending = ""
        decoder = codecs.getincrementaldecoder("utf-8")()
        while True:
            if "\n" in pending:
                line, pending = pending.split("\n", 1)
                command = json.loads(line)
                if command.get("finish"):
                    return
                yield command
                continue
            ready, _, _ = select.select([sys.stdin], [], [], 0.05)
            if not ready:
                terminal.read(timeout=0)
                continue
            received = os.read(sys.stdin.fileno(), 65536)
            if not received:
                return
            pending += decoder.decode(received)

    try:
        terminal.wait_for("yoctui", timeout=60)
        drain(1)
        if "Esc dismiss" in terminal.screen.text():
            os.write(master, b"\x1b")
        terminal.wait_for("Quit", absent="Esc dismiss", timeout=90)
        (evidence / "ready.json").write_text(
            json.dumps(
                {
                    "seconds_including_one_second_settle": round(
                        time.monotonic() - startup_started, 3
                    ),
                    "width": args.width,
                    "height": args.height,
                    "binary": args.binary,
                    "build_dir": args.build_dir,
                },
                indent=2,
            )
        )
        capture("000-start")
        print("READY: " + terminal.screen.text().splitlines()[1], flush=True)
        for index, step in enumerate(steps(), 1):
            label = f"{index:03d}-{step.get('label', 'step')}"
            started = time.monotonic()
            before_selection = selected_identities()
            if "browser_entry" in step:
                name = step["browser_entry"]
                pattern = re.compile(r"\b(?:DIR|FILE)\s+" + re.escape(name) + r"\s*│")
                terminal.wait_for("Import any file", absent="Loading/saving")
                for _ in range(150):
                    if any(pattern.search(row) for row in selected_rows()):
                        break
                    os.write(master, KEYS["Down"].encode())
                    drain(0.12)
                else:
                    raise AssertionError(
                        f"Import entry not selected: {name}; {selected_rows()}"
                    )
                os.write(master, KEYS["Enter"].encode())
                drain(0.2)
            if "route" in step:
                os.write(master, KEYS["F4"].encode())
                terminal.wait_for("Build Overview")
                os.write(master, KEYS["Ctrl+p"].encode())
                terminal.wait_for("Command Palette")
                os.write(master, KEYS["Ctrl+u"].encode())
                command_label = step.get("command_label", "Open " + step["route"])
                os.write(master, command_label.encode())
                terminal.wait_for("▶ " + command_label)
                os.write(master, KEYS["Enter"].encode())
                terminal.wait_for("Quit", absent="Command Palette")
                if (
                    step.get("activate_workspace", True)
                    and "h/l groups" in terminal.screen.text().splitlines()[-2]
                ):
                    os.write(master, KEYS["Enter"].encode())
                    drain(0.15)
            if "artifact" in step:
                os.write(master, KEYS["Right"].encode())
                drain(0.2)
                for _ in range(512):
                    if any(step["artifact"] in row for row in selected_rows()):
                        break
                    os.write(master, KEYS["Down"].encode())
                    drain(0.12)
                else:
                    raise AssertionError(
                        f"Artifact not selected: {step['artifact']}; {selected_rows()}"
                    )
            if "select_from_end" in step:
                os.write(master, KEYS["End"].encode())
                drain(0.2)
                for _ in range(256):
                    if any(step["select_from_end"] in row for row in selected_rows()):
                        break
                    os.write(master, KEYS["Up"].encode())
                    drain(0.12)
                else:
                    raise AssertionError(f"Cannot select {step['select_from_end']}")
            if "picker_target" in step:
                target = step["picker_target"]
                terminal.wait_for("Available image targets")
                for _ in range(512):
                    selected = [
                        row
                        for row in terminal.screen.text().splitlines()
                        if "▶ " in row
                        and target
                        in row.split("▶ ", 1)[1].split("│", 1)[0].strip().split()
                    ]
                    if selected:
                        break
                    os.write(master, KEYS["Down"].encode())
                    drain(0.12)
                else:
                    raise AssertionError(f"Image picker could not select {target}")
                os.write(master, KEYS["Enter"].encode())
                terminal.wait_for(
                    step.get("picker_expect", "build target " + target),
                    absent="Available image targets",
                )
            if "resize" in step:
                terminal.resize(*step["resize"])
            if "click_text" in step:
                rows = terminal.screen.text().splitlines()
                matches = [
                    (row.find(step["click_text"]) + 2, index + 1)
                    for index, row in enumerate(rows)
                    if step["click_text"] in row
                ]
                assert len(matches) == 1, matches
                x, y = matches[0]
                os.write(master, f"\x1b[<0;{x};{y}M\x1b[<0;{x};{y}m".encode())
            if "mouse" in step:
                button, x, y = step["mouse"]
                os.write(master, f"\x1b[<{button};{x};{y}M".encode())
            if "key" in step:
                key = step["key"]
                encoded = KEYS.get(
                    key,
                    "\x02" + key[7:]
                    if key.startswith("Prefix+")
                    else "\x1b" + key[4:]
                    if key.startswith("Alt+")
                    else key,
                )
                os.write(master, encoded.encode())
            if "paste" in step:
                os.write(master, ("\x1b[200~" + step["paste"] + "\x1b[201~").encode())
            if "text" in step:
                os.write(master, step["text"].encode())
            try:
                if "backend_job" in step:
                    deadline = time.monotonic() + step.get("timeout", 60)
                    status = ""
                    while time.monotonic() < deadline:
                        drain(0.1)
                        status = subprocess.run(
                            [args.binary, "daemon", "status"],
                            env=environment,
                            capture_output=True,
                            text=True,
                            timeout=5,
                        ).stdout
                        pattern = rf"(?m)^job {step['backend_job']} "
                        if re.search(pattern, status):
                            break
                        drain(0.5)
                    else:
                        raise AssertionError(
                            f"Daemon job {step['backend_job']} not observed: {status}"
                        )
                    (evidence / f"{label}.backend.txt").write_text(status)
                if "expect" in step:
                    terminal.wait_for(
                        step["expect"],
                        absent=step.get("absent"),
                        timeout=step.get("timeout", 30),
                    )
                else:
                    drain(step.get("observe_seconds", 0.25))
                if "observe_seconds" in step and "expect" in step:
                    drain(step["observe_seconds"])
                if "expect_regex" in step:
                    deadline = time.monotonic() + step.get("timeout", 30)
                    pattern = re.compile(step["expect_regex"])
                    while time.monotonic() < deadline:
                        received = terminal.read()
                        if not received and pattern.search(terminal.screen.text()):
                            break
                    else:
                        raise AssertionError(
                            f"Current frame did not match {pattern.pattern}"
                        )
                text = capture(label)
                if step.get("menu_centered"):
                    top = next(
                        row
                        for row in text.splitlines()
                        if "╔Yoctui Application Menu" in row
                    )
                    center = (top.index("╔") + top.rindex("╗")) / 2
                    assert abs(center - (terminal.screen.width - 1) / 2) <= 2, center
                if "contains" in step:
                    expected = step["contains"]
                    expected = [expected] if isinstance(expected, str) else expected
                    assert all(item in text for item in expected), text
                if "selection_contains" in step:
                    assert any(
                        step["selection_contains"] in row for row in selected_rows()
                    ), selected_rows()
                if step.get("selection_changed"):
                    assert selected_identities() != before_selection, selected_rows()
                results.append(
                    {
                        "label": label,
                        "status": "REAL PASS"
                        if "expect" in step
                        or "contains" in step
                        or "selection_contains" in step
                        or step.get("selection_changed")
                        or "backend_job" in step
                        or "expect_regex" in step
                        else "OBSERVATION ONLY",
                        "selected_rows": selected_rows(),
                        "seconds": round(time.monotonic() - started, 3),
                    }
                )
                print(
                    label
                    + " "
                    + results[-1]["status"]
                    + "\n"
                    + "\n".join(text.splitlines()[5:10]),
                    flush=True,
                )
            except (AssertionError, OSError) as error:
                capture(label + "-failure")
                results.append({"label": label, "status": "FAIL", "error": str(error)})
                if args.interactive:
                    print(f"{label} FAIL: {error}", flush=True)
                    continue
                raise
        terminal.unwind()
        terminal.finish()
    except (AssertionError, OSError) as error:
        # Picker/route failures occur before the individual assertion block.
        if not results or results[-1]["status"] != "FAIL":
            capture(label + "-failure")
            results.append({"label": label, "status": "FAIL", "error": str(error)})
        raise
    finally:
        capture("last-frame")
        (evidence / "results.json").write_text(json.dumps(results, indent=2))
        (evidence / "capture.ansi").write_bytes(terminal.raw)
        terminal.cleanup()
        os.close(master)


if __name__ == "__main__":
    main()
