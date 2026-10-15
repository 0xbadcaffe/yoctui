"""Single-recipe metadata remains authoritative without parsing unrelated recipes."""

from .support import (
    BRIDGE,
    Path,
    SimpleNamespace,
    importlib,
    os,
    patch,
    tempfile,
    unittest,
)


class FastRecipeInspectionTests(unittest.TestCase):
    def setUp(self):
        spec = importlib.util.spec_from_file_location("fast_inspection_bridge", BRIDGE)
        self.bridge = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.bridge)
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.recipe = self.root / "u-boot_2026.01.bb"
        self.recipe.write_text('PN = "u-boot"\n')
        self.config = {
            "BBFILES": str(self.root / "*.bb"),
            "PREFERRED_PROVIDER_virtual/bootloader": "u-boot",
        }
        self.data = {
            "PN": "u-boot",
            "PROVIDES": "virtual/bootloader",
            "S": "/work/u-boot/git",
            "OVERRIDES": "machine:local",
        }
        self.parsed = []
        self.connection = self.bridge.TinfoilConnection.__new__(
            self.bridge.TinfoilConnection
        )
        self.connection.recipe_files = {}
        self.connection.inspection_datastores = {}
        self.connection.tinfoil = SimpleNamespace(
            config_data=SimpleNamespace(getVar=lambda key, *args: self.config.get(key)),
            parse_recipe_file=self.parse,
        )
        self.patch = patch.dict(
            os.environ, {"YOCTUI_BRIDGE_FAST_RECIPE_INSPECTION": "1"}
        )
        self.patch.start()
        self.addCleanup(self.patch.stop)

    def parse(self, path):
        self.parsed.append(path)
        return SimpleNamespace(getVar=lambda key, *args: self.data.get(key))

    def test_unique_provider_is_parsed_once_for_repeated_variable_queries(self):
        first = self.connection._fast_recipe_data("virtual/bootloader")
        self.assertEqual(first.getVar("S"), "/work/u-boot/git")
        self.assertIs(first, self.connection._fast_recipe_data("virtual/bootloader"))
        self.assertEqual(self.parsed, [str(self.recipe)])
        self.assertEqual(
            self.connection.recipe_files["virtual/bootloader"], str(self.recipe)
        )

    def test_ambiguous_provider_defers_to_normal_bitbake_provider_resolution(self):
        (self.root / "u-boot_2025.01.bb").write_text("")
        self.assertIsNone(self.connection._fast_recipe_data("u-boot"))
        self.assertEqual(self.parsed, [])

    def test_masked_recipe_is_not_selected(self):
        self.config["BBMASK"] = "u-boot_2026"
        self.assertIsNone(self.connection._fast_recipe_data("u-boot"))
        self.assertEqual(self.parsed, [])

    def test_multiconfig_retains_normal_resolution(self):
        self.config["BBMULTICONFIG"] = "configured"
        self.assertIsNone(self.connection._fast_recipe_data("u-boot"))
        self.assertEqual(self.parsed, [])

    def test_prepared_dynamic_collections_are_included_in_candidate_selection(self):
        dynamic = self.root / "dynamic"
        dynamic.mkdir()
        (dynamic / "u-boot_2025.01.bb").write_text("")
        self.config["BBFILES_DYNAMIC"] = "collection:" + str(dynamic / "*.bb")
        self.config["BBFILES"] += " " + str(dynamic / "*.bb")
        self.assertIsNone(self.connection._fast_recipe_data("u-boot"))
        self.assertEqual(self.parsed, [])

    def test_actual_pn_and_virtual_provider_must_match(self):
        self.data["PN"] = "different"
        self.assertIsNone(self.connection._fast_recipe_data("u-boot"))
        self.data["PN"] = "u-boot"
        self.data["PROVIDES"] = "unrelated"
        self.assertIsNone(self.connection._fast_recipe_data("virtual/bootloader"))
        self.assertEqual(self.connection.recipe_files, {})

    def test_fast_path_is_opt_in_and_required_version_is_not_ignored(self):
        with patch.dict(os.environ, {"YOCTUI_BRIDGE_FAST_RECIPE_INSPECTION": "0"}):
            self.assertIsNone(self.connection._fast_recipe_data("u-boot"))
        self.config["REQUIRED_VERSION_u-boot"] = "2027.01"
        self.assertIsNone(self.connection._fast_recipe_data("u-boot"))
        self.assertEqual(self.connection.recipe_files, {})
