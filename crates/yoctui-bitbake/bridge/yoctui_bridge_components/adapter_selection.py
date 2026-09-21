class EnvironmentAdapter(BitBakeAdapter):
    def __init__(self, version=None):
        super().__init__(version)


def select_adapter(version=None, implementation=None):
    module = None
    if version is None:
        try:
            import bb as module  # type: ignore[import-not-found]

            version = getattr(module, "__version__", None)
        except ImportError:
            version = bitbake_version()
    if implementation is None:
        # Backward-compatible bridge clients have no daemon snapshot. Probe
        # the initialized module shape directly; never choose by version.
        return (
            BitBakeAdapter(version, module)
            if module is not None
            else EnvironmentAdapter(version)
        )
    if not (
        implementation.startswith("tinfoil.")
        or implementation.startswith("bitbake.server_socket")
    ):
        raise CompatibilityError(
            f"daemon selected an unsupported BitBake API implementation: {implementation!r}"
        )
    if module is None:
        raise CompatibilityError(
            "daemon selected a BitBake API implementation but the initialized environment does not expose the bb module"
        )
    return BitBakeAdapter(version, module)


def configure_compatibility(command):
    compatibility = command.get("compatibility")
    if compatibility is None:
        adapter = select_adapter()
        return adapter, None, []
    if not isinstance(compatibility, dict):
        raise CompatibilityError("bridge compatibility payload must be an object")
    generation = compatibility.get("generation")
    build_directory = compatibility.get("build_directory")
    capabilities = compatibility.get("capabilities")
    if (
        not isinstance(generation, int)
        or isinstance(generation, bool)
        or generation <= 0
    ):
        raise CompatibilityError("bridge compatibility generation must be positive")
    if not isinstance(build_directory, str) or not os.path.isabs(build_directory):
        raise CompatibilityError(
            "bridge compatibility build directory must be absolute"
        )
    if os.path.realpath(build_directory) != os.path.realpath(os.getcwd()):
        raise CompatibilityError(
            "bridge compatibility belongs to another build directory"
        )
    if not isinstance(capabilities, list) or len(capabilities) > 64:
        raise CompatibilityError(
            "bridge compatibility capability list is invalid or oversized"
        )
    seen = set()
    direct_implementations = {
        "bitbake.workspace_inspection": "tinfoil.workspace",
        "bitbake.recipe_inventory": "tinfoil.recipes",
        "bitbake.recipe_dependencies": "tinfoil.dependencies",
        "bitbake.recipe_sources": "tinfoil.recipe_sources",
        "bitbake.recipe_metadata": "tinfoil.recipe_metadata",
        "bitbake.layer_inventory": "tinfoil.layers",
        "bitbake.layer_relationships": "tinfoil.layer_relationships",
        "bitbake.build": "tinfoil.build",
        "bitbake.cancellation": "tinfoil.cancel",
        "bitbake.task_list": "tinfoil.tasks",
        "bitbake.dependency_graph": "tinfoil.dependency_graph",
        "bitbake.getvar": "tinfoil.getvar",
        "bitbake.variable_history": "tinfoil.variable_history",
        "bitbake.server_socket": "bitbake.server_socket",
        "bitbake.native_events": "tinfoil.native_events",
    }
    for capability in capabilities:
        if (
            not isinstance(capability, dict)
            or not isinstance(capability.get("id"), str)
            or not isinstance(capability.get("implementation"), str)
            or capability["id"] in seen
        ):
            raise CompatibilityError("bridge compatibility capability entry is invalid")
        expected = direct_implementations.get(capability["id"])
        if expected is None or not (
            capability["implementation"] == expected
            or capability["implementation"].startswith("tinfoil.adapter.")
        ):
            raise CompatibilityError(
                f"bridge compatibility implementation does not authorize {capability['id']}"
            )
        seen.add(capability["id"])
    implementation = next(
        (
            item["implementation"]
            for item in capabilities
            if item["implementation"].startswith("tinfoil.adapter.")
        ),
        next((item["implementation"] for item in capabilities), None),
    )
    adapter = select_adapter(implementation=implementation)
    adapter.compatibility_generation = generation
    return adapter, generation, adapter.negotiate(capabilities) if capabilities else []
