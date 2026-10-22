"""Unit tests for the bounded real-frame README animation tooling."""

import importlib.util
import os
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
from types import SimpleNamespace


sys.path.insert(0, str(Path(__file__).parent))
SPEC = importlib.util.spec_from_file_location(
    "record_readme_demo", Path(__file__).with_name("record-readme-demo.py")
)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class FrameSelectionTests(unittest.TestCase):
    def test_command_input_drains_terminal_and_keeps_queued_command_order(self):
        reader, writer = os.pipe()
        try:
            os.write(writer, b'{"chapter":"tasks"}\n{"seconds":1}\n')
            os.close(writer)
            writer = None
            with patch.object(MODULE.Recorder, "collect") as collect:
                recorder = MODULE.Recorder(None, Path("/unused"), 2)
                commands = list(MODULE.input_commands(recorder, reader))
            self.assertEqual(commands, [{"chapter": "tasks"}, {"seconds": 1}])
            self.assertGreaterEqual(collect.call_count, 2)
        finally:
            os.close(reader)
            if writer is not None:
                os.close(writer)

    def frames(self, count):
        return [
            {"chapter": chapter, "file": f"{chapter}-{index}"}
            for chapter in ("dashboard", "tasks", "composition", "files")
            for index in range(count)
        ]

    def test_empty_capture_is_rejected(self):
        with self.assertRaises(ValueError):
            MODULE.select_frames([])

    def test_quiet_scenes_remain_readable_despite_fewer_changed_frames(self):
        frames = (
            [{"chapter": "dashboard"}] * 12
            + [{"chapter": "tasks"}] * 24
            + [{"chapter": "composition"}] * 3
            + [{"chapter": "files"}] * 3
        )
        durations = MODULE.frame_durations(frames, 30)
        self.assertEqual(len(durations), len(frames))
        self.assertTrue(all(value >= 20 and value % 10 == 0 for value in durations))
        for start, end in ((0, 12), (12, 36), (36, 39), (39, 42)):
            self.assertAlmostEqual(sum(durations[start:end]), 8500, delta=100)

    def test_timing_rejects_empty_frames(self):
        with self.assertRaises(ValueError):
            MODULE.frame_durations([], 30)

    def test_missing_live_chapter_is_rejected(self):
        with self.assertRaisesRegex(ValueError, "composition"):
            MODULE.select_frames([{"chapter": "dashboard"}, {"chapter": "tasks"}])

    def test_selection_preserves_each_chapter_and_its_endpoints(self):
        selected = MODULE.select_frames(self.frames(100), 4)
        self.assertEqual(len(selected), 16)
        for chapter in MODULE.REQUIRED_CHAPTERS:
            files = [frame["file"] for frame in selected if frame["chapter"] == chapter]
            self.assertEqual(files[0], f"{chapter}-0")
            self.assertEqual(files[-1], f"{chapter}-99")

    def test_small_capture_is_not_duplicated_or_interpolated(self):
        frames = self.frames(2)
        self.assertEqual(MODULE.select_frames(frames, 24), frames)

    def test_invalid_per_chapter_limit_is_rejected(self):
        with self.assertRaises(ValueError):
            MODULE.select_frames(self.frames(2), 1)

    def test_trimming_uses_only_original_frames_and_still_requires_every_scene(self):
        frames = self.frames(4)
        selected = MODULE.select_frames(frames, 4, 2)
        self.assertEqual(selected, frames[2:])
        with self.assertRaisesRegex(ValueError, "dashboard"):
            MODULE.select_frames(frames, 4, 4)

    def test_trimming_outside_the_capture_is_rejected(self):
        for offset in (-1, 8):
            with self.assertRaisesRegex(ValueError, "outside"):
                MODULE.select_frames(self.frames(2), 4, offset)

    def test_revisited_chapters_keep_their_original_order(self):
        frames = self.frames(3) + [{"chapter": "dashboard", "file": "final-dashboard"}]
        selected = MODULE.select_frames(frames, 2)
        self.assertEqual(selected[-1]["file"], "final-dashboard")
        self.assertEqual(
            [frame["file"] for frame in selected[:2]], ["dashboard-0", "dashboard-2"]
        )

    def test_failed_current_screen_assertion_stops_the_command(self):
        recorder = MODULE.Recorder(None, Path("/unused"), 2)
        with patch.object(
            recorder, "collect", side_effect=AssertionError("missing live frame")
        ):
            with self.assertRaisesRegex(AssertionError, "missing live frame"):
                recorder.command({"wait": "Tasks", "seconds": 10})

    def test_digest_records_exact_frame_bytes(self):
        self.assertNotEqual(MODULE.digest(b"1%"), MODULE.digest(b"2%"))
        self.assertEqual(MODULE.digest(b"1%"), MODULE.digest(b"1%"))

    def test_binary_provenance_hashes_the_actual_file_without_assuming_its_revision(
        self,
    ):
        with tempfile.TemporaryDirectory() as directory:
            executable = Path(directory) / "binary"
            executable.write_bytes(b"actual executable bytes")
            result = MODULE.binary_provenance(str(executable), None)
            self.assertEqual(result["binary_path"], str(executable.resolve()))
            self.assertEqual(
                result["binary_sha256"], MODULE.digest(executable.read_bytes())
            )
            self.assertNotIn("binary_source_commit", result)

    def test_unchanged_styled_frames_are_not_duplicated(self):
        screen = SimpleNamespace(
            cell_golden=lambda: "actual cells", text=lambda: "actual text"
        )
        with tempfile.TemporaryDirectory() as directory:
            recorder = MODULE.Recorder(
                SimpleNamespace(screen=screen), Path(directory), 2
            )
            recorder.chapter = "tasks"
            recorder.next_frame = 0
            recorder.sample()
            recorder.next_frame = 0
            recorder.sample()
            self.assertEqual(len(recorder.frames), 1)
            self.assertEqual(
                (Path(directory) / recorder.frames[0]["file"]).read_text(),
                "actual cells",
            )

    def test_recording_stops_at_the_frame_bound(self):
        screen = SimpleNamespace(cell_golden=lambda: "new cells")
        recorder = MODULE.Recorder(SimpleNamespace(screen=screen), Path("/unused"), 2)
        recorder.chapter, recorder.next_frame = "tasks", 0
        recorder.frames = [None] * MODULE.MAX_FRAMES
        with self.assertRaisesRegex(AssertionError, "exceeded"):
            recorder.sample()

    @unittest.skipUnless(
        importlib.util.find_spec("PIL"), "Pillow is only needed for GIF encoding"
    )
    def test_gif_keeps_loop_duration_and_real_changed_pixels(self):
        from PIL import Image

        images = [
            Image.new("RGB", (16, 16), color) for color in ("red", "green", "blue")
        ]
        with tempfile.TemporaryDirectory() as directory:
            destination = Path(directory) / "demo.gif"
            MODULE.encode(images, destination, [200, 300, 400])
            with Image.open(destination) as gif:
                self.assertTrue(gif.is_animated)
                self.assertEqual(gif.n_frames, 3)
                self.assertEqual(gif.info["loop"], 0)
                durations, pixels = [], []
                for index in range(gif.n_frames):
                    gif.seek(index)
                    durations.append(gif.info["duration"])
                    pixels.append(gif.convert("RGB").getpixel((0, 0)))
                self.assertEqual(durations, [200, 300, 400])
                self.assertEqual(len(set(pixels)), 3)


if __name__ == "__main__":
    unittest.main()
