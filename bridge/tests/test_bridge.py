"""Bridge framing tests; compatible with both unittest and pytest collection."""

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


BRIDGE = Path(__file__).parents[1] / "yoctui_bridge.py"
MAX_LINE_BYTES = 1024 * 1024


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
                "YOCTUI_VARIABLE_PROVENANCE_JSON": json.dumps(
                    {"MACHINE": "conf/local.conf:12"}
                ),
            },
        )
        message = json.loads(result.stdout)
        self.assertEqual(message["message"]["type"], "workspace")
        self.assertIn("build_dir", message["message"]["data"])
        self.assertIn("variables", message["message"]["data"])
        self.assertEqual(message["message"]["data"]["release"], "5.0")
        self.assertEqual(
            message["message"]["data"]["variable_provenance"]["MACHINE"],
            "conf/local.conf:12",
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

    def test_unsupported_bitbake_version_is_reported(self) -> None:
        result = run_bridge(environment={"YOCTUI_BITBAKE_VERSION": "0.9"})
        message = json.loads(result.stdout)["message"]
        self.assertEqual(message["code"], "unsupported_bitbake")

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

    def test_dependencies_without_a_server_capability_are_not_guessed(self) -> None:
        result = run_bridge(
            b'{"protocol_version":1,"sequence":1,"message":{"type":"get_dependencies","recipe":"busybox"}}'
        )
        message = json.loads(result.stdout)["message"]
        self.assertEqual(message["type"], "command_failed")
        self.assertEqual(message["code"], "bitbake_server_unavailable")

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
 def drain_events(self): return [ParseProgress(), Warning(), Error(), TaskStarted(), TaskSucceeded(), BuildCompleted()]
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
                "task_started",
                "task_completed",
                "build_completed",
            ],
        )
        self.assertEqual(messages[1]["current"], 8)
        self.assertEqual(messages[1]["total"], 20)
        self.assertEqual(messages[2]["level"], "warning")
        self.assertEqual(messages[3]["level"], "error")
        self.assertEqual(messages[4]["pid"], 42)
        self.assertTrue(messages[5]["success"])
        self.assertEqual(messages[6]["exit_code"], 1)

    def test_parent_eof_exits_cleanly(self) -> None:
        result = run_bridge()
        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stdout, b"")

    def test_oversized_input_is_rejected_without_crashing(self) -> None:
        result = run_bridge(b"x" * (MAX_LINE_BYTES + 1))
        self.assertEqual(result.returncode, 0)
        message = json.loads(result.stdout)
        self.assertEqual(message["message"]["code"], "message_too_large")
