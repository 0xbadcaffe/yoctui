from importlib.util import module_from_spec, spec_from_file_location
from pathlib import Path
import tempfile
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / "check-library-layout.py"
SPEC = spec_from_file_location("check_library_layout", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
LAYOUT = module_from_spec(SPEC)
SPEC.loader.exec_module(LAYOUT)


class SourceLayoutTests(unittest.TestCase):
    def test_accepts_bounded_sources_and_test_bodies_in_test_folders(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "crates/example/src/lib.rs"
            test = root / "crates/example/src/tests/behavior.rs"
            source.parent.mkdir(parents=True)
            test.parent.mkdir(parents=True)
            source.write_text("pub fn value() -> u8 { 1 }\n", encoding="utf-8")
            test.write_text("#[test]\nfn value_is_one() {}\n", encoding="utf-8")

            self.assertEqual(
                LAYOUT.source_layout_errors(root, [source, test]),
                [],
            )

    def test_rejects_oversized_sources_and_inline_test_bodies(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            oversized = root / "scripts/oversized.py"
            inline = root / "crates/example/src/lib.rs"
            oversized.parent.mkdir(parents=True)
            inline.parent.mkdir(parents=True)
            oversized.write_text("line\n" * 501, encoding="utf-8")
            inline.write_text(
                "#[cfg(test)]\nmod tests {\n    #[test]\n    fn inline() {}\n}\n",
                encoding="utf-8",
            )

            errors = LAYOUT.source_layout_errors(root, [inline, oversized])

            self.assertTrue(any("source limit is 500" in error for error in errors))
            self.assertTrue(any("inline Rust test module" in error for error in errors))
            self.assertTrue(any("outside a tests folder" in error for error in errors))


if __name__ == "__main__":
    unittest.main()
