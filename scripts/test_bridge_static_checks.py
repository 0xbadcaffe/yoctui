import ast
import copy
import importlib.util
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("bridge_static", ROOT / "scripts/check-bridge-python.py")
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class BridgeStaticChecksTests(unittest.TestCase):
    def test_omitting_a_loader_fragment_is_rejected(self):
        original = Path.read_text
        with patch.object(Path, "read_text", autospec=True) as reader:
            reader.side_effect = lambda path: original(path).replace('    "entrypoint.py",\n', "") if path == MODULE.BRIDGE else original(path)
            with self.assertRaisesRegex(ValueError, "every loader component"):
                MODULE.assemble()

    def test_projection_includes_every_body_in_exact_loader_order(self):
        source, paths = MODULE.assemble()
        expected = [ast.dump(node) for path in paths for node in ast.parse(path.read_text()).body]
        self.assertEqual([ast.dump(node) for node in ast.parse(source).body], expected)
        self.assertEqual(len(paths), 11)

    def test_type_projection_keeps_bound_signatures_and_bodies_exact(self):
        source, _ = MODULE.assemble()
        original = ast.parse(source)
        functions = {node.name: node for node in original.body if isinstance(node, ast.FunctionDef)}
        projected = ast.parse(MODULE.type_projection(source))
        classes = {node.name: node for node in projected.body if isinstance(node, ast.ClassDef)}
        checked = 0
        for node in original.body:
            if isinstance(node, ast.Assign) and isinstance(node.targets[0], ast.Attribute):
                target = node.targets[0]
                if isinstance(target.value, ast.Name) and target.value.id in classes:
                    expected = copy.deepcopy(functions[node.value.id])
                    expected.name = target.attr
                    actual = next(item for item in classes[target.value.id].body if isinstance(item, ast.FunctionDef) and item.name == target.attr)
                    self.assertEqual(ast.dump(actual), ast.dump(expected))
                    checked += 1
        self.assertEqual(checked, 12)
        self.assertEqual(MODULE.type_projection(source).count("# type: ignore[import-not-found]"), source.count("# type: ignore[import-not-found]"))

    def test_unsupported_dynamic_binding_fails_closed(self):
        with self.assertRaisesRegex(ValueError, "Unsupported"):
            MODULE.type_projection("class Example:\n    pass\nExample.method = missing\n")

    def test_undefined_names_are_still_rejected_by_real_ruff(self):
        source, _ = MODULE.assemble()
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "broken.py"
            path.write_text(source + "\nmissing_ci_dependency()\n")
            result = subprocess.run(["ruff", "check", "--config", str(ROOT / "pyproject.toml"), str(path)], capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("F821", result.stdout)
            self.assertIn("missing_ci_dependency", result.stdout)


if __name__ == "__main__":
    unittest.main()
