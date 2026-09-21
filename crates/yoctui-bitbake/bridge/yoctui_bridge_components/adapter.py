class BitBakeAdapter:
    def __init__(self, version, module=None):
        self.version = version
        self.module = module
        self.connection = None
        self.build_correlation_id = None
        self.build_active = False
        self.task_identities_by_pid = {}
        self.native_event_iterator = None
        self.compatibility_generation = None
        self.negotiated_capabilities = set()

    def negotiate(self, requested):
        """Directly retain only API behavior exposed by this connection."""
        connection = self.server()
        operations = {
            "bitbake.workspace_inspection": ("inspect_workspace",),
            "bitbake.recipe_inventory": ("list_recipes",),
            "bitbake.recipe_dependencies": ("get_dependencies",),
            "bitbake.recipe_sources": ("get_recipe_sources",),
            "bitbake.recipe_metadata": ("get_recipe_metadata",),
            "bitbake.layer_inventory": ("list_layers",),
            "bitbake.layer_relationships": ("get_layer_relationships",),
            "bitbake.build": ("start_build",),
            "bitbake.cancellation": ("cancel_build",),
            "bitbake.task_list": ("get_recipe_metadata",),
            "bitbake.dependency_graph": ("get_dependency_graph",),
            "bitbake.getvar": ("get_variable",),
            "bitbake.variable_history": ("get_variable",),
            "bitbake.server_socket": ("terminate_server",),
            "bitbake.native_events": ("drain_events",),
        }
        negotiated = set()
        for capability in requested:
            capability_id = capability["id"]
            methods = operations.get(capability_id)
            if methods and all(
                callable(getattr(connection, name, None)) for name in methods
            ):
                if capability_id != "bitbake.native_events" or bool(
                    getattr(connection, "native_event_stream", False)
                ):
                    negotiated.add(capability_id)
        self.negotiated_capabilities = negotiated
        return sorted(negotiated)

    def workspace(self):
        operation = self.optional_server_operation("inspect_workspace")
        if operation is None:
            return workspace_data(self.version)
        try:
            response = operation()
        except Exception as exc:
            raise ServerUnavailable(
                f"could not inspect the BitBake workspace from the server: {exc}"
            )
        return {"type": "workspace", "data": typed_workspace(response)}

    def server(self):
        if self.connection is not None:
            return self.connection
        if self.module is not None and getattr(self.module, "__path__", None):
            try:
                self.connection = TinfoilConnection(self.module)
                return self.connection
            except (ImportError, AttributeError):
                pass
            except Exception as exc:
                raise ServerUnavailable(f"could not initialize BitBake Tinfoil: {exc}")
        server = getattr(self.module, "server", None) if self.module else None
        connector = getattr(server, "connect", None)
        if not callable(connector):
            raise ServerUnavailable(
                "no supported BitBake server connector is available; start BitBake and expose bb.server.connect"
            )
        try:
            self.connection = connector()
            return self.connection
        except Exception as exc:
            raise ServerUnavailable(f"could not connect to the BitBake server: {exc}")

    def start_build(self, targets, task, force=False):
        connection = self.server()
        self.task_identities_by_pid.clear()
        self.native_event_iterator = None
        operation = getattr(connection, "start_build", None)
        if not callable(operation):
            raise ServerUnavailable(
                "connected BitBake server does not provide start_build"
            )
        try:
            try:
                operation(targets, task, force)
            except TypeError:
                if force:
                    raise ServerUnavailable(
                        "connected BitBake server does not support forced task execution"
                    )
                operation(targets, task)
        except Exception as exc:
            raise ServerUnavailable(f"could not start the BitBake build: {exc}")
        self.build_active = True
        return bool(getattr(connection, "native_event_stream", False))

    def cancel_build(self):
        connection = self.server()
        operation = getattr(connection, "cancel_build", None)
        if not callable(operation):
            raise ServerUnavailable(
                "connected BitBake server does not provide cancel_build"
            )
        try:
            operation()
        except Exception as exc:
            raise ServerUnavailable(f"could not cancel the BitBake build: {exc}")
        return bool(getattr(connection, "native_event_stream", False))

    def shutdown(self):
        if self.connection is None:
            return
        operation = getattr(self.connection, "shutdown", None)
        if callable(operation):
            operation()
        self.connection = None
        self.build_active = False
        self.task_identities_by_pid.clear()
        self.native_event_iterator = None

    def optional_server_operation(self, name):
        if self.module is None:
            return None
        try:
            connection = self.server()
        except ServerUnavailable:
            return None
        operation = getattr(connection, name, None)
        return operation if callable(operation) else None

    def variable(self, name, recipe):
        """Query a server-provided effective value without interpreting metadata."""
        operation = self.optional_server_operation("get_variable")
        if operation is None:
            return None
        try:
            response = operation(name, recipe)
        except Exception as exc:
            raise ServerUnavailable(
                f"could not query {name} from the BitBake server: {exc}"
            )
        if response is None or isinstance(response, str):
            return {"value": response, "provenance": None}
        if isinstance(response, dict):
            value = response.get("value")
            provenance = response.get("provenance")
            unexpanded_value = response.get("unexpanded_value")
            operations = response.get("operations", [])
            active_overrides = response.get("active_overrides", [])
            scope = response.get("recipe", recipe)
            if (
                (value is None or isinstance(value, str))
                and (provenance is None or isinstance(provenance, str))
                and (unexpanded_value is None or isinstance(unexpanded_value, str))
            ):
                if scope is not None and not isinstance(scope, str):
                    raise ServerUnavailable(
                        f"BitBake server returned an invalid variable scope for {name}"
                    )
                if not isinstance(active_overrides, list) or not all(
                    isinstance(item, str) for item in active_overrides
                ):
                    raise ServerUnavailable(
                        f"BitBake server returned invalid active overrides for {name}"
                    )
                if not isinstance(operations, list) or not all(
                    isinstance(item, dict)
                    and isinstance(item.get("operation"), str)
                    and (item.get("file") is None or isinstance(item.get("file"), str))
                    and (
                        item.get("line") is None
                        or (
                            isinstance(item.get("line"), int)
                            and not isinstance(item.get("line"), bool)
                        )
                    )
                    and (
                        item.get("value") is None or isinstance(item.get("value"), str)
                    )
                    for item in operations
                ):
                    raise ServerUnavailable(
                        f"BitBake server returned invalid variable operations for {name}"
                    )
                return {
                    "recipe": scope,
                    "value": value,
                    "provenance": provenance,
                    "unexpanded_value": unexpanded_value,
                    "operations": operations,
                    "active_overrides": active_overrides,
                }
        raise ServerUnavailable(
            f"BitBake server returned an unsupported variable response for {name}"
        )

    def rootfs_sources(self, recipe):
        operation = self.optional_server_operation("get_rootfs_sources")
        if operation is None:
            return {
                "image_manifest": os.environ.get("IMAGE_MANIFEST"),
                "pkgdata_dir": os.environ.get("PKGDATA_DIR"),
                "image_rootfs": os.environ.get("IMAGE_ROOTFS"),
            }
        try:
            response = operation(recipe)
        except Exception as exc:
            raise ServerUnavailable(
                f"could not query rootfs sources for {recipe} from the BitBake server: {exc}"
            )
        keys = ("image_manifest", "pkgdata_dir", "image_rootfs")
        if not isinstance(response, dict) or any(
            response.get(key) is not None and not isinstance(response.get(key), str)
            for key in keys
        ):
            raise ServerUnavailable(
                "BitBake server returned malformed rootfs source data"
            )
        return {key: response.get(key) for key in keys}

    def recipes(self, filter_value):
        operation = self.optional_server_operation("list_recipes")
        if operation is None:
            return None
        try:
            response = operation(filter_value)
        except Exception as exc:
            raise ServerUnavailable(
                f"could not list recipes from the BitBake server: {exc}"
            )
        return typed_recipes(response)

    def layers(self):
        operation = self.optional_server_operation("list_layers")
        if operation is None:
            return None
        try:
            response = operation()
        except Exception as exc:
            raise ServerUnavailable(
                f"could not list layers from the BitBake server: {exc}"
            )
        return typed_layers(response)

    def dependencies(self, recipe):
        """Return server-resolved build and runtime dependencies for one recipe."""
        operation = self.optional_server_operation("get_dependencies")
        if operation is None:
            raise ServerUnavailable(
                "connected BitBake server does not provide get_dependencies; authoritative dependency inspection is unavailable"
            )
        try:
            response = operation(recipe)
        except Exception as exc:
            raise ServerUnavailable(
                f"could not inspect dependencies for {recipe} from the BitBake server: {exc}"
            )
        return typed_dependencies(response)

    def dependency_graph(self, recipe):
        """Return a bounded typed graph from an authoritative server operation."""
        operation = self.optional_server_operation("get_dependency_graph")
        if operation is not None:
            try:
                response = operation(recipe)
            except Exception as exc:
                raise ServerUnavailable(
                    f"could not inspect the dependency graph for {recipe} from the BitBake server: {exc}"
                )
            return typed_dependency_graph(response, recipe)

        dependencies = self.dependencies(recipe)
        edges = [
            {
                "from": {"recipe": recipe},
                "to": {"recipe": dependency},
                "kind": kind,
            }
            for kind, values in (
                ("build", dependencies["build"]),
                ("runtime", dependencies["runtime"]),
            )
            for dependency in values
        ]
        return typed_dependency_graph(
            {
                "root": {"recipe": recipe},
                "nodes": [],
                "edges": edges,
                "limitations": [
                    "Legacy server supplied direct recipe edges only; task dependencies are unavailable."
                ],
            },
            recipe,
        )

    def recipe_sources(self, recipe):
        operation = self.optional_server_operation("get_recipe_sources")
        if operation is None:
            raise ServerUnavailable(
                "connected BitBake server does not provide get_recipe_sources; authoritative recipe metadata paths are unavailable"
            )
        try:
            response = operation(recipe)
        except Exception as exc:
            raise ServerUnavailable(
                f"could not inspect metadata paths for {recipe} from the BitBake server: {exc}"
            )
        if not isinstance(response, list) or not all(
            isinstance(path, str) for path in response
        ):
            raise ServerUnavailable(
                "BitBake server returned malformed recipe source data"
            )
        return response

    def recipe_metadata(self, recipe):
        operation = self.optional_server_operation("get_recipe_metadata")
        if operation is None:
            raise ServerUnavailable(
                "connected BitBake server does not provide get_recipe_metadata; authoritative recipe details are unavailable"
            )
        try:
            return typed_recipe_metadata(operation(recipe))
        except Exception as exc:
            raise ServerUnavailable(
                f"could not inspect metadata for {recipe} from the BitBake server: {exc}"
            )

    def layer_relationships(self):
        operation = self.optional_server_operation("get_layer_relationships")
        if operation is None:
            raise ServerUnavailable(
                "connected BitBake server does not provide get_layer_relationships; authoritative layer relationships are unavailable"
            )
        try:
            return typed_layer_relationships(operation())
        except Exception as exc:
            raise ServerUnavailable(
                f"could not inspect layer relationships from the BitBake server: {exc}"
            )

    def native_events(self):
        """Return one bounded event slice without monopolizing command input."""
        if self.connection is None:
            return []
        drain = getattr(self.connection, "drain_events", None)
        if not callable(drain):
            return []
        if self.native_event_iterator is None:
            try:
                drained = drain()
            except Exception as exc:
                return [
                    {
                        "type": "warning",
                        "message": f"could not drain BitBake server events: {exc}",
                    }
                ]
            if drained is None:
                return []
            try:
                self.native_event_iterator = iter(drained)
            except TypeError:
                return [
                    {
                        "type": "warning",
                        "message": "BitBake server drain_events result is not iterable",
                    }
                ]

        events = []
        for _ in range(MAX_NATIVE_EVENTS_PER_POLL):
            try:
                raw = next(self.native_event_iterator)
            except StopIteration:
                self.native_event_iterator = None
                break
            event = normalize_event(raw, self.task_identities_by_pid)
            if not event:
                continue
            kind = event.get("type")
            if kind == "build_completed":
                if not self.build_active:
                    continue
                self.build_active = False
                self.task_identities_by_pid.clear()
                self.native_event_iterator = None
                events.append(event)
                break
            if not self.build_active and kind in {
                "build_started",
                "parse_progress",
                "task_queued",
                "task_stats",
                "task_started",
                "task_progress",
                "task_completed",
            }:
                continue
            events.append(event)
        return events

    def mock_events(self):
        try:
            raw = json.loads(os.environ.get("YOCTUI_MOCK_EVENTS_JSON", "[]"))
        except json.JSONDecodeError:
            return []
        if not isinstance(raw, list):
            return []
        return [
            event
            for event in (
                normalize_event(item, self.task_identities_by_pid) for item in raw
            )
            if event
        ]
