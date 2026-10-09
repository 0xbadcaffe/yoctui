#!/usr/bin/env python3

from __future__ import annotations

import contextlib
import importlib.util
import io
import unittest
import subprocess
import tempfile
from unittest.mock import patch
from pathlib import Path


SCRIPT = Path(__file__).with_name("check-version-bump.py")
SPEC = importlib.util.spec_from_file_location("check_version_bump", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
POLICY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(POLICY)


class VersionBumpPolicyTests(unittest.TestCase):
    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.git("init", "-q")
        self.git("config", "user.name", "Policy Test")
        self.git("config", "user.email", "policy@example.invalid")
        self.write("Cargo.toml", '[workspace]\n[workspace.package]\nversion = "0.1.1"\n')
        self.write("scripts/verify-cratesio-package.sh", 'version="0.1.1"\n')
        self.write("fuzz/Cargo.toml", "[dependencies]\n")
        self.write("crates/example/bridge/bridge.py", "def bridge():\n    return 1\n")
        self.write("crates/example/src/lib.rs", "fn original() {}\n")
        self.write("README.md", "# original documentation\n")
        self.commit()
        self.patcher = patch.object(POLICY, "ROOT", self.root)
        self.patcher.start()
        self.addCleanup(self.patcher.stop)

    def git(self, *args: str) -> None:
        subprocess.run(["git", *args], cwd=self.root, check=True, capture_output=True)

    def write(self, name: str, content: str) -> None:
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content)

    def commit(self) -> None:
        self.git("add", ".")
        self.git("commit", "-qm", "fixture")

    def check(self, failure: bool = False) -> None:
        with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            if failure:
                with self.assertRaises(SystemExit):
                    POLICY.main()
            else:
                POLICY.main()

    def test_clean_and_dirty_documentation_or_ci_changes_keep_version(self) -> None:
        self.write("README.md", "# documentation\n")
        self.check()
        self.commit()
        self.check()
        self.write("README.md", "# updated documentation\n")
        self.check()
        self.write(".github/workflows/ci.yml", "name: CI\n")
        self.commit()
        self.check()

    def test_product_change_requires_bump_clean_dirty_and_with_untracked_capture(self) -> None:
        self.write("crates/example/src/lib.rs", "fn changed() {}\n")
        self.check(failure=True)
        self.commit()
        self.check(failure=True)
        self.write("README.md", "# dirty documentation cannot mask product changes\n")
        self.check(failure=True)
        self.write("artifacts/unrelated.txt", "user capture\n")
        self.check(failure=True)

    def test_new_untracked_product_file_requires_bump(self) -> None:
        self.write("crates/example/src/added.rs", "fn added() {}\n")
        self.check(failure=True)

    def test_python_format_only_passes_but_changed_or_invalid_runtime_fails(self) -> None:
        self.write("crates/example/bridge/bridge.py", "# format only\ndef bridge( ):\n  return 1\n")
        self.check()
        self.commit()
        self.check()
        self.write("crates/example/bridge/bridge.py", "def bridge():\n    return 2\n")
        self.check(failure=True)
        self.write("crates/example/bridge/bridge.py", "invalid python ?\n")
        self.check(failure=True)

    def test_valid_product_bump_passes_and_downgrade_fails(self) -> None:
        self.write("crates/example/src/lib.rs", "fn changed() {}\n")
        self.write("Cargo.toml", '[workspace]\n[workspace.package]\nversion = "0.1.2"\n')
        self.write("scripts/verify-cratesio-package.sh", 'version="0.1.2"\n')
        self.check()
        self.commit()
        self.check()
        self.write("Cargo.toml", '[workspace]\n[workspace.package]\nversion = "0.1.1"\n')
        self.write("scripts/verify-cratesio-package.sh", 'version="0.1.1"\n')
        self.check(failure=True)

    def prepare_reference_include(self) -> None:
        self.write("docs/reference/old.md", "# Reference\n")
        self.write("crates/example/src/tests/reference.rs", 'const REFERENCE: &str =\n    include_str!("../../../../docs/reference/old.md");\n')
        self.commit()

    def rename_reference_include(self) -> None:
        (self.root / "docs/reference/old.md").rename(self.root / "docs/reference/new.md")
        self.write("crates/example/src/tests/reference.rs", 'const REFERENCE: &str =\n    include_str!("../../../../docs/reference/new.md");\n')

    def test_unchanged_test_reference_rename_keeps_version_dirty_and_committed(self) -> None:
        self.prepare_reference_include()
        self.rename_reference_include()
        self.check()
        self.commit()
        self.check()

    def test_test_reference_rename_with_changed_content_keeps_version(self) -> None:
        self.prepare_reference_include()
        self.rename_reference_include()
        self.write("docs/reference/new.md", "# Changed reference\n")
        self.check()

    def test_test_reference_rename_with_other_test_changes_keeps_version(self) -> None:
        self.prepare_reference_include()
        self.rename_reference_include()
        path = self.root / "crates/example/src/tests/reference.rs"
        path.write_text(path.read_text() + "fn changed_test() {}\n")
        self.check()

    def test_test_reference_rename_with_missing_target_keeps_version(self) -> None:
        self.prepare_reference_include()
        self.rename_reference_include()
        (self.root / "docs/reference/new.md").unlink()
        self.check()

    def test_test_only_fixture_correction_keeps_version_but_runtime_change_does_not(self) -> None:
        self.write("crates/example/src/tests/navigation.rs", "fn fixture() {}\n")
        self.write("crates/example/tests/acceptance.rs", "fn acceptance() {}\n")
        self.commit()
        self.check()
        self.write("crates/example/src/tests/navigation.rs", "fn corrected_fixture() {}\n")
        self.check()
        self.commit()
        self.check()
        self.write("crates/example/src/lib.rs", "fn changed_runtime() {}\n")
        self.check(failure=True)

    def test_nonproduct_commit_still_checks_internal_dependency_coherence(self) -> None:
        self.write("README.md", "# documentation\n")
        self.commit()
        self.write("crates/example/Cargo.toml", '[dependencies]\nyoctui-utils = { version = "0.1.0", path = "../yoctui-utils" }\n')
        self.commit()
        self.write("README.md", "# new documentation\n")
        self.commit()
        self.check(failure=True)

    def test_semantic_versions_compare_numerically(self) -> None:
        self.assertTrue(POLICY.version_increased((0, 2, 0), (0, 1, 99)))
        self.assertTrue(POLICY.version_increased((1, 0, 0), (0, 99, 99)))
        self.assertFalse(POLICY.version_increased((0, 1, 1), (0, 1, 1)))
        self.assertFalse(POLICY.version_increased((0, 1, 0), (0, 1, 1)))

    def test_workspace_version_parser_rejects_non_numeric_versions(self) -> None:
        valid = b'[workspace]\n[workspace.package]\nversion = "0.1.1"\n'
        self.assertEqual(POLICY.parse_version(valid, "fixture"), ("0.1.1", (0, 1, 1)))

        invalid = b'[workspace]\n[workspace.package]\nversion = "0.1.1-dev"\n'
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            POLICY.parse_version(invalid, "fixture")


if __name__ == "__main__":
    unittest.main()
