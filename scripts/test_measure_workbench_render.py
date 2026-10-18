"""Regression checks for renderer comparison completion and checksum guards."""

from pathlib import Path
import runpy
import subprocess
import unittest
from unittest.mock import patch
from unittest.mock import Mock

MODULE = runpy.run_path(str(Path(__file__).with_name("measure-workbench-render.py")))


class RendererMeasurementTests(unittest.TestCase):
    def test_paired_order_medians_and_checksums(self):
        baseline, candidate = Path("baseline"), Path("candidate")
        values = [("same", value) for value in (1, 1, 100, 80, 82, 110, 105, 79)]
        measure = Mock(side_effect=values)
        with patch.dict(MODULE["compare"].__globals__, measure=measure):
            result = MODULE["compare"]([baseline, candidate], "idle", 1000, 3)
        self.assertEqual(result["baseline_ns_per_frame"], [100, 110, 105])
        self.assertEqual(result["candidate_ns_per_frame"], [80, 82, 79])
        self.assertEqual(result["baseline_median_ns"], 105)
        self.assertEqual(result["candidate_median_ns"], 80)
        self.assertEqual(
            [call.args[0] for call in measure.call_args_list],
            [
                baseline,
                candidate,
                baseline,
                candidate,
                candidate,
                baseline,
                baseline,
                candidate,
            ],
        )

    def test_rejects_unstable_output_including_baseline_alone(self):
        for values in [
            [("same", 1)] * 2 + [("old", 100), ("new", 80)] * 3,
            [("same", 1)] * 2 + [("same", 100)] * 3 + [("changed", 80)] * 3,
        ]:
            with (
                self.subTest(values=values),
                patch.dict(
                    MODULE["compare"].__globals__, measure=Mock(side_effect=values)
                ),
            ):
                with self.assertRaisesRegex(ValueError, "changed or was unstable"):
                    MODULE["compare"]([Path("a"), Path("b")], "idle", 1000, 3)

    def test_accepts_only_matching_completed_scenario_and_frame_count(self):
        output = "yoctui ui performance: scenario=idle frames=128 checksum=0123456789abcdef elapsed_ms=42 ns_per_frame=328125\n"
        with patch(
            "subprocess.run", return_value=subprocess.CompletedProcess([], 0, output)
        ) as run:
            self.assertEqual(
                MODULE["measure"](Path("/test/bench"), "idle", 128),
                ("0123456789abcdef", 328125),
            )
            environment = run.call_args.kwargs["env"]
            self.assertEqual(environment["YOCTUI_PROFILE_FRAMES"], "128")
            self.assertEqual(environment["YOCTUI_PROFILE_SCENARIO"], "idle")

    def test_rejects_missing_partial_wrong_or_zero_completion(self):
        valid = "yoctui ui performance: scenario=idle frames=128 checksum=0123456789abcdef elapsed_ms=42 ns_per_frame=328125\n"
        for output in (
            "",
            valid.replace("idle", "telemetry"),
            valid.replace("frames=128", "frames=129"),
            valid.replace("elapsed_ms=42", "elapsed_ms=0"),
            valid.replace("ns_per_frame=328125", "ns_per_frame=0"),
        ):
            with (
                self.subTest(output=output),
                patch(
                    "subprocess.run",
                    return_value=subprocess.CompletedProcess([], 0, output),
                ),
            ):
                with self.assertRaises(ValueError):
                    MODULE["measure"](Path("/test/bench"), "idle", 128)


if __name__ == "__main__":
    unittest.main()
