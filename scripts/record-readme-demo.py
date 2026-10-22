#!/usr/bin/env python3
"""Record styled native PTY frames and encode a bounded, time-compressed README GIF.

Capture accepts JSON lines on stdin: keys/wait/absent/timeout, chapter (empty
pauses recording), seconds, and finish. Failed assertions stop the recording;
they never send the next queued action. Only the client is closed, not its daemon.
Install Pillow in a virtualenv for encoding; capture uses the standard library.
"""

from __future__ import annotations

import argparse
import fcntl
import hashlib
import itertools
import json
import os
import pty
import runpy
import select
import shutil
import struct
import subprocess
import sys
import tempfile
import termios
import time
from datetime import datetime, timezone
from pathlib import Path

from pty_acceptance import TerminalAcceptance, become_session_leader


ROOT = Path(__file__).resolve().parents[1]
WIDTH, HEIGHT = 160, 50
MAX_FRAMES = 1200
MAX_RAW_BYTES = 32 * 1024 * 1024
REQUIRED_CHAPTERS = {"dashboard", "tasks", "composition", "files"}


def input_commands(recorder: Recorder, descriptor: int):
    """Keep draining the PTY while the operator decides the next action."""
    pending = b""
    while True:
        recorder.collect(0.05)
        if not select.select([descriptor], [], [], 0)[0]:
            continue
        chunk = os.read(descriptor, 65536)
        if not chunk:
            if pending.strip():
                yield json.loads(pending)
            return
        pending += chunk
        if len(pending) > 65536:
            raise ValueError("Capture command exceeded 64 KiB")
        while b"\n" in pending:
            line, pending = pending.split(b"\n", 1)
            if line.strip():
                yield json.loads(line)


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def binary_provenance(binary: str, source_commit: str | None) -> dict:
    executable = Path(shutil.which(binary) or binary).resolve(strict=True)
    with executable.open("rb") as stream:
        sha256 = hashlib.file_digest(stream, "sha256").hexdigest()
    result = {"binary_path": str(executable), "binary_sha256": sha256}
    if source_commit:
        result["binary_source_commit"] = subprocess.check_output(
            ["git", "rev-parse", "--verify", source_commit + "^{commit}"],
            cwd=ROOT,
            text=True,
        ).strip()
    return result


def select_frames(
    frames: list[dict], per_chapter: int = 24, start_frame: int = 0
) -> list[dict]:
    """Preserve chronology and chapter boundaries; never invent intermediate frames."""
    if not frames or per_chapter < 2:
        raise ValueError("Need frames and at least two samples per chapter")
    if start_frame < 0 or start_frame >= len(frames):
        raise ValueError("Start frame is outside the real capture")
    frames = frames[start_frame:]
    chapters = {frame["chapter"] for frame in frames}
    if missing := REQUIRED_CHAPTERS - chapters:
        raise ValueError(f"Missing live chapters: {sorted(missing)}")
    selected: list[dict] = []
    start = 0
    while start < len(frames):
        end = start + 1
        while end < len(frames) and frames[end]["chapter"] == frames[start]["chapter"]:
            end += 1
        count = min(per_chapter, end - start)
        indexes = sorted(
            {round(i * (end - start - 1) / max(1, count - 1)) for i in range(count)}
        )
        selected.extend(frames[start + index] for index in indexes)
        start = end
    return selected


def frame_durations(frames: list[dict], seconds: float) -> list[int]:
    """Give quiet scenes the same reading time as busy scenes, in GIF centiseconds."""
    groups = [
        list(group) for _, group in itertools.groupby(frames, lambda f: f["chapter"])
    ]
    if not groups:
        raise ValueError("Need real frames to calculate animation timing")
    durations = []
    for group in groups:
        interval = max(20, round(seconds * 1000 / len(groups) / len(group) / 10) * 10)
        durations.extend([interval] * len(group))
        durations[-1] += 1000
    return durations


class Recorder:
    def __init__(self, terminal: TerminalAcceptance, output: Path, fps: float) -> None:
        self.terminal, self.output = terminal, output
        self.interval = 1 / fps
        self.started = time.monotonic()
        self.next_frame = self.started
        self.chapter = ""
        self.frames: list[dict] = []
        self.previous = None
        self.dropped_raw_bytes = 0

    def sample(self) -> None:
        now = time.monotonic()
        if not self.chapter or now < self.next_frame:
            return
        self.next_frame = now + self.interval
        cells = self.terminal.screen.cell_golden().encode()
        identity = (self.chapter, digest(cells))
        if identity == self.previous:
            return
        if len(self.frames) >= MAX_FRAMES:
            raise AssertionError(f"Recording exceeded {MAX_FRAMES} frames")
        self.previous = identity
        filename = f"frame-{len(self.frames):04d}.cells"
        (self.output / filename).write_bytes(cells)
        (self.output / filename).with_suffix(".txt").write_text(
            self.terminal.screen.text()
        )
        self.frames.append(
            {
                "file": filename,
                "chapter": self.chapter,
                "seconds": round(now - self.started, 3),
                "sha256": identity[1],
            }
        )

    def collect(
        self, seconds: float, expected: str | None = None, absent: str | None = None
    ) -> None:
        deadline = time.monotonic() + seconds
        while time.monotonic() < deadline:
            received = self.terminal.read(timeout=0.025)
            overflow = len(self.terminal.raw) - MAX_RAW_BYTES
            if overflow > 0:
                del self.terminal.raw[:overflow]
                self.dropped_raw_bytes += overflow
            self.sample()
            text = self.terminal.screen.text()
            if (
                expected is not None
                and not received
                and expected in text
                and (absent is None or absent not in text)
            ):
                return
            if self.terminal.process.poll() is not None:
                raise AssertionError("Yoctui exited during recording")
        if expected is not None:
            raise AssertionError(
                f"Missing {expected!r} in current frame:\n{self.terminal.screen.text()}"
            )

    def command(self, command: dict) -> bool:
        if "chapter" in command:
            self.chapter = command["chapter"]
            self.next_frame = 0
            self.sample()
        if "keys" in command:
            os.write(self.terminal.master, command["keys"].encode())
        if "wait" in command:
            self.collect(
                command.get("timeout", 20), command["wait"], command.get("absent")
            )
        if "seconds" in command:
            self.collect(command["seconds"])
        if "save" in command:
            (self.output / (command["save"] + ".txt")).write_text(
                self.terminal.screen.text()
            )
        if command.get("finish"):
            self.chapter = ""
            os.write(self.terminal.master, b"q")
            deadline = time.monotonic() + 10
            confirmed = False
            while self.terminal.process.poll() is None and time.monotonic() < deadline:
                self.terminal.read()
                if not confirmed and "Confirm exit" in self.terminal.screen.text():
                    os.write(self.terminal.master, b"y")
                    confirmed = True
            if self.terminal.process.poll() != 0:
                raise AssertionError(
                    "Client did not exit cleanly; daemon-owned work is preserved"
                )
            return False
        return True


def capture(args: argparse.Namespace) -> None:
    output = args.capture.resolve()
    output.mkdir(parents=True, exist_ok=False)
    version = subprocess.check_output([args.binary, "--version"], text=True).strip()
    provenance = binary_provenance(args.binary, args.binary_source_commit)
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", HEIGHT, WIDTH, 0, 0))
    with tempfile.TemporaryDirectory(prefix="yoctui-readme-client-") as private:
        environment = dict(
            os.environ,
            TERM="xterm-256color",
            YOCTUI_TERMINAL_GRAPHICS="none",
            XDG_CONFIG_HOME=private + "/config",
            XDG_STATE_HOME=private + "/state",
        )
        process = subprocess.Popen(
            [args.binary, "attach"],
            stdin=slave,
            stdout=slave,
            stderr=slave,
            env=environment,
            preexec_fn=become_session_leader,
        )
        os.close(slave)
        terminal = TerminalAcceptance(master, process, bytearray(), WIDTH, HEIGHT)
        recorder = Recorder(terminal, output, args.fps)
        metadata = {
            "schema_version": 1,
            "authority": "native optimized Yoctui attached to a real daemon",
            "captured_utc": datetime.now(timezone.utc).isoformat(),
            "binary": version,
            "capture_source_commit": subprocess.check_output(
                ["git", "rev-parse", "HEAD"], cwd=ROOT, text=True
            ).strip(),
            "width": WIDTH,
            "height": HEIGHT,
            "image": args.image,
            "frames": recorder.frames,
            **provenance,
        }
        try:
            recorder.collect(30, "F12")
            print("ATTACHED", flush=True)
            for command in input_commands(recorder, sys.stdin.fileno()):
                if not recorder.command(command):
                    break
                print(
                    f"OK {len(recorder.frames)} frames · {recorder.chapter or 'paused'}",
                    flush=True,
                )
                print(terminal.screen.text(), flush=True)
        finally:
            metadata["dropped_raw_bytes"] = recorder.dropped_raw_bytes
            (output / "capture.json").write_text(json.dumps(metadata, indent=2) + "\n")
            (output / "capture.ansi").write_bytes(terminal.raw)
            (output / "last-screen.txt").write_text(terminal.screen.text())
            terminal.cleanup()
            os.close(master)


def encode(images: list, destination: Path, durations: list[int]) -> None:
    from PIL import Image

    # One palette across scenes avoids flicker; GIF stores changed rectangles.
    atlas = Image.new("RGB", (320, 200 * len(images)))
    for index, image in enumerate(images):
        thumbnail = image.copy()
        thumbnail.thumbnail((320, 200))
        atlas.paste(thumbnail, (0, index * 200))
    palette = atlas.quantize(colors=256, method=Image.Quantize.MEDIANCUT)
    frames = [
        image.quantize(palette=palette, dither=Image.Dither.NONE) for image in images
    ]
    destination.parent.mkdir(parents=True, exist_ok=True)
    frames[0].save(
        destination,
        save_all=True,
        append_images=frames[1:],
        duration=durations,
        loop=0,
        optimize=True,
        disposal=1,
    )


def render(args: argparse.Namespace) -> None:
    from PIL import Image

    source = args.render.resolve()
    metadata = json.loads(source.read_text())
    frames = select_frames(metadata["frames"], args.per_chapter, args.start_frame)
    raster = runpy.run_path(str(ROOT / "scripts/render-m22-concept-screenshots.py"))
    raster["require_renderer"]()
    images = []
    with tempfile.TemporaryDirectory(prefix="yoctui-demo-raster-") as temporary:
        for index, frame in enumerate(frames):
            cells = source.parent / frame["file"]
            if digest(cells.read_bytes()) != frame["sha256"]:
                raise ValueError(f"Changed source frame: {cells}")
            png = Path(temporary) / f"{index:04d}.png"
            raster["render_cell_golden"](cells, png)
            with Image.open(png) as image:
                images.append(image.convert("RGB"))
        durations = frame_durations(frames, args.seconds)
        encode(images, args.output, durations)
    with Image.open(args.output) as gif:
        if gif.n_frames < 8 or not gif.is_animated:
            raise ValueError("Expected a genuinely animated workflow")
        report = {key: value for key, value in metadata.items() if key != "frames"}
        display_path = (
            args.output.resolve().relative_to(ROOT)
            if args.output.resolve().is_relative_to(ROOT)
            else args.output
        )
        report.update(
            {
                "file": str(display_path),
                "gif_sha256": digest(args.output.read_bytes()),
                "gif_bytes": args.output.stat().st_size,
                "frame_count": gif.n_frames,
                "time_compressed": True,
                "capture_manifest_sha256": digest(source.read_bytes()),
                "chapters": list(dict.fromkeys(frame["chapter"] for frame in frames)),
                "source_frames": frames,
                "source_frame_offset": args.start_frame,
            }
        )
    args.output.with_suffix(".json").write_text(json.dumps(report, indent=2) + "\n")
    print(
        f"GIF: {gif.n_frames} frames, {args.output.stat().st_size:,} bytes · {args.output}"
    )


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument(
        "--capture",
        type=Path,
        help="New directory for real PTY frames; reads JSON actions on stdin",
    )
    mode.add_argument("--render", type=Path, help="Captured capture.json to encode")
    parser.add_argument("--binary", default="yoctui")
    parser.add_argument(
        "--binary-source-commit",
        help="Known source revision of the binary; distinct from the capture checkout",
    )
    parser.add_argument("--image", default="core-image-minimal")
    parser.add_argument("--fps", type=float, default=2)
    parser.add_argument(
        "--output", type=Path, default=ROOT / "docs/media/yoctui-poky-workflow.gif"
    )
    parser.add_argument("--seconds", type=float, default=30)
    parser.add_argument("--per-chapter", type=int, default=24)
    parser.add_argument(
        "--start-frame",
        type=int,
        default=0,
        help="Omit real preparation frames before this index",
    )
    args = parser.parse_args()
    if (
        not 0 < args.fps <= 10
        or not 5 <= args.seconds <= 120
        or not 2 <= args.per_chapter <= 100
    ):
        parser.error(
            "Use fps (0,10], duration 5–120 seconds and 2–100 frames per chapter"
        )
    capture(args) if args.capture else render(args)


if __name__ == "__main__":
    main()
