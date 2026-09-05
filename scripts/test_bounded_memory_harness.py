import importlib.util
from pathlib import Path
import unittest


SCRIPT = Path(__file__).with_name("measure-bounded-memory.py")
SPEC = importlib.util.spec_from_file_location("measure_bounded_memory", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def sample(elapsed: float, daemon_rss: int, client_rss: int) -> dict[str, object]:
    return {
        "elapsed_seconds": elapsed,
        "processes": {
            "daemon": {"rss_bytes": daemon_rss, "threads": 3},
            "client": {"rss_bytes": client_rss, "threads": 1},
        },
    }


class BoundedMemoryHarnessTests(unittest.TestCase):
    def test_slope_uses_elapsed_minutes_and_role_samples(self) -> None:
        samples = [sample(0, 1_000, 2_000), sample(60, 66_536, 2_000)]
        self.assertAlmostEqual(
            MODULE.least_squares_slope_bytes_per_minute(samples, "daemon"),
            65_536,
        )
        self.assertEqual(
            MODULE.least_squares_slope_bytes_per_minute(samples, "client"), 0
        )

    def test_summary_uses_post_warmup_peak_and_final_window(self) -> None:
        samples = [
            sample(10, 1_000, 2_000),
            sample(70, 5_000, 2_000),
            sample(130, 3_000, 2_000),
        ]
        summary = MODULE.summarize(samples, "daemon", 1)
        self.assertEqual(summary["rss_growth_bytes"], 4_000)
        self.assertEqual(summary["rss_final_bytes"], 3_000)
        self.assertEqual(summary["threads_max"], 3)


if __name__ == "__main__":
    unittest.main()
