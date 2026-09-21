"""BridgeMetadataEventTests regression coverage."""

from .support import *  # noqa: F403


class BridgeMetadataEventTests(unittest.TestCase):  # noqa: F405
    def test_dependencies_without_a_server_capability_are_not_guessed(self) -> None:
        result = run_bridge(
            b'{"protocol_version":1,"sequence":1,"message":{"type":"get_dependencies","recipe":"busybox"}}'
        )
        message = json.loads(result.stdout)["message"]
        self.assertEqual(message["type"], "command_failed")
        self.assertEqual(message["code"], "bitbake_server_unavailable")

    def test_mocked_server_adapter_returns_recipe_source_paths(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            Path(directory, "bb.py").write_text(
                """__version__ = "2.8.1"
class Connection:
 def get_recipe_sources(self, recipe):
  assert recipe == "busybox"
  return ["/layers/meta/recipes-core/busybox/busybox_1.0.bb", "/layers/meta-custom/recipes-core/busybox/busybox_%.bbappend"]
class Server:
 def connect(self): return Connection()
server = Server()
""",
                encoding="utf-8",
            )
            result = run_bridge(
                b'{"protocol_version":1,"sequence":1,"message":{"type":"get_recipe_sources","recipe":"busybox"}}',
                environment={"PYTHONPATH": directory},
            )
        message = json.loads(result.stdout)["message"]
        self.assertEqual(message["type"], "recipe_sources")
        self.assertEqual(message["recipe"], "busybox")
        self.assertEqual(len(message["paths"]), 2)

    def test_mocked_server_adapter_lists_typed_workspace_data(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            Path(directory, "bb.py").write_text(
                """__version__ = "2.8.1"
class Connection:
 def list_recipes(self, filter_value):
  assert filter_value == "busy"
  return [{"name": "busybox", "version": "1.36", "layer": "meta"}]
 def list_layers(self):
  return [{"name": "meta", "path": "/src/meta", "priority": 5}]
class Server:
 def connect(self): return Connection()
server = Server()
""",
                encoding="utf-8",
            )
            result = run_bridge(
                b'{"protocol_version":1,"sequence":1,"message":{"type":"list_recipes","filter":"busy"}}',
                b'{"protocol_version":1,"sequence":2,"message":{"type":"list_layers"}}',
                environment={"PYTHONPATH": directory},
            )
        messages = [json.loads(line)["message"] for line in result.stdout.splitlines()]
        self.assertEqual(messages[0]["recipes"][0]["name"], "busybox")
        self.assertEqual(messages[1]["layers"][0]["path"], "/src/meta")

    def test_mocked_server_adapter_inspects_typed_workspace(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            Path(directory, "bb.py").write_text(
                """__version__ = "2.8.1"
class Connection:
 def inspect_workspace(self):
  return {"build_dir": "/build", "source_dir": "/src", "variables": {"MACHINE": "qemuarm"}, "variable_provenance": {"MACHINE": "conf/local.conf:12"}, "bitbake_version": "2.8.1", "release": "5.0", "layers": [], "recipes": []}
class Server:
 def connect(self): return Connection()
server = Server()
""",
                encoding="utf-8",
            )
            result = run_bridge(
                b'{"protocol_version":1,"sequence":1,"message":{"type":"inspect_workspace"}}',
                environment={"PYTHONPATH": directory},
            )
        data = json.loads(result.stdout)["message"]["data"]
        self.assertEqual(data["build_dir"], "/build")
        self.assertEqual(data["variables"]["MACHINE"], "qemuarm")
        self.assertEqual(data["variable_provenance"]["MACHINE"], "conf/local.conf:12")

    def test_mocked_server_drains_native_event_objects(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            Path(directory, "bb.py").write_text(
                """__version__ = "2.8.1"
class TaskStarted:
 def __init__(self): self.pn = "busybox"; self.task = "do_compile"; self.pid = 42
class TaskSucceeded:
 def __init__(self): self.pn = "busybox"; self.task = "do_compile"
class Stats:
 def __init__(self): self.completed = 3; self.total = 10; self.active = 1; self.failed = 0
class runQueueTaskStarted:
 def __init__(self): self.taskfile = "/layers/meta/recipes-core/busybox/busybox_1.36.bb"; self.taskname = "do_compile"; self.stats = Stats()
class ParseProgress:
 def __init__(self): self.current = 8; self.total = 20
class Warning:
 def __init__(self): self.message = "deprecated setting"; self.pn = "busybox"
class Error:
 def __init__(self): self.message = "task failed"; self.pn = "busybox"; self.task = "do_compile"
class BuildCompleted:
 def __init__(self): self.success = False; self.returncode = 1
class Connection:
 def start_build(self, targets, task): pass
 def drain_events(self): return [ParseProgress(), Warning(), Error(), runQueueTaskStarted(), TaskStarted(), TaskSucceeded(), BuildCompleted()]
class Server:
 def connect(self): return Connection()
server = Server()
""",
                encoding="utf-8",
            )
            result = run_bridge(
                b'{"protocol_version":1,"sequence":1,"message":{"type":"start_build","targets":["busybox"],"task":null}}',
                environment={"PYTHONPATH": directory},
            )
        messages = [json.loads(line)["message"] for line in result.stdout.splitlines()]
        self.assertEqual(
            [message["type"] for message in messages],
            [
                "build_started",
                "parse_progress",
                "log",
                "log",
                "task_stats",
                "task_started",
                "task_completed",
                "build_completed",
            ],
        )
        self.assertEqual(messages[1]["current"], 8)
        self.assertEqual(messages[1]["total"], 20)
        self.assertEqual(messages[2]["level"], "warning")
        self.assertEqual(messages[3]["level"], "error")
        self.assertNotIn("recipe", messages[4])
        self.assertEqual(messages[4]["stats"]["total"], 10)
        self.assertEqual(messages[5]["pid"], 42)
        self.assertTrue(messages[6]["success"])
        self.assertEqual(messages[7]["exit_code"], 1)

    def test_live_progress_normalizes_fractions_and_correlates_worker_pid(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            Path(directory, "bb.py").write_text(
                """__version__ = "2.8.1"
class ProcessStarted:
 def __init__(self): self.total = 100
class ProcessProgress:
 def __init__(self, progress): self.progress = progress
class TaskStarted:
 def __init__(self): self.pn = "busybox"; self.task = "do_compile"; self.pid = 42
class TaskProgress:
 def __init__(self, pid, progress): self.pid = pid; self.progress = progress
class LogRecord:
 def __init__(self, pid, message): self.pid = pid; self.message = message; self.levelname = "INFO"
class TaskSucceeded:
 def __init__(self): self.pn = "busybox"; self.task = "do_compile"; self.pid = 42
class BuildCompleted:
 def __init__(self): self._failures = 0; self._interrupted = 0
 def getFailures(self): return self._failures
class Connection:
 def __init__(self):
  self.events = [
   ProcessStarted(), ProcessProgress(77.92379445665797),
   ProcessProgress(180), ProcessProgress(-1), ProcessProgress(float("nan")),
   ProcessProgress(True),
   TaskProgress(99, 50), TaskStarted(), LogRecord(42, "compiler output"), TaskProgress(42, 63.9),
   TaskProgress(42, 150), TaskProgress(42, -1), TaskSucceeded(),
   TaskProgress(42, 90), BuildCompleted()
  ]
 def start_build(self, targets, task): pass
 def drain_events(self):
  events, self.events = self.events, []
  return events
class Server:
 def connect(self): return Connection()
server = Server()
""",
                encoding="utf-8",
            )
            result = run_bridge(
                b'{"protocol_version":1,"sequence":1,"message":{"type":"start_build","targets":["busybox"],"task":null}}',
                environment={"PYTHONPATH": directory},
            )
        messages = [json.loads(line)["message"] for line in result.stdout.splitlines()]
        self.assertEqual(
            [message["type"] for message in messages],
            [
                "build_started",
                "parse_progress",
                "parse_progress",
                "parse_progress",
                "parse_progress",
                "parse_progress",
                "parse_progress",
                "task_started",
                "log",
                "task_progress",
                "task_progress",
                "task_progress",
                "task_completed",
                "build_completed",
            ],
        )
        self.assertEqual(
            messages[1], {"type": "parse_progress", "current": 0, "total": 100}
        )
        self.assertEqual(messages[2]["current"], 77)
        self.assertEqual(messages[2]["total"], 100)
        self.assertEqual(messages[3]["current"], 100)
        self.assertIsNone(messages[4]["current"])
        self.assertIsNone(messages[5]["current"])
        self.assertIsNone(messages[6]["current"])
        self.assertEqual(messages[7]["pid"], 42)
        self.assertEqual(messages[8]["recipe"], "busybox")
        self.assertEqual(messages[8]["task"], "do_compile")
        self.assertEqual(messages[9]["recipe"], "busybox")
        self.assertEqual(messages[9]["task"], "do_compile")
        self.assertEqual(messages[9]["progress"], 63)
        self.assertEqual(messages[10]["progress"], 100)
        self.assertIsNone(messages[11]["progress"])
        self.assertNotIn("unrecognized BitBake event", result.stdout.decode())

    def test_real_logrecord_taskpid_correlates_selected_task(self) -> None:
        spec = importlib.util.spec_from_file_location("yoctui_bridge_taskpid", BRIDGE)
        if spec is None or spec.loader is None:
            self.fail(f"could not load bridge module from {BRIDGE}")
        bridge = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(bridge)

        class LogRecord:
            taskpid = 42
            process = 999
            message = "compiler output"
            levelname = "INFO"
            pathname = "/build/tmp/work/busybox/temp/log.do_compile"

        normalized = bridge.normalize_event(
            LogRecord(), {42: ("busybox", "do_compile")}
        )
        self.assertEqual(normalized["type"], "log")
        self.assertEqual(normalized["recipe"], "busybox")
        self.assertEqual(normalized["task"], "do_compile")
        self.assertEqual(
            normalized["path"], "/build/tmp/work/busybox/temp/log.do_compile"
        )

    def test_real_build_completion_shape_infers_success_from_failures(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            Path(directory, "bb.py").write_text(
                """__version__ = "2.8.1"
class BuildCompleted:
 def __init__(self): self._failures = 0; self._interrupted = 0
 def getFailures(self): return self._failures
class Connection:
 def start_build(self, targets, task): pass
 def drain_events(self): return [BuildCompleted()]
class Server:
 def connect(self): return Connection()
server = Server()
""",
                encoding="utf-8",
            )
            result = run_bridge(
                b'{"protocol_version":1,"sequence":1,"message":{"type":"start_build","targets":["base-files"],"task":"listtasks"}}',
                environment={"PYTHONPATH": directory},
            )
        completion = json.loads(result.stdout.splitlines()[-1])["message"]
        self.assertEqual(completion["type"], "build_completed")
        self.assertTrue(completion["success"])
        self.assertEqual(completion["exit_code"], 0)

    def test_recipe_metadata_uses_tinfoil_authoritative_queries(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            package = Path(directory, "bb")
            package.mkdir()
            Path(package, "__init__.py").write_text(
                '__version__ = "2.19.0"\n', encoding="utf-8"
            )
            Path(package, "tinfoil.py").write_text(
                """class History:
 def variable(self, name): return []
class Data:
 varhistory = History()
 def getVar(self, name):
  return {"BBLAYERS": "/layers/meta", "TOPDIR": "/build", "COREBASE": "/layers", "MACHINE": "qemux86-64", "DISTRO_VERSION": "6.0.99", "BBMULTICONFIG": "", "__BBTASKS": ["do_build", "do_compile"], "PACKAGES": "base-files base-files-doc", "SRC_URI": "file://fix.patch file://config"}.get(name)
class Tinfoil:
 def __init__(self, **kwargs): self.config_data = Data()
 def prepare(self, **kwargs): pass
 def parse_recipes(self): pass
 def parse_recipe(self, recipe): return Data()
 def parse_recipe_file(self, path): return Data()
 def get_recipe_file(self, recipe): return "/layers/meta/recipes-core/base-files/base-files_3.0.14.bb"
 def get_file_appends(self, path): return ["/layers/meta-extra/recipes-core/base-files/base-files_%.bbappend"]
 def shutdown(self): pass
 def run_command(self, command, *args, **kwargs):
  if command == "getLayerPriorities": return [("core", "", "^/layers/meta/", 5)]
  if command == "getRecipes": return [("base-files", ["/layers/meta/recipes-core/base-files/base-files_3.0.14.bb"])]
  if command == "getRecipeVersions": return {"/layers/meta/recipes-core/base-files/base-files_3.0.14.bb": ("", "3.0.14", "r0")}
  if command == "findProviders": return ({}, {"base-files": (("", "3.0.14", "r0"), "/layers/meta/recipes-core/base-files/base-files_3.0.14.bb")}, {})
  if command == "getAllAppends": return [("base-files_%.bb", "/layers/meta-extra/recipes-core/base-files/base-files_%.bbappend")]
  return None
""",
                encoding="utf-8",
            )
            Path(package, "fetch2.py").write_text(
                """class Fetch:
 def __init__(self, urls, datastore): pass
 def localpath(self, uri):
  assert uri == "file://fix.patch"
  return "/layers/meta/recipes-core/base-files/files/fix.patch"
""",
                encoding="utf-8",
            )
            result = run_bridge(
                b'{"protocol_version":1,"sequence":1,"message":{"type":"inspect_workspace"}}',
                b'{"protocol_version":1,"sequence":2,"message":{"type":"list_recipes","filter":"base-files"}}',
                b'{"protocol_version":1,"sequence":3,"message":{"type":"get_recipe_metadata","recipe":"base-files"}}',
                b'{"protocol_version":1,"sequence":4,"message":{"type":"shutdown"}}',
                environment={"PYTHONPATH": directory},
            )
        messages = [json.loads(line)["message"] for line in result.stdout.splitlines()]
        self.assertEqual(messages[0]["data"]["bitbake_version"], "2.19.0")
        self.assertEqual(messages[0]["data"]["layers"][0]["name"], "core")
        self.assertEqual(messages[1]["recipes"][0]["version"], "3.0.14")
        self.assertEqual(messages[1]["recipes"][0]["layer"], "core")
        self.assertEqual(
            messages[1]["recipes"][0]["file"],
            "/layers/meta/recipes-core/base-files/base-files_3.0.14.bb",
        )
        self.assertEqual(messages[1]["recipes"][0]["append_count"], 1)
        self.assertEqual(messages[2]["type"], "recipe_metadata")
        self.assertEqual(messages[2]["data"]["tasks"], ["do_build", "do_compile"])
        self.assertEqual(
            messages[2]["data"]["patches"],
            ["/layers/meta/recipes-core/base-files/files/fix.patch"],
        )
        self.assertEqual(
            messages[2]["data"]["packages"], ["base-files", "base-files-doc"]
        )
        self.assertIsNone(messages[2]["data"]["history"])
        self.assertEqual(messages[3]["type"], "bridge_shutdown")

    def test_parent_eof_exits_cleanly(self) -> None:
        result = run_bridge()
        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stdout, b"")

    def test_oversized_input_is_rejected_without_crashing(self) -> None:
        result = run_bridge(b"x" * (MAX_LINE_BYTES + 1))
        self.assertEqual(result.returncode, 0)
        message = json.loads(result.stdout)
        self.assertEqual(message["message"]["code"], "message_too_large")
