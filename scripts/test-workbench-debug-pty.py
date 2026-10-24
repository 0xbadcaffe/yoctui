#!/usr/bin/env python3
"""Native editor and GDB acceptance with isolated files/daemon and a host C program.

The optional gdbserver run validates the remote protocol on loopback, not a Yocto
cross-toolchain or an image build. Requires cc/GDB; --gdbserver selects a helper.
"""
import argparse
import os
from pathlib import Path
import re
import select
import signal
import subprocess
import tempfile
import time

from pty_acceptance import binary_path, isolated_environment, snapshot_fixture, start_terminal
import importlib.util

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("workbench_pty", ROOT / "scripts/test-workbench-pty.py")
existing = importlib.util.module_from_spec(spec)
spec.loader.exec_module(existing)
send, paste = existing.send, existing.paste


def capture(terminal, folder, label):
    (folder / f"{label}.cells").write_text(terminal.screen.cell_golden())
    (folder / f"{label}.txt").write_text(terminal.screen.text())


def wait(terminal, predicate, description):
    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        terminal.read()
        if predicate(terminal.screen.text()):
            return
    raise AssertionError(f"{description}\n{terminal.screen.text()}")


def configure_debug(terminal, binary, local, endpoint):
    send(terminal, b"\x1bg")
    terminal.wait_for("Workbench GDB")
    # The first launch starts in remote mode; the second inherits saved local mode.
    send(terminal, b"\x1b[C")
    terminal.wait_for("Local program" if local else "Remote / QEMU gdbserver")
    send(terminal, b"\t")  # GDB executable
    send(terminal, b"\t")  # executable / symbols
    send(terminal, b"\x15")
    paste(terminal, str(binary))
    send(terminal, b"\t")  # sysroot
    send(terminal, b"\x15")
    paste(terminal, "/")  # Fixture target uses this host's libraries.
    send(terminal, b"\t")  # endpoint
    send(terminal, b"\x15")
    paste(terminal, endpoint)
    send(terminal, b"\t")  # compiled-source prefix
    send(terminal, b"\x15")
    paste(terminal, "/compiled/source")
    send(terminal, b"\r")
    terminal.wait_for("Choose terminal destination")
    terminal.wait_for("Workbench GDB")
    send(terminal, b"\r")
    terminal.wait_for("(gdb)")
    terminal.wait_for("WRITER")
    send(terminal, b"\x1bz")
    terminal.wait_for("(gdb)")


def exercise_gdb(terminal, folder, mode):
    send(terminal, b"run\r" if mode == "local" else b"continue\r")
    terminal.wait_for("Breakpoint 1, helper")
    capture(terminal, folder, f"{mode}-breakpoint")
    send(terminal, b"bt\r")
    terminal.wait_for("#1")
    capture(terminal, folder, f"{mode}-backtrace")
    send(terminal, b"p/x $pc\r")
    wait(terminal, lambda text: bool(re.search(r"\$\d+ = 0x[0-9a-f]+", text)), "program-counter register")
    send(terminal, b"p/x $sp\r")
    terminal.wait_for("$2 = 0x")
    send(terminal, b"list 1,9\r")
    terminal.wait_for("return helper(35)")
    capture(terminal, folder, f"{mode}-registers-source")
    send(terminal, b"\x1be")
    terminal.wait_for("000-main.c · C")
    send(terminal, b"\x1bt")
    terminal.wait_for("WRITER")
    send(terminal, b"\x1bz")
    send(terminal, b"p n\r")
    wait(terminal, lambda text: bool(re.search(r"\$\d+ = 35\b", text)), "same stopped debugger frame")
    capture(terminal, folder, f"{mode}-resumed-debugger")
    send(terminal, b"continue\r")
    terminal.wait_for("exited normally")
    send(terminal, b"quit\r")
    terminal.wait_for("Exited")
    send(terminal, b"\x1be")
    terminal.wait_for("000-main.c · C")


def start_server(program, binary):
    process = subprocess.Popen([str(program), "127.0.0.1:0", str(binary)], stdout=subprocess.PIPE,
                               stderr=subprocess.STDOUT, bufsize=0, start_new_session=True)
    deadline = time.monotonic() + 10
    output = b""
    while time.monotonic() < deadline:
        ready, _, _ = select.select([process.stdout], [], [], .2)
        if ready:
            output += os.read(process.stdout.fileno(), 4096)
            match = re.search(rb"Listening on port (\d+)", output)
            if match:
                return process, "127.0.0.1:" + match.group(1).decode(), output.decode(errors="replace")
        if process.poll() is not None:
            break
    os.killpg(process.pid, signal.SIGTERM)
    try:
        process.wait(timeout=5)
    except subprocess.TimeoutExpired:
        os.killpg(process.pid, signal.SIGKILL)
        process.wait(timeout=5)
    raise AssertionError("gdbserver did not listen:\n" + output.decode(errors="replace"))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--capture-dir", type=Path, required=True)
    parser.add_argument("--gdbserver", type=Path)
    args = parser.parse_args()
    folder = args.capture_dir.resolve()
    folder.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="yoctui-workbench-debug-") as tmp:
        environment = isolated_environment(tmp)
        build = snapshot_fixture(tmp, ROOT, environment)
        source = Path(tmp) / "source with spaces"
        source.mkdir()
        path = source / "000-main.c"
        content = ('#include <stdio.h>\nint helper(int n) {\n\tvolatile int counter = n + 7;\n'
                   '\treturn counter;\n}\nint main(void) {\n\tputs("WORKBENCH_NATIVE_READY");\n'
                   '\treturn helper(35) == 42 ? 0 : 1;\n}\n')
        content += "".join(f"// source row {index:03}\n" for index in range(100))
        path.write_text(content)
        (source / "001-notes.txt").write_text("WORKBENCH_NOTES\n")
        binary = source / "program"
        subprocess.run(["cc", "-g3", "-O0", f"-fdebug-prefix-map={source}=/compiled/source",
                        str(path), "-o", str(binary)], check=True)
        prefix = [binary_path(ROOT), "--backend", "process", "--build-dir", build, "daemon"]
        subprocess.run(prefix + ["start"], env=environment, check=True, capture_output=True)
        terminal = None
        server = None
        try:
            terminal = start_terminal(ROOT, environment, 160, 50, "attach")
            terminal.dismiss_onboarding()
            existing.open_workbench(terminal)
            send(terminal, b"n")
            terminal.wait_for("New workspace · name")
            paste(terminal, "Debug lab")
            send(terminal, b"\r")
            terminal.wait_for("browse filesystem")
            send(terminal, b"\x15")
            paste(terminal, str(source))
            send(terminal, b"\r")
            terminal.wait_for("000-main.c")
            send(terminal, b"\r")
            terminal.wait_for("volatile int counter")
            send(terminal, b"\x1bw")
            terminal.wait_for("→")
            capture(terminal, folder, "whitespace-syntax")
            assert path.read_text() == content, "whitespace rendering changed source"
            send(terminal, b"\x1bw")
            send(terminal, b"\x06")
            terminal.wait_for("Find in current file")
            paste(terminal, "counter")
            send(terminal, b"\r")
            terminal.wait_for("Find 1/2")
            send(terminal, b"\x07")  # Ctrl+g also works when Alt+n is composed by the desktop.
            terminal.wait_for("Find 2/2")
            send(terminal, b"\x1bp")
            terminal.wait_for("Find 1/2")
            send(terminal, b"\x06")
            terminal.wait_for("Find in current file")
            send(terminal, b"\x15\r")
            terminal.wait_for("000-main.c · C", absent="Find in current file")
            send(terminal, b"\x1b[1;5F")  # Ctrl+End
            terminal.wait_for("// source row 099")
            send(terminal, b"\x1b[1;5H")  # Ctrl+Home
            terminal.wait_for("Ln 1 Col 1")
            send(terminal, b"\x1b[<65;100;20M")  # wheel over editor
            wait(terminal, lambda text: "Ln 1 Col 1" in text and "#include" not in next(
                text.splitlines()[i+1] for i,line in enumerate(text.splitlines()) if "000-main.c · C" in line),
                "wheel should scroll without moving cursor")
            capture(terminal, folder, "independent-scroll")
            send(terminal, b"\x1b[1;5H")
            terminal.wait_for("#include")
            send(terminal, b"\x1bf")
            send(terminal, b"\x1b[B")
            send(terminal, b"\r")
            terminal.wait_for("WORKBENCH_NOTES")
            send(terminal, b"\x1b1")
            terminal.wait_for("000-main.c · C")
            send(terminal, b"\x1bv")
            terminal.wait_for("Editor tabs")
            capture(terminal, folder, "tab-picker")
            send(terminal, b"\x1b")
            terminal.wait_for("000-main.c · C", absent="Editor tabs")
            send(terminal, b"\x1b2")
            terminal.wait_for("001-notes.txt ·")
            send(terminal, b"\x17")  # Ctrl+w
            terminal.wait_for("000-main.c · C")
            send(terminal, b"\x1b[1;5H")
            send(terminal, b"\x1b[B")
            send(terminal, b"\x1b[B")
            terminal.wait_for("Ln 3 Col 1")
            send(terminal, b"\x1b[20~")  # F9 stages a line breakpoint.
            terminal.wait_for("●  3")
            capture(terminal, folder, "staged-breakpoint")
            configure_debug(terminal, binary, True, "127.0.0.1:2345")
            exercise_gdb(terminal, folder, "local")
            if args.gdbserver:
                server, endpoint, startup = start_server(args.gdbserver.resolve(), binary)
                configure_debug(terminal, binary, False, endpoint)
                exercise_gdb(terminal, folder, "remote")
                output, _ = server.communicate(timeout=5)
                (folder / "gdbserver.log").write_text(startup + output.decode(errors="replace"))
                server = None
            assert path.read_text() == content
            send(terminal, b"\x1bo")
            terminal.wait_for("Workbench · Sessions")
            send(terminal, b"\x1b")
            terminal.wait_for("h/l groups")
            terminal.finish()
            print("REAL PASS: whitespace/syntax, independent scrolling, search, tabs, staged breakpoint,")
            print("local GDB breakpoint/backtrace/registers/source, editor return and same-session resume.")
            print("REAL PASS: loopback remote GDB protocol" if args.gdbserver else "BLOCKED: remote GDB test needs --gdbserver")
            print("Build/SDK command execution is outside this fixture test.")
        except BaseException:
            if terminal is not None:
                capture(terminal, folder, "failure")
            raise
        finally:
            if server is not None:
                try:
                    os.killpg(server.pid, signal.SIGTERM)
                    server.wait(timeout=5)
                except (ProcessLookupError, subprocess.TimeoutExpired):
                    if server.poll() is None:
                        os.killpg(server.pid, signal.SIGKILL)
                        server.wait(timeout=5)
            if terminal is not None:
                terminal.cleanup()
                os.close(terminal.master)
            subprocess.run(prefix + ["stop"], env=environment, capture_output=True, timeout=15)


if __name__ == "__main__":
    main()
