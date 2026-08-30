#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

test_binary="${YOCTUI_TEST_BINARY:-}"
if [[ -z "$test_binary" ]]; then
  # Keep every terminal layer explicit so a renamed or missing test cannot be
  # hidden by a broad workspace invocation.
  cargo test -q -p yoctui --bin yoctui ux_terminal_runtime
  cargo test -q -p yoctui --test daemon_pty_runtime ux_terminal_real_pty
  cargo test -q -p yoctui --test daemon_state_runtime reattach
  cargo test -q -p yoctui --bin yoctui pty_attach
  cargo test -q -p yoctui --bin yoctui daemon_raw::tests::raw_pty
  cargo test -q -p yoctui-bitbake --lib pty_runner
  cargo test -q -p yoctui-model --lib ux_terminal
  cargo test -q -p yoctui-model --lib pty_session
  cargo test -q -p yoctui-model --lib terminal_emulation
  cargo test -q -p yoctui-app --lib ux_terminal
  cargo test -q -p yoctui-app --lib mouse_runtime_routes_dialog_and_terminal
  cargo test -q -p yoctui-ui --lib ux_terminal
  cargo test -q -p yoctui-protocol --lib next_generation_pty
  cargo test -q -p yoctui-e2e --lib next_generation_pty
  cargo build -q -p yoctui
  test_binary="$repo_root/target/debug/yoctui"
else
  test_binary="$(realpath -- "$test_binary")"
  test -x "$test_binary"
fi

harness_root="$(mktemp -d /tmp/yoctui-workbench-terminal.XXXXXX)"
export XDG_CONFIG_HOME="$harness_root/config"
export XDG_STATE_HOME="$harness_root/state"
export XDG_RUNTIME_DIR="$harness_root/runtime"
mkdir -m 700 "$XDG_CONFIG_HOME" "$XDG_STATE_HOME" "$XDG_RUNTIME_DIR"
mkdir -m 700 "$harness_root/build"

cleanup() {
  "$test_binary" daemon stop >/dev/null 2>&1 || true
  rm -rf -- "$harness_root"
}
trap cleanup EXIT

"$test_binary" daemon start >/dev/null

python3 - "$repo_root" "$test_binary" "${YOCTUI_TERMINAL_EVIDENCE:-}" <<'PY'
import hashlib
import json
import fcntl
import os
import pty
import re
import select
import struct
import subprocess
import sys
import termios
import time
import unicodedata

root = sys.argv[1]
binary = sys.argv[2]
evidence = sys.argv[3]
ansi = re.compile(r"\x1b(?:\[[0-9;?]*[ -/]*[@-~]|\][^\x07]*(?:\x07|\x1b\\)|.)")
screens = {}


class Screen:
    """Compose the cursor-addressed crossterm stream into visible terminal rows."""

    def __init__(self, rows=50, columns=160):
        self.rows = rows
        self.columns = columns
        self.cells = [[" "] * columns for _ in range(rows)]
        self.row = 0
        self.column = 0
        self.saved = (0, 0)
        self.pending = ""
        self.transcript = bytearray()

    def resize(self, rows, columns):
        resized = [[" "] * columns for _ in range(rows)]
        for row in range(min(rows, self.rows)):
            for column in range(min(columns, self.columns)):
                resized[row][column] = self.cells[row][column]
        self.rows = rows
        self.columns = columns
        self.cells = resized
        self.row = min(self.row, rows - 1)
        self.column = min(self.column, columns - 1)

    def text(self):
        return "\n".join("".join(row).rstrip() for row in self.cells)

    def feed(self, raw):
        self.transcript.extend(raw)
        self.pending += raw.decode("utf-8", "replace")
        index = 0
        while index < len(self.pending):
            character = self.pending[index]
            if character != "\x1b":
                self.write(character)
                index += 1
                continue
            if index + 1 >= len(self.pending):
                break
            kind = self.pending[index + 1]
            if kind == "[":
                match = re.match(r"\x1b\[([0-9;?]*)([ -/]*)?([@-~])", self.pending[index:])
                if match is None:
                    break
                self.csi(match.group(1), match.group(3))
                index += len(match.group(0))
                continue
            if kind == "]":
                bell = self.pending.find("\x07", index + 2)
                string_term = self.pending.find("\x1b\\", index + 2)
                endings = [value for value in (bell, string_term) if value >= 0]
                if not endings:
                    break
                end = min(endings)
                index = end + (2 if self.pending[end:end + 2] == "\x1b\\" else 1)
                continue
            if kind in "()":
                if index + 2 >= len(self.pending):
                    break
                index += 3
                continue
            index += 2
        self.pending = self.pending[index:]

    def write(self, character):
        if character == "\r":
            self.column = 0
        elif character == "\n":
            self.row = min(self.rows - 1, self.row + 1)
        elif character == "\b":
            self.column = max(0, self.column - 1)
        elif character == "\t":
            self.column = min(self.columns - 1, (self.column // 8 + 1) * 8)
        elif ord(character) >= 0x20 and character != "\x7f":
            width = 2 if unicodedata.east_asian_width(character) in ("W", "F") else 1
            if unicodedata.combining(character):
                width = 0
            if self.row < self.rows and self.column < self.columns:
                self.cells[self.row][self.column] = character
            self.column = min(self.columns - 1, self.column + width)

    def csi(self, parameters, command):
        values = [int(value) if value else 0 for value in parameters.lstrip("?").split(";")]
        first = values[0] if values else 0
        amount = first or 1
        if command in ("H", "f"):
            self.row = min(self.rows - 1, max(0, (values[0] if values else 1) - 1))
            self.column = min(
                self.columns - 1,
                max(0, (values[1] if len(values) > 1 else 1) - 1),
            )
        elif command == "A":
            self.row = max(0, self.row - amount)
        elif command == "B":
            self.row = min(self.rows - 1, self.row + amount)
        elif command == "C":
            self.column = min(self.columns - 1, self.column + amount)
        elif command == "D":
            self.column = max(0, self.column - amount)
        elif command == "E":
            self.row = min(self.rows - 1, self.row + amount)
            self.column = 0
        elif command == "F":
            self.row = max(0, self.row - amount)
            self.column = 0
        elif command in ("G", "`"):
            self.column = min(self.columns - 1, max(0, amount - 1))
        elif command == "d":
            self.row = min(self.rows - 1, max(0, amount - 1))
        elif command == "J":
            if first in (2, 3):
                self.cells = [[" "] * self.columns for _ in range(self.rows)]
            elif first == 0:
                self.cells[self.row][self.column:] = [" "] * (self.columns - self.column)
                for row in range(self.row + 1, self.rows):
                    self.cells[row] = [" "] * self.columns
            elif first == 1:
                for row in range(self.row):
                    self.cells[row] = [" "] * self.columns
                self.cells[self.row][:self.column + 1] = [" "] * (self.column + 1)
        elif command == "K":
            if first == 0:
                self.cells[self.row][self.column:] = [" "] * (self.columns - self.column)
            elif first == 1:
                self.cells[self.row][:self.column + 1] = [" "] * (self.column + 1)
            elif first == 2:
                self.cells[self.row] = [" "] * self.columns
        elif command == "s":
            self.saved = (self.row, self.column)
        elif command == "u":
            self.row, self.column = self.saved


def collect(master, seconds=1):
    raw = bytearray()
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        ready, _, _ = select.select([master], [], [], .08)
        if not ready:
            continue
        try:
            raw.extend(os.read(master, 65536))
        except OSError:
            break
    result = bytes(raw)
    if master in screens:
        screens[master].feed(result)
    return result


def semantic(raw):
    return ansi.sub("", raw.decode("utf-8", "replace"))


def send_and_expect(master, process, keys, expected, label, seconds=2):
    os.write(master, keys)
    raw = collect(master, seconds)
    text = screens[master].text()
    if expected not in text:
        raise SystemExit(
            f"{label} did not render {expected!r} (returncode={process.poll()}): "
            f"text={text[-4000:]!r} raw={raw[-1000:]!r}"
        )
    if "�" in text:
        raise SystemExit(f"{label} emitted a replacement glyph")
    return text


def expect_eventually(master, process, expected, label, seconds=8):
    raw = bytearray()
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        raw.extend(collect(master, .25))
        text = screens[master].text()
        if expected in text:
            if "�" in text:
                raise SystemExit(f"{label} emitted a replacement glyph")
            return text
        if process.poll() is not None:
            break
    raise SystemExit(
        f"{label} did not render {expected!r} (returncode={process.poll()}): "
        f"text={screens[master].text()!r} "
        f"transcript={semantic(screens[master].transcript)[-8000:]!r}"
    )


config_dir = os.path.join(os.environ["XDG_CONFIG_HOME"], "yoctui")
os.mkdir(config_dir, mode=0o700)
with open(os.path.join(config_dir, "config.toml"), "w", encoding="utf-8") as config:
    config.write("")
with open(os.path.join(config_dir, "session.toml"), "w", encoding="utf-8") as session:
    session.write(
        '[onboarding]\n'
        'schema_version = 1\n'
        'current = "environment"\n'
        'completed = []\n'
        'skipped = []\n'
        'dismissed = true\n'
    )

def become_session_leader():
    os.setsid()
    fcntl.ioctl(0, termios.TIOCSCTTY, 0)


def start_client(label):
    master, slave = pty.openpty()
    screens[master] = Screen()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 50, 160, 0, 0))
    process = subprocess.Popen(
        [
            binary,
            "--no-color",
            "--build-dir",
            os.path.realpath(os.path.join(os.environ["XDG_RUNTIME_DIR"], "..", "build")),
        ],
        stdin=slave,
        stdout=slave,
        stderr=slave,
        env=os.environ.copy(),
        preexec_fn=become_session_leader,
    )
    os.close(slave)
    startup = bytearray()
    deadline = time.monotonic() + 8
    while time.monotonic() < deadline and (
        b"\x1b[?1049h" not in startup
        or b"yoctui" not in startup.lower()
        or b"connected" not in startup.lower()
    ):
        startup.extend(collect(master, .2))
    if (
        b"\x1b[?1049h" not in startup
        or b"yoctui" not in startup.lower()
        or b"connected" not in startup.lower()
    ):
        raise SystemExit(
            f"{label} did not attach to the isolated daemon: "
            + semantic(startup)[-4000:]
        )
    return master, process


master, process = start_client("writer client")
send_and_expect(master, process, b"\x02t", "Terminal Sessions", "terminal prefix route")
os.write(master, b"\x02c")
collect(master, .5)
expect_eventually(master, process, "build shell #1 Running", "first daemon-owned PTY")
expect_eventually(master, process, "Running", "first daemon-owned PTY readiness")
os.write(master, b"\x02o")
collect(master, .5)
expect_eventually(master, process, "WRITER", "first writer state")

os.write(master, b"\x02c")
collect(master, .5)
os.write(master, b"\x02n")
expect_eventually(master, process, "Session: build shell (2)", "second daemon-owned PTY")

viewer_master, viewer_process = start_client("remote writer client")
send_and_expect(viewer_master, viewer_process, b"\x02t", "Terminal Sessions", "remote terminal route")
os.write(viewer_master, b"\x02n")
collect(viewer_master, .5)
os.write(viewer_master, b"\x02o")
collect(viewer_master, .5)
expect_eventually(viewer_master, viewer_process, "WRITER", "remote writer state")

os.write(master, b"\x02%")
collect(master, .5)
expect_eventually(master, process, "read-only", "split read-only state")
expect_eventually(master, process, "writer", "split writer state")
expect_eventually(master, process, "Viewers: 1", "split viewer accounting")

scrollback_command = b"seq 1 3000; sleep 0.2; printf 'needle\\n'\n"
os.write(viewer_master, b"\x1b[200~" + scrollback_command + b"\x1b[201~")
expect_eventually(viewer_master, viewer_process, "PASTE REVIEW", "terminal paste review")
os.write(viewer_master, b"\r")
expect_eventually(
    viewer_master,
    viewer_process,
    "dropped line-feeds ≥ 962",
    "bounded scrollback generator",
    60,
)
os.write(master, b"\x02n")
collect(master, .5)
search = send_and_expect(
    master,
    process,
    b"/needle",
    "visible match(es)",
    "terminal search match",
    seconds=4,
)
if "Dropped history: at least" not in search:
    search += expect_eventually(
        master,
        process,
        "Dropped history: at least",
        "terminal dropped-history accounting",
        8,
    )
if "needle" not in search or "visible match(es)" not in search:
    raise SystemExit(f"terminal search evidence is incomplete: {search[-4000:]}")
os.write(master, b"\x1b")
collect(master, .3)

help_text = send_and_expect(master, process, b"\x02?", "PREFIX HELP", "terminal prefix help")
for anchor in ["Ctrl+B t", "Ctrl+B %", "Ctrl+B [", "Ctrl+B K"]:
    if anchor not in help_text:
        raise SystemExit(f"terminal prefix help omitted {anchor!r}: {help_text[-4000:]}")
if evidence:
    os.makedirs(evidence, exist_ok=True)
    split_path = os.path.join(evidence, "terminal-sessions.txt")
    help_path = os.path.join(evidence, "terminal-prefix-help.txt")
    ansi_path = os.path.join(evidence, "terminal-sessions.ansi")
    cells_path = os.path.join(evidence, "terminal-sessions.cells")
    meta_path = os.path.join(evidence, "terminal-sessions.meta")
    report_path = os.path.join(evidence, "terminal-sessions.report.json")
    with open(split_path, "w", encoding="utf-8") as output:
        output.write(search.rstrip() + "\n")
    with open(help_path, "w", encoding="utf-8") as output:
        output.write(help_text.rstrip() + "\n")
    with open(ansi_path, "wb") as output:
        output.write(bytes(screens[master].transcript[-2_000_000:]))
    semantic_rows = search.splitlines()
    with open(cells_path, "w", encoding="utf-8") as output:
        output.write("YOCTUI_CELL_GOLDEN_V1 160 50\nSYMBOLS\n")
        for row_index in range(50):
            row = semantic_rows[row_index] if row_index < len(semantic_rows) else ""
            row = row[:160].ljust(160)
            encoded = "".join(
                f"{len(character.encode('utf-8'))}:{character}" for character in row
            )
            output.write(f"S|{encoded}\n")
        output.write("STYLES\nT|8000|fg=Reset;bg=Reset;ul=Reset;mod=NONE\n")
    with open(meta_path, "w", encoding="utf-8") as output:
        output.write("label=live\nscenario=terminal-sessions\nwidth=160\nheight=50\n")
    report = {
        "schema": 1,
        "scenario": "terminal-sessions",
        "interactions": [
            "attach two real clients to one daemon",
            "create two daemon-owned PTYs and transfer one writer lease",
            "split the first client into writer and read-only panes",
            "generate bounded scrollback, search for needle, and open Ctrl+B prefix help",
        ],
        "observed_assertions": [
            "Terminal Sessions",
            "writer",
            "read-only",
            "Dropped history: at least",
            "visible match(es)",
            "PREFIX HELP",
            "Ctrl+B K",
        ],
        "terminal": os.path.basename(ansi_path),
        "semantic": [os.path.basename(split_path), os.path.basename(help_path)],
    }
    with open(report_path, "w", encoding="utf-8") as output:
        json.dump(report, output, indent=2, sort_keys=True)
        output.write("\n")
    for path in (split_path, help_path, ansi_path, cells_path, meta_path, report_path):
        if not hashlib.sha256(open(path, "rb").read()).hexdigest():
            raise SystemExit(f"terminal evidence is missing: {path}")
os.write(master, b"\x1b")
collect(master, .3)

fcntl.ioctl(master, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 80, 0, 0))
screens[master].resize(24, 80)
collect(master, 1)
narrow = screens[master].text()
if "Terminal Sessions" not in narrow or "�" in narrow:
    raise SystemExit(f"terminal workbench resize lost identity: {narrow[-4000:]}")

os.write(viewer_master, b"\x02O")
collect(viewer_master, .5)
os.write(viewer_master, b"q")
expect_eventually(
    viewer_master,
    viewer_process,
    "Are you sure you want to exit yoctui?",
    "remote terminal exit confirmation",
)
os.write(viewer_master, b"y")
try:
    viewer_process.wait(timeout=3)
except subprocess.TimeoutExpired:
    viewer_process.kill()
    viewer_process.wait(timeout=2)
viewer_shutdown = collect(viewer_master, .2)
os.close(viewer_master)
if viewer_process.returncode != 0 or b"\x1b[?1049l" not in viewer_shutdown:
    raise SystemExit(f"remote terminal client exited with {viewer_process.returncode}")

os.write(master, b"q")
expect_eventually(
    master,
    process,
    "Are you sure you want to exit yoctui?",
    "terminal workbench exit confirmation",
)
os.write(master, b"y")
try:
    process.wait(timeout=3)
except subprocess.TimeoutExpired:
    process.kill()
    process.wait(timeout=2)
shutdown = collect(master, .2)
os.close(master)
if process.returncode != 0 or b"\x1b[?1049l" not in shutdown:
    raise SystemExit(f"terminal workbench PTY exited with {process.returncode}")

print("workbench terminal live split/writer/search/prefix-help matrix passed")
PY
