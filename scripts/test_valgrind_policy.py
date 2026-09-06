"""Negative controls for the explicitly approved Memcheck cache exception."""

import copy
import json
from pathlib import Path
import unittest
import xml.etree.ElementTree as ET

from scripts.check_valgrind import check_dependencies, check_growth, evaluate


def report() -> ET.Element:
    root = ET.fromstring(
        "<valgrindoutput><protocoltool>memcheck</protocoltool>"
        "<status><state>FINISHED</state></status></valgrindoutput>"
    )
    record = json.loads(
        Path("artifacts/performance/logger/memcheck-optimized.json").read_text()
    )
    for finding in record["captures"][0]["findings"]:
        error = ET.SubElement(root, "error")
        ET.SubElement(error, "kind").text = "Leak_PossiblyLost"
        what = ET.SubElement(error, "xwhat")
        ET.SubElement(what, "leakedbytes").text = str(finding["bytes"])
        ET.SubElement(what, "leakedblocks").text = str(finding["blocks"])
        stack = ET.SubElement(error, "stack")
        for symbol in finding["allocation_stack"]:
            ET.SubElement(ET.SubElement(stack, "frame"), "fn").text = symbol
    return root


def assess(root: ET.Element) -> dict:
    return evaluate(ET.tostring(root, encoding="unicode"))


class ValgrindPolicyTests(unittest.TestCase):
    def test_exact_reviewed_allocations_pass(self):
        result = assess(report())
        self.assertEqual(result["accepted_cache_bytes"], 39367)
        self.assertEqual(result["accepted_cache_blocks"], 10)
        check_growth(result, result)

    def test_dependencies_match_lock(self):
        check_dependencies(Path("Cargo.lock").read_text())

    def test_changed_dependency_rejected(self):
        lock = (
            Path("Cargo.lock")
            .read_text()
            .replace('version = "0.18.3"', 'version = "0.18.4"')
        )
        with self.assertRaises(ValueError):
            check_dependencies(lock)

    def test_unknown_stack_rejected_even_for_small_allocation(self):
        root = report()
        root.find("error").remove(root.find("error/stack"))
        with self.assertRaises(ValueError):
            assess(root)

    def test_growth_duplicates_bytes_and_blocks_rejected(self):
        for mutation in ("duplicate", "bytes", "blocks"):
            with self.subTest(mutation=mutation):
                root = report()
                if mutation == "duplicate":
                    root.append(copy.deepcopy(root.find("error")))
                else:
                    root.find(f"error/xwhat/leaked{mutation}").text = "2"
                with self.assertRaises(ValueError):
                    assess(root)

    def test_new_cache_after_warmup_rejected(self):
        root = report()
        root.remove(root.find("error"))
        with self.assertRaises(ValueError):
            check_growth(assess(root), assess(report()))

    def test_unrelated_errors_never_accepted(self):
        for kind in (
            "Leak_DefinitelyLost",
            "Leak_IndirectlyLost",
            "InvalidRead",
            "InvalidWrite",
            "UninitCondition",
            "UnknownKind",
            "FdNotClosed",
        ):
            with self.subTest(kind=kind):
                root = report()
                root.find("error/kind").text = kind
                with self.assertRaises(ValueError):
                    assess(root)

    def test_incomplete_or_wrong_tool_rejected(self):
        for path, text in (("status/state", "RUNNING"), ("protocoltool", "other")):
            root = report()
            root.find(path).text = text
            with self.assertRaises(ValueError):
                assess(root)
        with self.assertRaises(ET.ParseError):
            evaluate("<valgrindoutput>")

    def test_signal_and_invalid_counts_rejected(self):
        root = report()
        ET.SubElement(root, "fatal_signal")
        with self.assertRaises(ValueError):
            assess(root)
        for value in ("-1", "", "not a number"):
            root = report()
            root.find("error/xwhat/leakedbytes").text = value
            with self.assertRaises(ValueError):
                assess(root)

    def test_summary_cannot_hide_unattributed_loss(self):
        root = report()
        summary = ET.SubElement(root, "leak_summary")
        ET.SubElement(ET.SubElement(summary, "definitely_lost"), "bytes").text = "1"
        with self.assertRaises(ValueError):
            assess(root)


if __name__ == "__main__":
    unittest.main()
