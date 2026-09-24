"""BridgeWorkspaceInventoryTests regression coverage."""

from .support import *  # noqa: F403


class BridgeWorkspaceInventoryTests(unittest.TestCase):  # noqa: F405
    def test_cache_network_policy_uses_initialized_metadata_defaults(self) -> None:
        spec = importlib.util.spec_from_file_location("yoctui_cache_policy", BRIDGE)
        assert spec is not None and spec.loader is not None
        bridge = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(bridge)
        values = {"DL_DIR": "/cache/downloads", "BB_NO_NETWORK": "1"}
        connection = object.__new__(bridge.TinfoilConnection)
        connection.tinfoil = SimpleNamespace(
            config_data=SimpleNamespace(getVar=values.get)
        )
        connection.module = SimpleNamespace()
        connection._layers = lambda: []
        connection._variable_provenance = lambda data, key: None
        variables = connection.inspect_workspace()["variables"]
        self.assertEqual(variables["BB_NO_NETWORK"], "1")
        self.assertEqual(variables["BB_FETCH_PREMIRRORONLY"], "0")
        self.assertEqual(variables["DL_DIR"], "/cache/downloads")
        values.clear()
        self.assertEqual(connection.inspect_workspace()["variables"]["BB_NO_NETWORK"], "0")

    def test_task_identity_uses_initialized_metadata_for_native_git_and_pn_overrides(
        self,
    ) -> None:
        spec = importlib.util.spec_from_file_location("yoctui_task_identity", BRIDGE)
        assert spec is not None and spec.loader is not None
        bridge = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(bridge)
        identities = [
            ("llvm-native", "virtual:native:/layers/llvm_git.bb"),
            ("lib32-actual-name", "virtual:multilib:lib32:/layers/vendor_git.bb"),
            ("overridden-name", "/layers/not-the-pn_1.0.bb"),
        ]
        stats = SimpleNamespace(completed=3, total=10, active=1, failed=0)

        def event(name, **fields):
            return type(name, (), fields)()

        pending = [event("BuildStarted")]
        for index, (pn, taskfile) in enumerate(identities):
            pending.extend(
                [
                    event(
                        "runQueueTaskStarted",
                        taskfile=taskfile,
                        taskname="do_compile",
                        stats=stats,
                    ),
                    event("TaskStarted", pn=pn, task="do_compile", pid=42 + index),
                    event(
                        "TaskSucceeded", pn=pn, task="do_compile", taskpid=42 + index
                    ),
                ]
            )
        calls = []

        def command(name, *args, **kwargs):
            calls.append((name, args))
            self.assertEqual(name, "getRecipes")
            self.assertEqual(kwargs, {"handle_events": False})
            return [(pn, [path]) for pn, path in identities]

        connection = object.__new__(bridge.TinfoilConnection)
        connection.active = True
        connection.force_active = False
        connection.task_recipe_identities = None
        connection.tinfoil = SimpleNamespace(
            run_command=command,
            wait_event=lambda _: pending.pop(0) if pending else None,
        )
        by_pid: dict[int, tuple[str, str]] = {}
        normalized = [
            bridge.normalize_event(raw, by_pid) for raw in connection.drain_events()
        ]
        queued = [row for row in normalized if row and row["type"] == "task_queued"]
        self.assertEqual(
            [row["recipe"] for row in queued], [pn for pn, _ in identities]
        )
        self.assertTrue(all(row["stats"]["total"] == 10 for row in queued))
        self.assertEqual(calls, [("getRecipes", ("",))])
        self.assertEqual(by_pid, {})

    def test_task_identity_bounds_conflicts_and_unresolved_statistics(self) -> None:
        spec = importlib.util.spec_from_file_location(
            "yoctui_task_identity_bounds", BRIDGE
        )
        assert spec is not None and spec.loader is not None
        bridge = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(bridge)
        index = bridge.task_recipe_identity_index(
            [
                ("one", ["/same.bb"]),
                ("two", ["/same.bb"]),
                ("one", ["/same.bb"]),
            ]
        )
        self.assertIsNone(index["/same.bb"])
        for invalid in [
            None,
            {"one": ["/one.bb"]},
            [("", ["/one.bb"])],
            [("one", [None])],
            [("one", "not-path-list")],
            [("one", ["/one.bb"])] * (bridge.MAX_RECIPE_INVENTORY_RECORDS + 1),
            [("one", ["x" * (bridge.MAX_RECIPE_INVENTORY_BYTES + 1)])],
        ]:
            with self.assertRaises(ValueError):
                bridge.task_recipe_identity_index(invalid)
        stats = {"completed": 3, "total": 10, "active": 1, "failed": 0}
        queued: dict[str, object] = {
            "type": "runQueueTaskStarted",
            "taskfile": "/missing_git.bb",
            "taskname": "do_compile",
            "stats": stats,
        }
        self.assertEqual(
            bridge.normalize_event(queued), {"type": "task_stats", "stats": stats}
        )
        queued["stats"] = None
        self.assertIsNone(bridge.normalize_event(queued))
        queued["stats"] = {**stats, "completed": -1}
        self.assertIsNone(bridge.normalize_event(queued))

    def test_task_identity_cache_refreshes_per_build_and_failure_does_not_guess(
        self,
    ) -> None:
        spec = importlib.util.spec_from_file_location(
            "yoctui_task_identity_refresh", BRIDGE
        )
        assert spec is not None and spec.loader is not None
        bridge = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(bridge)
        pending = []
        queried = []
        build_number = 0

        def event(name, **fields):
            return type(name, (), fields)()

        def command(name, *args, **kwargs):
            nonlocal build_number
            if name == "buildTargets":
                build_number += 1
                pending.extend(
                    [
                        event("BuildStarted"),
                        event("BuildStarted"),
                        event(
                            "runQueueTaskStarted",
                            taskfile="/vendor_git.bb",
                            taskname="do_compile",
                            stats=SimpleNamespace(
                                completed=3, total=10, active=1, failed=0
                            ),
                        ),
                        event("BuildCompleted"),
                    ]
                )
            elif name == "getRecipes":
                self.assertFalse(
                    kwargs.get("handle_events", True),
                    "Tinfoil must not drain and discard native events",
                )
                queried.append(build_number)
                if build_number == 3:
                    raise RuntimeError("fixture cache unavailable")
                return [(f"actual-pn-{build_number}", ["/vendor_git.bb"])]
            else:
                self.fail(f"unexpected per-event command: {name}")

        connection = object.__new__(bridge.TinfoilConnection)
        connection.active = False
        connection.force_active = False
        connection.recipes_parsed = False
        connection.task_recipe_identities = None
        connection.tinfoil = SimpleNamespace(
            run_command=command,
            set_event_mask=lambda _: None,
            wait_event=lambda _: pending.pop(0) if pending else None,
        )
        for number in [1, 2, 3]:
            connection.start_build(["image"], "build")
            self.assertEqual(
                len(queried),
                number - 1,
                "no cache query before build metadata is ready",
            )
            with patch.object(bridge.sys.stderr, "write"):
                rows = [
                    bridge.normalize_event(raw) for raw in connection.drain_events()
                ]
            if number < 3:
                self.assertEqual(rows[2]["recipe"], f"actual-pn-{number}")
            else:
                self.assertEqual(rows[2]["type"], "task_stats")
                self.assertNotIn("recipe", rows[2])
        self.assertEqual(
            queried, [1, 2, 3], "duplicate starts do not repeat metadata lookup"
        )

    def test_recipe_inventory_chunks_preserve_all_records_with_bounded_frames(
        self,
    ) -> None:
        spec = importlib.util.spec_from_file_location("yoctui_inventory_test", BRIDGE)
        assert spec is not None and spec.loader is not None
        bridge = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(bridge)
        recipes = [
            {"name": f"recipe-{i}", "file": "/" + "ü" * 110} for i in range(6000)
        ]
        emitted = []
        with patch.object(
            bridge,
            "emit",
            side_effect=lambda event, correlation: emitted.append((event, correlation)),
        ):
            bridge.handle(
                {"type": "list_recipes", "filter": None, "chunked": True},
                "inventory-7",
                SimpleNamespace(
                    recipes=lambda _: recipes, compatibility_generation=None
                ),
            )
        self.assertGreater(len(emitted), 1)
        merged: list[dict[str, str]] = []
        for index, (event, correlation) in enumerate(emitted):
            self.assertEqual(correlation, "inventory-7")
            self.assertEqual(event["type"], "recipes_chunk")
            self.assertEqual(event["offset"], len(merged))
            self.assertEqual(event["total"], len(recipes))
            self.assertEqual(event["complete"], index == len(emitted) - 1)
            self.assertLess(
                len(
                    json.dumps(
                        event, ensure_ascii=False, separators=(",", ":")
                    ).encode()
                ),
                512 * 1024,
            )
            merged.extend(event["recipes"])
        self.assertEqual(merged, recipes)

    def test_recipe_inventory_legacy_empty_and_resource_errors(self) -> None:
        spec = importlib.util.spec_from_file_location(
            "yoctui_inventory_limits_test", BRIDGE
        )
        assert spec is not None and spec.loader is not None
        bridge = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(bridge)
        with patch.object(bridge, "emit") as emit:
            bridge.emit_recipe_inventory([], "empty", False)
            self.assertEqual(emit.call_args.args[0], {"type": "recipes", "recipes": []})
            bridge.emit_recipe_inventory([], "empty", True)
            self.assertTrue(emit.call_args.args[0]["complete"])
            recipes: list[dict[str, str]]
            for recipes in (
                [{"name": ""}] * 16385,
                [{"name": "x" * (512 * 1024)}],
                [{"name": "x" * 400000}] * 9,
            ):
                emit.reset_mock()
                bridge.emit_recipe_inventory(recipes, "limit", True)
                self.assertEqual(emit.call_count, 1)
                self.assertEqual(emit.call_args.args[0]["type"], "command_failed")

    def test_backend_probe_uses_real_api_shape_and_only_pings(self) -> None:
        spec = importlib.util.spec_from_file_location("yoctui_probe_test", BRIDGE)
        assert spec is not None and spec.loader is not None
        bridge = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(bridge)
        calls: list[str] = []

        def run_command(name: str) -> str:
            calls.append(name)
            self.assertEqual(name, "ping", "probe must not execute work")
            return "Still alive!"

        def noop(*args: object, **kwargs: object) -> None:
            pass

        tinfoil = SimpleNamespace(
            run_command=run_command,
            config_data=SimpleNamespace(getVar=lambda name: str(Path.cwd())),
            parse_recipes=noop,
            parse_recipe=noop,
            parse_recipe_file=noop,
            get_file_appends=noop,
            set_event_mask=noop,
            wait_event=noop,
            server_connection=SimpleNamespace(
                events=SimpleNamespace(waitEvent=noop),
                connection=SimpleNamespace(terminateServer=noop),
            ),
        )
        commands = SimpleNamespace(
            CommandsSync=SimpleNamespace(
                ping=noop,
                getLayerPriorities=noop,
                getRecipes=noop,
                getRecipeVersions=noop,
                parseRecipeFile=noop,
                stateForceShutdown=noop,
                setEventMask=noop,
            ),
            CommandsAsync=SimpleNamespace(buildTargets=noop, generateDepTreeEvent=noop),
        )
        with patch.object(bridge.importlib, "import_module", return_value=commands):
            capabilities = bridge.tinfoil_probe_capabilities(tinfoil)
            for expected in [
                "workspace",
                "recipes",
                "layers",
                "build",
                "cancel",
                "native_events",
                "dependency_graph",
            ]:
                self.assertIn(expected, capabilities)
            commands.CommandsAsync.buildTargets = None
            self.assertNotIn("build", bridge.tinfoil_probe_capabilities(tinfoil))
            tinfoil.wait_event = None
            self.assertNotIn(
                "native_events", bridge.tinfoil_probe_capabilities(tinfoil)
            )
            tinfoil.run_command = lambda _: "wrong server"
            with self.assertRaises(bridge.CompatibilityError):
                bridge.tinfoil_probe_capabilities(tinfoil)
        self.assertEqual(calls, ["ping", "ping", "ping"])

    def test_rootfs_sources_are_exact_expanded_bitbake_paths(self) -> None:
        result = run_bridge(
            b'{"protocol_version":1,"sequence":1,"message":{"type":"get_rootfs_sources","recipe":"core-image-minimal"}}',
            environment={
                "IMAGE_MANIFEST": "/build/tmp/deploy/images/qemux86-64/core-image-minimal.rootfs.manifest",
                "PKGDATA_DIR": "/build/tmp/pkgdata/qemux86-64",
                "IMAGE_ROOTFS": "/build/tmp/work/qemux86-64/core-image-minimal/1.0-r0/rootfs",
            },
        )
        message = json.loads(result.stdout)["message"]
        self.assertEqual(message["type"], "rootfs_sources")
        self.assertEqual(message["recipe"], "core-image-minimal")
        self.assertTrue(message["image_manifest"].endswith(".rootfs.manifest"))
        self.assertEqual(message["pkgdata_dir"], "/build/tmp/pkgdata/qemux86-64")
        self.assertTrue(message["image_rootfs"].endswith("/rootfs"))

    def test_rootfs_sources_reject_non_exact_recipe_identity(self) -> None:
        result = run_bridge(
            b'{"protocol_version":1,"sequence":1,"message":{"type":"get_rootfs_sources","recipe":"core-image-*"}}'
        )
        message = json.loads(result.stdout)["message"]
        self.assertEqual(message["type"], "command_failed")
        self.assertEqual(message["code"], "invalid_request")

    def test_tinfoil_cancellation_uses_bounded_cooker_shutdown(self) -> None:
        spec = importlib.util.spec_from_file_location("yoctui_bridge_test", BRIDGE)
        if spec is None or spec.loader is None:
            self.fail(f"could not load bridge module from {BRIDGE}")
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
