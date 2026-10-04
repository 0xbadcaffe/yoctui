"""Vendored BitBake imports stay inside the bridge subprocess."""

import json
import os
from pathlib import Path
import tempfile
import unittest

from .support import run_bridge


QUERY = b'{"protocol_version":1,"sequence":1,"message":{"type":"get_variable","name":"MACHINE","recipe":null}}'
METADATA_QUERY = b'{"protocol_version":1,"sequence":1,"message":{"type":"get_recipe_metadata","recipe":"virtual/kernel"}}'
TINFOIL = """class Data:
 def getVar(self, name, expand=True): return VALUE if name == "MACHINE" else None
class Tinfoil:
 def __init__(self, **kwargs): self.config_data = Data()
 def prepare(self, **kwargs): pass
 def shutdown(self): pass
"""


class TinfoilImportTests(unittest.TestCase):
    def package(self, directory, import_line):
        package = Path(directory, "bb")
        package.mkdir()
        Path(package, "__init__.py").write_text('__version__ = "2.19.1"\n')
        Path(package, "tinfoil.py").write_text(import_line + "\n" + TINFOIL)
        return package

    def query(self, directory, request=QUERY):
        result = run_bridge(request, environment={"PYTHONPATH": directory})
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        return json.loads(result.stdout)["message"]

    def test_packaged_vendored_parser_imports_without_parent_path_override(self):
        before = os.environ.copy()
        with tempfile.TemporaryDirectory() as directory:
            package = self.package(directory, "from pysh.pyshtables import VALUE")
            vendor = package / "pysh"
            vendor.mkdir()
            (vendor / "__init__.py").write_text("")
            (vendor / "pyshtables.py").write_text('VALUE = "romulus"\n')
            message = self.query(directory)
        self.assertEqual(message["type"], "variable")
        self.assertEqual(message["value"], "romulus")
        self.assertEqual(os.environ, before)

    def test_package_without_vendor_retains_normal_tinfoil_behavior(self):
        with tempfile.TemporaryDirectory() as directory:
            self.package(directory, 'VALUE = "qemuarm"')
            message = self.query(directory)
        self.assertEqual(message["type"], "variable")
        self.assertEqual(message["value"], "qemuarm")

    def test_existing_vendor_parent_is_not_duplicated(self):
        with tempfile.TemporaryDirectory() as directory:
            package = self.package(
                directory,
                """import bb, os, sys
assert sys.path.count(os.path.dirname(bb.__file__)) == 1
from pysh.pyshtables import VALUE""",
            )
            vendor = package / "pysh"
            vendor.mkdir()
            (vendor / "__init__.py").write_text("")
            (vendor / "pyshtables.py").write_text('VALUE = "romulus"\n')
            result = run_bridge(
                QUERY,
                environment={"PYTHONPATH": os.pathsep.join((directory, str(package)))},
            )
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        message = json.loads(result.stdout)["message"]
        self.assertEqual(message["type"], "variable")
        self.assertEqual(message["value"], "romulus")

    def test_missing_vendored_package_does_not_fabricate_success(self):
        with tempfile.TemporaryDirectory() as directory:
            self.package(directory, "from pysh.pyshtables import VALUE")
            message = self.query(directory, METADATA_QUERY)
        self.assertEqual(message["type"], "command_failed")
        self.assertEqual(message["code"], "bitbake_server_unavailable")

    def test_other_import_failure_is_not_masked_by_existing_vendor(self):
        with tempfile.TemporaryDirectory() as directory:
            package = self.package(directory, "import missing_yoctui_test_dependency")
            (package / "pysh").mkdir()
            message = self.query(directory, METADATA_QUERY)
        self.assertEqual(message["type"], "command_failed")
        self.assertEqual(message["code"], "bitbake_server_unavailable")

    def test_single_module_legacy_server_connector_is_unchanged(self):
        with tempfile.TemporaryDirectory() as directory:
            Path(directory, "bb.py").write_text("""__version__ = "2.8.1"
class Connection:
 def get_variable(self, name, recipe): return {"value": "legacy", "provenance": None}
class Server:
 def connect(self): return Connection()
server = Server()
""")
            message = self.query(directory)
        self.assertEqual(message["type"], "variable")
        self.assertEqual(message["value"], "legacy")
