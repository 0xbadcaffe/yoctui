def handle(command, correlation_id, adapter):
    kind = command.get("type") if isinstance(command, dict) else None
    required_capabilities = {
        "inspect_workspace": ("bitbake.workspace_inspection",),
        "list_recipes": ("bitbake.recipe_inventory",),
        "list_layers": ("bitbake.layer_inventory",),
        "get_variable": ("bitbake.getvar",),
        "get_rootfs_sources": ("bitbake.getvar",),
        "get_dependencies": ("bitbake.recipe_dependencies",),
        "get_dependency_graph": ("bitbake.dependency_graph",),
        "get_recipe_sources": ("bitbake.recipe_sources",),
        "get_recipe_metadata": ("bitbake.recipe_metadata",),
        "get_layer_relationships": ("bitbake.layer_relationships",),
        "start_build": ("bitbake.build", "bitbake.native_events"),
        "cancel_build": ("bitbake.cancellation",),
        "terminate_server": ("bitbake.server_socket",),
    }
    if adapter.compatibility_generation is not None:
        missing = [
            capability
            for capability in required_capabilities.get(kind, ())
            if capability not in adapter.negotiated_capabilities
        ]
        if missing:
            error(
                "compatibility_unavailable",
                f"{kind} requires negotiated capability {missing[0]}",
                correlation_id,
            )
            return True
    if kind == "hello":
        emit(
            {
                "type": "hello_ack",
                "bitbake_version": adapter.version,
                "compatibility_generation": adapter.compatibility_generation,
                "capabilities": sorted(adapter.negotiated_capabilities),
            },
            correlation_id,
        )
    elif kind == "inspect_workspace":
        try:
            workspace = adapter.workspace()
        except ServerUnavailable as exc:
            error("bitbake_server_unavailable", str(exc), correlation_id)
            return True
        emit(workspace, correlation_id)
    elif kind == "start_build":
        targets = command.get("targets")
        if (
            not isinstance(targets, list)
            or not targets
            or not all(isinstance(t, str) and t for t in targets)
        ):
            error(
                "invalid_request",
                "start_build requires non-empty string targets",
                correlation_id,
            )
        else:
            try:
                force = command.get("force", False)
                if not isinstance(force, bool):
                    error(
                        "invalid_request",
                        "start_build force must be a boolean",
                        correlation_id,
                    )
                    return True
                native_events = adapter.start_build(targets, command.get("task"), force)
            except ServerUnavailable as exc:
                error("bitbake_server_unavailable", str(exc), correlation_id)
            else:
                adapter.build_correlation_id = correlation_id
                if not native_events:
                    emit({"type": "build_started"}, correlation_id)
                for event in adapter.mock_events():
                    emit(event, correlation_id)
    elif kind == "list_recipes":
        filter_value = command.get("filter")
        chunked = command.get("chunked", False)
        if not isinstance(chunked, bool):
            error(
                "invalid_request",
                "list_recipes chunked must be boolean",
                correlation_id,
            )
            return True
        if filter_value is not None and not isinstance(filter_value, str):
            error(
                "invalid_request",
                "list_recipes filter must be a string",
                correlation_id,
            )
            return True
        try:
            recipes = adapter.recipes(filter_value)
        except ServerUnavailable as exc:
            error("bitbake_server_unavailable", str(exc), correlation_id)
            return True
        if recipes is None:
            recipes = bitbake_layer_recipes(filter_value)
            if recipes is None:
                recipes = bitbake_recipes(filter_value)
            if recipes is None:
                recipes = configured_recipes()
                if filter_value is not None:
                    recipes = [
                        recipe
                        for recipe in recipes
                        if filter_value.lower() in recipe["name"].lower()
                    ]
        emit_recipe_inventory(recipes, correlation_id, chunked)
    elif kind == "list_layers":
        try:
            layers = adapter.layers()
        except ServerUnavailable as exc:
            error("bitbake_server_unavailable", str(exc), correlation_id)
            return True
        if layers is None:
            layers = bitbake_layers()
        emit(
            {
                "type": "layers",
                "layers": configured_layers() if layers is None else layers,
            },
            correlation_id,
        )
    elif kind == "get_variable":
        name = command.get("name")
        recipe = command.get("recipe")
        if (
            not isinstance(name, str)
            or not name
            or (recipe is not None and not isinstance(recipe, str))
        ):
            error(
                "invalid_request",
                "get_variable requires a variable name and optional recipe name",
                correlation_id,
            )
        else:
            try:
                variable = adapter.variable(name, recipe)
            except ServerUnavailable as exc:
                error("bitbake_server_unavailable", str(exc), correlation_id)
                return True
            if variable is None:
                variable = {
                    "value": os.environ.get(name),
                    "provenance": configured_variable_provenance().get(name),
                }
            emit(
                {
                    "type": "variable",
                    "name": name,
                    "recipe": recipe,
                    **variable,
                },
                correlation_id,
            )
    elif kind == "get_rootfs_sources":
        recipe = command.get("recipe")
        if not isinstance(recipe, str) or not re.fullmatch(r"[A-Za-z0-9_.+-]+", recipe):
            error(
                "invalid_request",
                "get_rootfs_sources requires an exact image recipe name",
                correlation_id,
            )
        else:
            try:
                sources = adapter.rootfs_sources(recipe)
            except ServerUnavailable as exc:
                error("bitbake_server_unavailable", str(exc), correlation_id)
            else:
                emit(
                    {"type": "rootfs_sources", "recipe": recipe, **sources},
                    correlation_id,
                )
    elif kind == "get_dependency_graph":
        recipe = command.get("recipe")
        if not isinstance(recipe, str) or not recipe:
            error(
                "invalid_request",
                "get_dependency_graph requires a recipe name",
                correlation_id,
            )
        else:
            try:
                graph = adapter.dependency_graph(recipe)
            except ServerUnavailable as exc:
                error("bitbake_server_unavailable", str(exc), correlation_id)
            else:
                emit(
                    {
                        "type": "dependency_graph",
                        "data": graph,
                    },
                    correlation_id,
                )
    elif kind == "get_dependencies":
        recipe = command.get("recipe")
        if not isinstance(recipe, str) or not recipe:
            error(
                "invalid_request",
                "get_dependencies requires a recipe name",
                correlation_id,
            )
        else:
            try:
                dependencies = adapter.dependencies(recipe)
            except ServerUnavailable as exc:
                error("bitbake_server_unavailable", str(exc), correlation_id)
            else:
                emit(
                    {
                        "type": "dependencies",
                        "recipe": recipe,
                        **dependencies,
                    },
                    correlation_id,
                )
    elif kind == "get_recipe_sources":
        recipe = command.get("recipe")
        if not isinstance(recipe, str) or not recipe:
            error(
                "invalid_request",
                "get_recipe_sources requires a recipe name",
                correlation_id,
            )
        else:
            try:
                paths = adapter.recipe_sources(recipe)
            except ServerUnavailable as exc:
                error("bitbake_server_unavailable", str(exc), correlation_id)
            else:
                emit(
                    {"type": "recipe_sources", "recipe": recipe, "paths": paths},
                    correlation_id,
                )
    elif kind == "get_recipe_metadata":
        recipe = command.get("recipe")
        if not isinstance(recipe, str) or not recipe:
            error(
                "invalid_request",
                "get_recipe_metadata requires a recipe name",
                correlation_id,
            )
        else:
            try:
                metadata = adapter.recipe_metadata(recipe)
            except ServerUnavailable as exc:
                error("bitbake_server_unavailable", str(exc), correlation_id)
            else:
                emit(
                    {"type": "recipe_metadata", "data": metadata},
                    correlation_id,
                )
    elif kind == "get_layer_relationships":
        try:
            layers = adapter.layer_relationships()
        except ServerUnavailable as exc:
            error("bitbake_server_unavailable", str(exc), correlation_id)
        else:
            emit({"type": "layer_relationships", "layers": layers}, correlation_id)
    elif kind == "cancel_build":
        try:
            adapter.cancel_build()
        except ServerUnavailable as exc:
            error("bitbake_server_unavailable", str(exc), correlation_id)
        else:
            # The cooker shutdown command acknowledges cancellation, but some
            # BitBake releases do not subsequently deliver BuildCompleted to
            # this Tinfoil event stream.  The accepted shutdown request is the
            # authoritative terminal boundary for Yoctui.  Stop polling here
            # so delayed native records cannot resurrect the cancelled build;
            # the daemon then closes the server through terminate_server.
            build_correlation_id = adapter.build_correlation_id or correlation_id
            adapter.build_active = False
            adapter.native_event_iterator = None
            adapter.task_identities_by_pid.clear()
            emit(
                {"type": "build_completed", "success": False, "exit_code": 1},
                build_correlation_id,
            )
            adapter.build_correlation_id = None
    elif kind == "shutdown":
        emit({"type": "bridge_shutdown"}, correlation_id)
        return False
    elif kind == "terminate_server":
        try:
            adapter.server().terminate_server()
        except (ServerUnavailable, RuntimeError) as exc:
            error("bitbake_server_unavailable", str(exc), correlation_id)
        else:
            adapter.connection = None
            adapter.build_active = False
            emit({"type": "server_terminated"}, correlation_id)
            return False
    else:
        error("unknown_command", f"unknown command: {kind!r}", correlation_id)
    return True
