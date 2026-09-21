"""BridgeServerAdapterTests regression coverage."""

from .support import *  # noqa: F403


class BridgeServerAdapterTests(unittest.TestCase):  # noqa: F405
    def test_mocked_bitbake_events_are_normalized(self) -> None:
        events = json.dumps(
            [
                {
                    "type": "task_started",
                    "recipe": "busybox",
                    "task": "do_compile",
                    "pid": 42,
                },
                {"type": "unknown"},
            ]
        )
        result = run_bridge(
            b'{"protocol_version":1,"sequence":1,"message":{"type":"start_build","targets":["busybox"],"task":null}}',
            environment={"YOCTUI_MOCK_EVENTS_JSON": events},
        )
        messages = [json.loads(line)["message"] for line in result.stdout.splitlines()]
        self.assertEqual(messages[-1]["code"], "bitbake_server_unavailable")

    def test_mocked_server_adapter_starts_and_cancels(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            Path(directory, "bb.py").write_text(
                """__version__ = "2.8.1"\nclass Connection:\n def start_build(self, targets, task): pass\n def cancel_build(self): pass\nclass Server:\n def connect(self): return Connection()\nserver = Server()\n""",
                encoding="utf-8",
            )
            result = run_bridge(
                b'{"protocol_version":1,"sequence":1,"message":{"type":"start_build","targets":["busybox"],"task":null}}',
                b'{"protocol_version":1,"sequence":2,"message":{"type":"cancel_build"}}',
                environment={"PYTHONPATH": directory},
            )
        self.assertEqual(
            [
                json.loads(line)["message"]["type"]
                for line in result.stdout.splitlines()
            ],
            ["build_started", "build_completed"],
        )

    def test_compatibility_cancellation_preempts_native_event_flood(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            cancelled = Path(directory, "cancelled")
            Path(directory, "bb.py").write_text(
                f"""__version__ = "2.18.0"
class Connection:
 native_event_stream = True
 def __init__(self): self.cancelled = False
 def start_build(self, targets, task, force=False): pass
 def cancel_build(self):
  self.cancelled = True
  open({str(cancelled)!r}, "w", encoding="utf-8").write("cancelled")
 def drain_events(self):
  def events():
   for index in range(10000):
    if self.cancelled:
     yield {{"type": "build_completed", "success": False, "exit_code": 1}}
     yield {{"type": "build_started"}}
     return
    yield {{"type": "parse_progress", "parsed": index, "total": 10000}}
  return events()
 def shutdown(self): pass
class Server:
 def __init__(self): self.connection = Connection()
 def connect(self): return self.connection
server = Server()
""",
                encoding="utf-8",
            )
            capabilities = [
                {
                    "id": capability,
                    "implementation": "tinfoil.adapter.modern",
                }
                for capability in (
                    "bitbake.build",
                    "bitbake.cancellation",
                    "bitbake.native_events",
                )
            ]
            commands = [
                {
                    "protocol_version": 1,
                    "sequence": 1,
                    "correlation_id": 100,
                    "message": {
                        "type": "hello",
                        "compatibility": {
                            "generation": 1,
                            "build_directory": str(Path.cwd()),
                            "capabilities": capabilities,
                        },
                    },
                },
                {
                    "protocol_version": 1,
                    "sequence": 2,
                    "correlation_id": 101,
                    "message": {
                        "type": "start_build",
                        "targets": ["base-files"],
                        "task": None,
                    },
                },
                {
                    "protocol_version": 1,
                    "sequence": 3,
                    "correlation_id": 102,
                    "message": {"type": "cancel_build"},
                },
                {
                    "protocol_version": 1,
                    "sequence": 4,
                    "correlation_id": 103,
                    "message": {"type": "shutdown"},
                },
            ]
            result = run_bridge(
                *(json.dumps(command).encode() for command in commands),
                environment={"PYTHONPATH": directory},
            )
            cancelled_text = cancelled.read_text(encoding="utf-8")

        self.assertEqual(result.returncode, 0, result.stderr.decode())
        self.assertEqual(cancelled_text, "cancelled")
        messages = [json.loads(line) for line in result.stdout.splitlines()]
        terminal = [
            message
            for message in messages
            if message["message"]["type"] == "build_completed"
        ]
        self.assertEqual(len(terminal), 1)
        self.assertEqual(terminal[0]["correlation_id"], 101)
        self.assertFalse(terminal[0]["message"]["success"])
        self.assertFalse(
            any(message["message"]["type"] == "build_started" for message in messages)
        )
        terminal_index = messages.index(terminal[0])
        self.assertEqual(messages[-1]["message"]["type"], "bridge_shutdown")
        self.assertLess(terminal_index, len(messages) - 1)
        self.assertLessEqual(
            sum(
                message["message"]["type"] == "parse_progress"
                for message in messages[:terminal_index]
            ),
            MAX_NATIVE_EVENTS_PER_POLL,
        )

    def test_mocked_server_adapter_reports_variable_provenance(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            Path(directory, "bb.py").write_text(
                """__version__ = "2.8.1"
class Connection:
 def get_variable(self, name, recipe):
  assert name == "MACHINE"
  assert recipe is None
  return {"value": "qemuarm", "provenance": "conf/local.conf:12"}
class Server:
 def connect(self): return Connection()
server = Server()
""",
                encoding="utf-8",
            )
            result = run_bridge(
                b'{"protocol_version":1,"sequence":1,"message":{"type":"get_variable","name":"MACHINE","recipe":null}}',
                environment={"PYTHONPATH": directory},
            )
        message = json.loads(result.stdout)["message"]
        self.assertEqual(message["type"], "variable")
        self.assertEqual(message["value"], "qemuarm")
        self.assertEqual(message["provenance"], "conf/local.conf:12")

    def test_config_metadata_uses_tinfoil_unexpanded_values_and_history(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            package = Path(directory, "bb")
            package.mkdir()
            Path(package, "__init__.py").write_text(
                '__version__ = "2.19.0"\n', encoding="utf-8"
            )
            Path(package, "tinfoil.py").write_text(
                """class History:
 def variable(self, name):
  assert name == "MACHINE"
  return [
   {"op": "set", "file": "/layers/meta/conf/machine/include/qemu.inc", "line": 3, "detail": "${DEFAULT_MACHINE}"},
   {"op": "append[qemux86-64]", "file": "/build/conf/local.conf", "line": 12, "detail": " qemux86-64"},
   {"op": "set", "file": "/ignored", "line": 1, "detail": "flag", "flag": "doc"},
  ]
class Data:
 varhistory = History()
 def getVar(self, name, expand=True):
  values = {"MACHINE": "qemux86-64" if expand else "${DEFAULT_MACHINE}", "OVERRIDES": "x86-64:qemux86-64:poky"}
  return values.get(name)
class Tinfoil:
 def __init__(self, **kwargs): self.config_data = Data()
 def prepare(self, **kwargs): pass
 def parse_recipes(self): pass
 def parse_recipe(self, recipe):
  assert recipe == "base-files"
  return Data()
 def shutdown(self): pass
""",
                encoding="utf-8",
            )
            result = run_bridge(
                b'{"protocol_version":1,"sequence":1,"message":{"type":"get_variable","name":"MACHINE","recipe":"base-files"}}',
                environment={"PYTHONPATH": directory},
            )
        message = json.loads(result.stdout)["message"]
        self.assertEqual(message["type"], "variable")
        self.assertEqual(message["recipe"], "base-files")
        self.assertEqual(message["value"], "qemux86-64")
        self.assertEqual(message["unexpanded_value"], "${DEFAULT_MACHINE}")
        self.assertEqual(message["provenance"], "/build/conf/local.conf:12")
        self.assertEqual(len(message["operations"]), 2)
        self.assertEqual(message["operations"][1]["operation"], "append[qemux86-64]")
        self.assertEqual(message["active_overrides"], ["x86-64", "qemux86-64", "poky"])

    def test_config_metadata_rejects_malformed_server_operations(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            Path(directory, "bb.py").write_text(
                """__version__ = "2.8.1"
class Connection:
 def get_variable(self, name, recipe):
  return {"value": "qemuarm", "operations": [{"operation": "set", "line": "twelve"}]}
class Server:
 def connect(self): return Connection()
server = Server()
""",
                encoding="utf-8",
            )
            result = run_bridge(
                b'{"protocol_version":1,"sequence":1,"message":{"type":"get_variable","name":"MACHINE","recipe":null}}',
                environment={"PYTHONPATH": directory},
            )
        message = json.loads(result.stdout)["message"]
        self.assertEqual(message["type"], "command_failed")
        self.assertEqual(message["code"], "bitbake_server_unavailable")
        self.assertIn("invalid variable operations", message["message"])

    def test_mocked_server_adapter_returns_authoritative_dependencies(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            Path(directory, "bb.py").write_text(
                """__version__ = "2.8.1"
class Connection:
 def get_dependencies(self, recipe):
  assert recipe == "busybox"
  return {"build": ["virtual/libc", "zlib"], "runtime": ["base-files"]}
class Server:
 def connect(self): return Connection()
server = Server()
""",
                encoding="utf-8",
            )
            result = run_bridge(
                b'{"protocol_version":1,"sequence":1,"message":{"type":"get_dependencies","recipe":"busybox"}}',
                environment={"PYTHONPATH": directory},
            )
        message = json.loads(result.stdout)["message"]
        self.assertEqual(message["type"], "dependencies")
        self.assertEqual(message["recipe"], "busybox")
        self.assertEqual(message["build"], ["virtual/libc", "zlib"])
        self.assertEqual(message["runtime"], ["base-files"])

    def test_dependency_graph_is_typed_bounded_and_identity_correlated(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            Path(directory, "bb.py").write_text(
                """__version__ = "2.8.1"
class Connection:
 def get_dependency_graph(self, recipe):
  assert recipe == "image"
  nodes = [{"id": {"recipe": "dep-%04d" % index}} for index in range(1600)]
  nodes += [{"id": {"recipe": "image"}}, {"id": {"recipe": "image"}}]
  return {
   "root": {"recipe": "image"},
   "nodes": nodes,
   "edges": [
    {"from": {"recipe": "image"}, "to": {"recipe": "dep-0001"}, "kind": "build"},
    {"from": {"recipe": "image"}, "to": {"recipe": "dep-0001"}, "kind": "build"},
   ],
   "limitations": [],
  }
class Server:
 def connect(self): return Connection()
server = Server()
""",
                encoding="utf-8",
            )
            result = run_bridge(
                b'{"protocol_version":1,"sequence":1,"message":{"type":"get_dependency_graph","recipe":"image"}}',
                environment={"PYTHONPATH": directory},
            )
        self.assertEqual(result.returncode, 0, result.stderr)
        message = json.loads(result.stdout)["message"]
        self.assertEqual(message["type"], "dependency_graph")
        self.assertEqual(message["data"]["root"], {"recipe": "image"})
        self.assertEqual(len(message["data"]["nodes"]), 1500)
        self.assertEqual(len(message["data"]["edges"]), 1)
        self.assertIn("bounds dropped", message["data"]["limitations"][0])

    def test_dependency_graph_rejects_wrong_root_and_malformed_edges(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            Path(directory, "bb.py").write_text(
                """__version__ = "2.8.1"
class Connection:
 def get_dependency_graph(self, recipe):
  return {
   "root": {"recipe": "other"},
   "nodes": [],
   "edges": [{"from": {"recipe": recipe}, "to": {"recipe": "dep"}, "kind": "guessed"}],
  }
class Server:
 def connect(self): return Connection()
server = Server()
""",
                encoding="utf-8",
            )
            result = run_bridge(
                b'{"protocol_version":1,"sequence":1,"message":{"type":"get_dependency_graph","recipe":"image"}}',
                environment={"PYTHONPATH": directory},
            )
        message = json.loads(result.stdout)["message"]
        self.assertEqual(message["type"], "command_failed")
        self.assertEqual(message["code"], "bitbake_server_unavailable")
        self.assertIn("different root", message["message"])

    def test_dependency_graph_falls_back_to_legacy_direct_edges(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            Path(directory, "bb.py").write_text(
                """__version__ = "2.8.1"
class Connection:
 def get_dependencies(self, recipe):
  return {"build": ["zlib"], "runtime": ["base-files"]}
class Server:
 def connect(self): return Connection()
server = Server()
""",
                encoding="utf-8",
            )
            result = run_bridge(
                b'{"protocol_version":1,"sequence":1,"message":{"type":"get_dependency_graph","recipe":"busybox"}}',
                environment={"PYTHONPATH": directory},
            )
        message = json.loads(result.stdout)["message"]
        self.assertEqual(message["type"], "dependency_graph")
        self.assertEqual(
            [edge["kind"] for edge in message["data"]["edges"]],
            ["runtime", "build"],
        )
        self.assertIn("Legacy server", message["data"]["limitations"][0])

    def test_tinfoil_dependency_tree_events_become_a_typed_graph(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            package = Path(directory, "bb")
            package.mkdir()
            Path(package, "__init__.py").write_text(
                '__version__ = "2.19.0"\n', encoding="utf-8"
            )
            Path(package, "tinfoil.py").write_text(
                """class Data:
 def getVar(self, name): return "build" if name == "BB_DEFAULT_TASK" else None
class DepTreeGenerated:
 def __init__(self):
  self._depgraph = {
   "pn": {
    "image": {"filename": "/layers/meta/recipes-core/images/image.bb"},
    "busybox": {"filename": "/layers/meta/recipes-core/busybox/busybox.bb"},
   },
   "depends": {"image": ["virtual/busybox"]},
   "rdepends-pn": {"image": ["busybox"]},
   "tdepends": {"image.do_build": ["busybox.do_package"]},
   "providermap": {"virtual/busybox": ["busybox"]},
  }
class CommandCompleted: pass
class Tinfoil:
 def __init__(self, **kwargs):
  self.config_data = Data()
  self.events = []
 def prepare(self, **kwargs): pass
 def parse_recipes(self): pass
 def set_event_mask(self, event_mask): self.event_mask = event_mask
 def run_command(self, command, targets, task, **kwargs):
  assert command == "generateDepTreeEvent"
  assert targets == ["image"]
  assert task == "build"
  self.events = [DepTreeGenerated(), CommandCompleted()]
 def wait_event(self, timeout):
  return self.events.pop(0) if self.events else None
 def shutdown(self): pass
""",
                encoding="utf-8",
            )
            result = run_bridge(
                b'{"protocol_version":1,"sequence":1,"message":{"type":"get_dependency_graph","recipe":"image"}}',
                environment={"PYTHONPATH": directory},
            )
        self.assertEqual(result.returncode, 0, result.stderr)
        message = json.loads(result.stdout)["message"]
        self.assertEqual(message["type"], "dependency_graph")
        self.assertEqual(message["data"]["root"], {"recipe": "image"})
        self.assertEqual(
            [edge["kind"] for edge in message["data"]["edges"]],
            ["build", "runtime", "task"],
        )
        self.assertEqual(
            next(
                node["provider"]
                for node in message["data"]["nodes"]
                if node["id"] == {"recipe": "busybox"}
            ),
            "/layers/meta/recipes-core/busybox/busybox.bb",
        )
