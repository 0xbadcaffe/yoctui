"""BridgeProtocolCompatibilityTests regression coverage."""

from .support import *  # noqa: F403


class BridgeProtocolCompatibilityTests(unittest.TestCase):  # noqa: F405
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

    def test_compatibility_handshake_negotiates_only_direct_backend_behavior(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as directory:
            Path(directory, "bb.py").write_text(
                """__version__ = "99.0"
class Connection:
 native_event_stream = True
 def inspect_workspace(self): return {"build_dir": %r, "variables": {}}
 def start_build(self, targets, task, force=False): pass
 def drain_events(self): return []
class Server:
 def connect(self): return Connection()
server = Server()
"""
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
                f"""__version__ = "2.18"
class Connection:
 def inspect_workspace(self): return {{"build_dir": {str(Path.cwd())!r}, "variables": {{}}}}
class Server:
 def connect(self): return Connection()
server = Server()
""",
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
