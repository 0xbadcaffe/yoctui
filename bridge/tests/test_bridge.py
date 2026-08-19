"""Bridge framing tests; compatible with both unittest and pytest collection."""

import json
import importlib.util
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


BRIDGE = Path(__file__).parents[2] / "crates/yoctui-bitbake/bridge/yoctui_bridge.py"
MAX_LINE_BYTES = 1024 * 1024
MAX_NATIVE_EVENTS_PER_POLL = 64


def run_bridge(
    *lines: bytes, environment: dict[str, str] | None = None
) -> subprocess.CompletedProcess[bytes]:
    env = os.environ.copy()
    if environment:
        env.update(environment)
    return subprocess.run(
        [sys.executable, str(BRIDGE)],
        input=b"".join(line + b"\n" for line in lines),
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        env=env,
        check=False,
    )


class BridgeProtocolTests(unittest.TestCase):
    def test_tinfoil_cancellation_uses_bounded_cooker_shutdown(self) -> None:
        spec = importlib.util.spec_from_file_location("yoctui_bridge_test", BRIDGE)
        self.assertIsNotNone(spec)
        self.assertIsNotNone(spec.loader)
        bridge = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(bridge)

        class FakeTinfoil:
            def __init__(self) -> None:
                self.calls: list[tuple[str, bool]] = []

            def run_command(self, command: str, *, handle_events: bool) -> None:
                self.calls.append((command, handle_events))

        connection = object.__new__(bridge.TinfoilConnection)
        connection.active = True
        connection.tinfoil = FakeTinfoil()
        connection.cancel_build()

        self.assertEqual(
            connection.tinfoil.calls,
            [("stateForceShutdown", False)],
        )

    def test_hello_and_shutdown_are_framed_as_json_lines(self) -> None:
        result = run_bridge(
            b'{"protocol_version":1,"sequence":1,"message":{"type":"hello"}}',
            b'{"protocol_version":1,"sequence":2,"message":{"type":"shutdown"}}',
        )
        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stderr, b"")
        messages = [json.loads(line) for line in result.stdout.splitlines()]
        self.assertEqual(
            [m["message"]["type"] for m in messages], ["hello_ack", "bridge_shutdown"]
        )
        self.assertEqual([m["sequence"] for m in messages], [1, 2])

    def test_protocol_output_isolated_from_process_stdout(self) -> None:
        script = (
            "import importlib.util, pathlib; "
            f"p=pathlib.Path({str(BRIDGE)!r}); "
            "s=importlib.util.spec_from_file_location('bridge', p); "
            "m=importlib.util.module_from_spec(s); s.loader.exec_module(m); "
            "m.isolate_protocol_output(); print('bitbake diagnostic'); "
            "m.emit({'type':'hello_ack'})"
        )
        result = subprocess.run(
            [sys.executable, "-c", script],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )
        self.assertEqual(json.loads(result.stdout)["message"]["type"], "hello_ack")
        self.assertEqual(result.stderr, b"bitbake diagnostic\n")

    def test_malformed_input_is_reported_without_crashing(self) -> None:
        result = run_bridge(b"not json")
        self.assertEqual(result.returncode, 0)
        message = json.loads(result.stdout)
        self.assertEqual(message["message"]["type"], "command_failed")
        self.assertEqual(message["message"]["code"], "malformed_command")

    def test_unknown_command_is_typed_error(self) -> None:
        result = run_bridge(
            b'{"protocol_version":1,"sequence":1,"message":{"type":"future"}}'
        )
        message = json.loads(result.stdout)
        self.assertEqual(message["message"]["code"], "unknown_command")

    def test_recipe_bitbake_action_rejects_non_boolean_force(self) -> None:
        result = run_bridge(
            b'{"protocol_version":1,"sequence":1,"message":{"type":"start_build","targets":["busybox"],"task":"compile","force":"yes"}}'
        )
        message = json.loads(result.stdout)["message"]
        self.assertEqual(message["type"], "command_failed")
        self.assertEqual(message["code"], "invalid_request")
        self.assertIn("force must be a boolean", message["message"])

    def test_typed_queries_reject_missing_or_malformed_identities(self) -> None:
        result = run_bridge(
            b'{"protocol_version":1,"sequence":1,"message":{"type":"start_build","targets":[]}}',
            b'{"protocol_version":1,"sequence":2,"message":{"type":"list_recipes","filter":7}}',
            b'{"protocol_version":1,"sequence":3,"message":{"type":"get_variable","name":""}}',
            b'{"protocol_version":1,"sequence":4,"message":{"type":"get_dependency_graph","recipe":null}}',
            b'{"protocol_version":1,"sequence":5,"message":{"type":"get_dependencies"}}',
            b'{"protocol_version":1,"sequence":6,"message":{"type":"get_recipe_sources","recipe":false}}',
            b'{"protocol_version":1,"sequence":7,"message":{"type":"get_recipe_metadata","recipe":""}}',
        )
        messages = [json.loads(line)["message"] for line in result.stdout.splitlines()]
        self.assertEqual(len(messages), 7)
        self.assertTrue(
            all(message["code"] == "invalid_request" for message in messages)
        )
        self.assertEqual(
            [message["message"].split()[0] for message in messages],
            [
                "start_build",
                "list_recipes",
                "get_variable",
                "get_dependency_graph",
                "get_dependencies",
                "get_recipe_sources",
                "get_recipe_metadata",
            ],
        )

    def test_protocol_version_mismatch_is_rejected(self) -> None:
        result = run_bridge(
            b'{"protocol_version":999,"sequence":1,"message":{"type":"hello"}}'
        )
        message = json.loads(result.stdout)
        self.assertEqual(message["message"]["code"], "version_mismatch")

    def test_workspace_contains_environment_values(self) -> None:
        result = run_bridge(
            b'{"protocol_version":1,"sequence":1,"message":{"type":"inspect_workspace"}}',
            environment={
                "DISTRO_VERSION": "5.0",
                "DEPLOY_DIR_IMAGE": "/build/tmp/deploy/images/qemux86-64",
                "WKS_FILE": "/layers/meta/wic/directdisk.wks",
                "YOCTUI_VARIABLE_PROVENANCE_JSON": json.dumps(
                    {"MACHINE": "conf/local.conf:12"}
                ),
                "YOCTUI_VARIABLE_PROVENANCE_CHAIN_JSON": json.dumps(
                    {"MACHINE": ["meta/conf/bitbake.conf:1", "conf/local.conf:12"]}
                ),
            },
        )
        message = json.loads(result.stdout)
        self.assertEqual(message["message"]["type"], "workspace")
        self.assertIn("build_dir", message["message"]["data"])
        self.assertIn("variables", message["message"]["data"])
        self.assertEqual(message["message"]["data"]["release"], "5.0")
        self.assertEqual(
            message["message"]["data"]["variables"]["DEPLOY_DIR_IMAGE"],
            "/build/tmp/deploy/images/qemux86-64",
        )
        self.assertEqual(
            message["message"]["data"]["variables"]["WKS_FILE"],
            "/layers/meta/wic/directdisk.wks",
        )
        self.assertEqual(
            message["message"]["data"]["variable_provenance"]["MACHINE"],
            "conf/local.conf:12",
        )
        self.assertEqual(
            message["message"]["data"]["variable_provenance_chain"]["MACHINE"],
            ["meta/conf/bitbake.conf:1", "conf/local.conf:12"],
        )

    def test_typed_workspace_queries_return_protocol_responses(self) -> None:
        result = run_bridge(
            b'{"protocol_version":1,"sequence":1,"message":{"type":"list_recipes","filter":null}}',
            b'{"protocol_version":1,"sequence":2,"message":{"type":"list_layers"}}',
            b'{"protocol_version":1,"sequence":3,"message":{"type":"get_variable","name":"PATH","recipe":null}}',
            environment={
                "YOCTUI_VARIABLE_PROVENANCE_JSON": json.dumps(
                    {"PATH": "conf/local.conf:8"}
                )
            },
        )
        messages = [json.loads(line)["message"] for line in result.stdout.splitlines()]
        self.assertEqual(
            [message["type"] for message in messages], ["recipes", "layers", "variable"]
        )
        self.assertEqual(messages[0]["recipes"], [])
        self.assertIsInstance(messages[1]["layers"], list)
        self.assertEqual(messages[2]["name"], "PATH")
        self.assertEqual(messages[2]["provenance"], "conf/local.conf:8")

    def test_recipe_listing_uses_bitbake_when_server_api_is_unavailable(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            command = Path(directory, "bitbake")
            command.write_text(
                "#!/bin/sh\nprintf 'busybox : 1.36.0-r0\\ncore-image-minimal : 1.0-r0\\n'\n",
                encoding="utf-8",
            )
            command.chmod(0o755)
            result = run_bridge(
                b'{"protocol_version":1,"sequence":1,"message":{"type":"list_recipes","filter":"busy"}}',
                environment={"PATH": f"{directory}:{os.environ['PATH']}"},
            )
        message = json.loads(result.stdout)["message"]
        self.assertEqual(
            message["recipes"],
            [{"name": "busybox", "version": "1.36.0-r0", "layer": None}],
        )

    def test_recipe_metadata_unavailable_is_a_typed_error(self) -> None:
        result = run_bridge(
            b'{"protocol_version":1,"sequence":1,"message":{"type":"get_recipe_metadata","recipe":"busybox"}}'
        )
        message = json.loads(result.stdout)["message"]
        self.assertEqual(message["type"], "command_failed")
        self.assertEqual(message["code"], "bitbake_server_unavailable")
        self.assertIn("get_recipe_metadata", message["message"])

    def test_layer_reports_supply_recipe_ownership_and_paths(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            command = Path(directory, "bitbake-layers")
            command.write_text(
                """#!/bin/sh
if [ "$1" = show-recipes ]; then
 printf 'busybox:\\n  meta-core 1.38.0\\n'
else
 printf 'layer path priority\\n=====\\nmeta-core /layers/meta-core 5\\n'
fi
""",
                encoding="utf-8",
            )
            command.chmod(0o755)
            result = run_bridge(
                b'{"protocol_version":1,"sequence":1,"message":{"type":"list_recipes","filter":null}}',
                b'{"protocol_version":1,"sequence":2,"message":{"type":"list_layers"}}',
                environment={"PATH": f"{directory}:{os.environ['PATH']}"},
            )
        messages = [json.loads(line)["message"] for line in result.stdout.splitlines()]
        self.assertEqual(messages[0]["recipes"][0]["layer"], "meta-core")
        self.assertEqual(messages[1]["layers"][0]["path"], "/layers/meta-core")

    def test_mocked_bitbake_module_selects_modern_adapter(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            Path(directory, "bb.py").write_text(
                '__version__ = "2.8.1"\n', encoding="utf-8"
            )
            result = run_bridge(
                b'{"protocol_version":1,"sequence":1,"message":{"type":"hello"}}',
                environment={"PYTHONPATH": directory},
            )
        message = json.loads(result.stdout)["message"]
        self.assertEqual(message["type"], "hello_ack")
        self.assertEqual(message["bitbake_version"], "2.8.1")

    def test_compatibility_unknown_future_bitbake_version_is_not_rejected(self) -> None:
        result = run_bridge(
            b'{"protocol_version":1,"sequence":1,"message":{"type":"hello"}}',
            environment={"YOCTUI_BITBAKE_VERSION": "99.0"},
        )
        message = json.loads(result.stdout)["message"]
        self.assertEqual(message["type"], "hello_ack")
        self.assertEqual(message["bitbake_version"], "99.0")

    def test_compatibility_handshake_negotiates_only_direct_backend_behavior(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            Path(directory, "bb.py").write_text(
                '''__version__ = "99.0"
class Connection:
 native_event_stream = True
 def inspect_workspace(self): return {"build_dir": %r, "variables": {}}
 def start_build(self, targets, task, force=False): pass
 def drain_events(self): return []
class Server:
 def connect(self): return Connection()
server = Server()
'''
                % str(Path.cwd()),
                encoding="utf-8",
            )
            hello = {
                "protocol_version": 1,
                "sequence": 1,
                "message": {
                    "type": "hello",
                    "compatibility": {
                        "generation": 7,
                        "build_directory": str(Path.cwd()),
                        "capabilities": [
                            {
                                "id": "bitbake.workspace_inspection",
                                "implementation": "tinfoil.adapter.modern",
                            },
                            {
                                "id": "bitbake.build",
                                "implementation": "tinfoil.adapter.modern",
                            },
                            {
                                "id": "bitbake.native_events",
                                "implementation": "tinfoil.adapter.modern",
                            },
                        ],
                    },
                },
            }
            result = run_bridge(
                json.dumps(hello).encode(), environment={"PYTHONPATH": directory}
            )
        message = json.loads(result.stdout)["message"]
        self.assertEqual(message["type"], "hello_ack")
        self.assertEqual(message["compatibility_generation"], 7)
        self.assertEqual(
            message["capabilities"],
            [
                "bitbake.build",
                "bitbake.native_events",
                "bitbake.workspace_inspection",
            ],
        )

    def test_compatibility_absent_api_is_rejected_before_backend_call(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            marker = Path(directory, "called")
            Path(directory, "bb.py").write_text(
                f'''__version__ = "2.18"
class Connection:
 def inspect_workspace(self): return {{"build_dir": {str(Path.cwd())!r}, "variables": {{}}}}
class Server:
 def connect(self): return Connection()
server = Server()
''',
                encoding="utf-8",
            )
            hello = {
                "protocol_version": 1,
                "sequence": 1,
                "message": {
                    "type": "hello",
                    "compatibility": {
                        "generation": 9,
                        "build_directory": str(Path.cwd()),
                        "capabilities": [
                            {
                                "id": "bitbake.cancellation",
                                "implementation": "tinfoil.cancel",
                            }
                        ],
                    },
                },
            }
            cancel = {
                "protocol_version": 1,
                "sequence": 2,
                "message": {"type": "cancel_build"},
            }
            result = run_bridge(
                json.dumps(hello).encode(),
                json.dumps(cancel).encode(),
                environment={"PYTHONPATH": directory, "MARKER": str(marker)},
            )
        messages = [json.loads(line)["message"] for line in result.stdout.splitlines()]
        self.assertEqual(messages[0]["capabilities"], [])
        self.assertEqual(messages[1]["code"], "compatibility_unavailable")
        self.assertFalse(marker.exists())

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
                f'''__version__ = "2.18.0"
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
''',
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
                "task_queued",
                "task_started",
                "task_completed",
                "build_completed",
            ],
        )
        self.assertEqual(messages[1]["current"], 8)
        self.assertEqual(messages[1]["total"], 20)
        self.assertEqual(messages[2]["level"], "warning")
        self.assertEqual(messages[3]["level"], "error")
        self.assertEqual(messages[4]["recipe"], "busybox")
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
   TaskProgress(99, 50), TaskStarted(), TaskProgress(42, 63.9),
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
        self.assertEqual(messages[8]["progress"], 63)
        self.assertEqual(messages[9]["progress"], 100)
        self.assertIsNone(messages[10]["progress"])
        self.assertNotIn("unrecognized BitBake event", result.stdout.decode())

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
