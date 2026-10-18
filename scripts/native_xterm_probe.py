"""Drive only a test-owned XTerm and capture its actual X11 pixels."""

import ctypes as C
import ctypes.util
import json
import os
from pathlib import Path
import fcntl
import pty
import re
import select
import subprocess
import sys
import termios
import time
import tty

import cairo

from live_ui_probe import KEYS
from pty_acceptance import become_session_leader
from terminal_capture import Screen


def relay(directory):
    """Render the real native PTY in XTerm; inject keys only into that owned PTY."""
    evidence = Path(directory)
    fifo = evidence / "commands.fifo"
    os.mkfifo(fifo, 0o600)
    commands = os.open(fifo, os.O_RDWR | os.O_NONBLOCK)
    master, slave = pty.openpty()
    size = fcntl.ioctl(0, termios.TIOCGWINSZ, bytes(8))
    fcntl.ioctl(slave, termios.TIOCSWINSZ, size)
    environment = os.environ.copy()
    environment["YOCTUI_TERMINAL_GRAPHICS"] = "sixel"
    process = subprocess.Popen(
        [
            os.environ.get("YOCTUI_NATIVE_TEST_BINARY", "/home/bspguy-dev/.cargo/bin/yoctui"),
            "--build-dir",
            "/home/bspguy-dev/src/build-openbmc-romulus",
            "attach",
        ],
        stdin=slave,
        stdout=slave,
        stderr=slave,
        env=environment,
        preexec_fn=become_session_leader,
    )
    os.close(slave)
    original = termios.tcgetattr(0)
    tty.setraw(0)
    output, pending = bytearray(), b""
    try:
        while process.poll() is None:
            ready, _, _ = select.select([master, commands, 0], [], [], 0.05)
            if master in ready:
                try:
                    chunk = os.read(master, 65536)
                except OSError:
                    break
                if not chunk:
                    break
                output.extend(chunk)
                os.write(1, chunk)
            if 0 in ready:
                os.write(master, os.read(0, 4096))
            if commands in ready:
                pending += os.read(commands, 4096)
                while b"\n" in pending:
                    line, pending = pending.split(b"\n", 1)
                    command = json.loads(line)
                    if command.get("quit"):
                        return
                    if "input" in command:
                        os.write(master, command["input"].encode())
                    if "capture" in command:
                        # Native DCS rasters are rendered by XTerm, not text cells.
                        plain = re.sub(
                            rb"\x1bP.*?\x1b\\", b"", bytes(output), flags=re.S
                        )
                        plain = plain.replace(b"\x1b7", b"\x1b[s").replace(
                            b"\x1b8", b"\x1b[u"
                        )
                        screen = Screen(160, 48)
                        screen.feed(plain)
                        (evidence / (command["capture"] + ".txt")).write_text(
                            screen.text()
                        )
                        (evidence / "capture.ansi").write_bytes(output)
                        (evidence / (command["capture"] + ".done")).touch()
    finally:
        termios.tcsetattr(0, termios.TCSANOW, original)
        process.terminate()
        process.wait(timeout=5)
        os.close(master)
        os.close(commands)


class KeyEvent(C.Structure):
    _fields_ = [
        ("type", C.c_int),
        ("serial", C.c_ulong),
        ("send_event", C.c_int),
        ("display", C.c_void_p),
        ("window", C.c_ulong),
        ("root", C.c_ulong),
        ("subwindow", C.c_ulong),
        ("time", C.c_ulong),
        ("x", C.c_int),
        ("y", C.c_int),
        ("x_root", C.c_int),
        ("y_root", C.c_int),
        ("state", C.c_uint),
        ("keycode", C.c_uint),
        ("same_screen", C.c_int),
    ]


class XImage(C.Structure):
    _fields_ = [
        ("width", C.c_int),
        ("height", C.c_int),
        ("xoffset", C.c_int),
        ("format", C.c_int),
        ("data", C.c_void_p),
        ("byte_order", C.c_int),
        ("bitmap_unit", C.c_int),
        ("bitmap_bit_order", C.c_int),
        ("bitmap_pad", C.c_int),
        ("depth", C.c_int),
        ("bytes_per_line", C.c_int),
        ("bits_per_pixel", C.c_int),
        ("red_mask", C.c_ulong),
        ("green_mask", C.c_ulong),
        ("blue_mask", C.c_ulong),
    ]


def main():
    evidence = Path(sys.argv[1]).resolve()
    evidence.mkdir(parents=True, exist_ok=True)
    environment = os.environ.copy()
    for variable in ("CONFIG", "STATE", "DATA"):
        environment[f"XDG_{variable}_HOME"] = str(
            Path(os.environ.get("YOCTUI_NATIVE_TEST_CLIENT_ROOT", "artifacts/e2e/hardware-create")).resolve() / variable.lower()
        )
    environment["YOCTUI_TERMINAL_GRAPHICS"] = "sixel"
    x = C.CDLL(ctypes.util.find_library("X11"))
    x.XOpenDisplay.restype = C.c_void_p
    display = x.XOpenDisplay(None)
    if not display:
        raise RuntimeError("X11 display unavailable")
    for name in (
        "XDefaultRootWindow",
        "XKeysymToKeycode",
        "XQueryTree",
        "XFetchName",
        "XSendEvent",
        "XFlush",
        "XGetGeometry",
        "XCloseDisplay",
    ):
        getattr(x, name).argtypes = [C.c_void_p] + {
            "XDefaultRootWindow": [],
            "XKeysymToKeycode": [C.c_ulong],
            "XQueryTree": [C.c_ulong, C.c_void_p, C.c_void_p, C.c_void_p, C.c_void_p],
            "XFetchName": [C.c_ulong, C.c_void_p],
            "XSendEvent": [C.c_ulong, C.c_int, C.c_long, C.c_void_p],
            "XFlush": [],
            "XGetGeometry": [C.c_ulong] + [C.c_void_p] * 7,
            "XCloseDisplay": [],
        }[name]
    x.XDefaultRootWindow.restype = C.c_ulong
    x.XStringToKeysym.argtypes = [C.c_char_p]
    x.XStringToKeysym.restype = C.c_ulong
    x.XFree.argtypes = [C.c_void_p]
    x.XGetImage.argtypes = [
        C.c_void_p,
        C.c_ulong,
        C.c_int,
        C.c_int,
        C.c_uint,
        C.c_uint,
        C.c_ulong,
        C.c_int,
    ]
    x.XGetImage.restype = C.POINTER(XImage)
    x.XDestroyImage.argtypes = [C.POINTER(XImage)]
    x.XGetInputFocus.argtypes = [C.c_void_p, C.c_void_p, C.c_void_p]
    x.XSetInputFocus.argtypes = [C.c_void_p, C.c_ulong, C.c_int, C.c_ulong]
    x.XSync.argtypes = [C.c_void_p, C.c_int]
    root = x.XDefaultRootWindow(display)
    title = f"Yoctui-native-test-{os.getpid()}"
    process = subprocess.Popen(
        [
            "xterm",
            "-title",
            title,
            "-geometry",
            "160x48+0+0",
            "-fa",
            "DejaVu Sans Mono",
            "-fs",
            "8",
            "-ti",
            "vt340",
            "-xrm",
            "XTerm*allowSendEvents: true",
            "-e",
            sys.executable,
            str(Path(__file__).resolve()),
            "--relay",
            str(evidence),
        ],
        env=environment,
    )

    def children(window):
        returned_root, parent = C.c_ulong(), C.c_ulong()
        pointers, count = C.POINTER(C.c_ulong)(), C.c_uint()
        if not x.XQueryTree(
            display,
            window,
            C.byref(returned_root),
            C.byref(parent),
            C.byref(pointers),
            C.byref(count),
        ):
            return []
        result = [pointers[index] for index in range(count.value)]
        if pointers:
            x.XFree(pointers)
        return result

    def find(window):
        name = C.c_char_p()
        if x.XFetchName(display, window, C.byref(name)) and name:
            found = name.value.decode(errors="replace") == title
            x.XFree(name)
            if found:
                return window
        for child in children(window):
            found = find(child)
            if found:
                return found
        return None

    try:
        deadline = time.monotonic() + 15
        window = None
        while not window and time.monotonic() < deadline:
            window = find(root)
            time.sleep(0.05)
        assert window, "test-owned XTerm window not found"
        fifo = evidence / "commands.fifo"
        while not fifo.exists() and time.monotonic() < deadline:
            time.sleep(0.05)
        channel = os.open(fifo, os.O_WRONLY | os.O_NONBLOCK)

        def send(command):
            os.write(channel, (json.dumps(command) + "\n").encode())

        print(
            json.dumps({"ready": True, "pid": process.pid, "window": window}),
            flush=True,
        )
        for line in sys.stdin:
            command = json.loads(line)
            if command.get("quit"):
                send(command)
                break
            if "key" in command:
                name = command["key"]
                encoded = KEYS.get(
                    name, "\x1b" + name[4:] if name.startswith("Alt+") else name
                )
                send({"input": encoded})
            if "text" in command:
                send({"input": command["text"]})
            if "capture" in command:
                send({"capture": command["capture"]})
                completed = evidence / (command["capture"] + ".done")
                deadline = time.monotonic() + 10
                while not completed.exists() and time.monotonic() < deadline:
                    time.sleep(0.05)
                assert completed.exists(), "native text capture timed out"
                geometry_root = C.c_ulong()
                values = [C.c_int(), C.c_int()] + [C.c_uint() for _ in range(4)]
                x.XGetGeometry(
                    display,
                    window,
                    C.byref(geometry_root),
                    *[C.byref(value) for value in values],
                )
                width, height = values[2].value, values[3].value
                native = x.XGetImage(
                    display, window, 0, 0, width, height, C.c_ulong(-1).value, 2
                )
                assert native, "XGetImage failed"
                try:
                    header = native.contents
                    assert header.bits_per_pixel == 32 and header.byte_order == 0
                    pixels = bytearray(
                        C.string_at(header.data, header.bytes_per_line * height)
                    )
                    surface = cairo.ImageSurface.create_for_data(
                        pixels, cairo.FORMAT_RGB24, width, height, header.bytes_per_line
                    )
                    path = evidence / (command["capture"] + ".png")
                    surface.write_to_png(str(path))
                finally:
                    x.XDestroyImage(native)
                print(
                    json.dumps({"capture": str(path), "size": [width, height]}),
                    flush=True,
                )
            else:
                print(json.dumps({"sent": command}), flush=True)
    finally:
        # Only this test-owned graphical terminal is stopped; daemon work survives.
        process.terminate()
        process.wait(timeout=5)
        x.XCloseDisplay(display)


if __name__ == "__main__":
    if len(sys.argv) > 2 and sys.argv[1] == "--relay":
        relay(sys.argv[2])
    else:
        main()
