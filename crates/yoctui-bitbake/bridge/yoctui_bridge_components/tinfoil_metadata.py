def _tinfoil_fast_recipe_data(self, recipe):
    """Inspect one unambiguous configured recipe without parsing every recipe.

    This opt-in path uses current BBFILES/BBMASK, applies BitBake's bbappends,
    and validates PN/PROVIDES. Ambiguous selection retains normal provider lookup.
    A datastore is cached only for this short-lived inspection connection.
    """
    if os.environ.get("YOCTUI_BRIDGE_FAST_RECIPE_INSPECTION") != "1":
        return None
    cache = getattr(self, "inspection_datastores", {})
    if recipe in cache:
        return cache[recipe]
    config = self.tinfoil.config_data
    target = recipe
    if recipe.startswith("virtual/"):
        target = config.getVar("PREFERRED_PROVIDER_" + recipe)
    if not target or not re.fullmatch(r"[A-Za-z0-9+._-]+", target):
        return None
    if config.getVar("BBMULTICONFIG"):
        return None
    import glob

    masks = (config.getVar("BBMASK") or "").split()
    try:
        mask = (
            re.compile("|".join(part.removeprefix("|") for part in masks))
            if masks
            else None
        )
    except re.error:
        return None
    candidates = set()
    count = 0
    # Tinfoil's prepared configuration has already expanded active
    # BBFILES_DYNAMIC collections into BBFILES, just as BitBake does.
    for pattern in (config.getVar("BBFILES") or "").split():
        if os.path.isdir(pattern):
            return None  # Directory-recursive selection belongs to BitBake.
        for path in glob.iglob(pattern):
            count += 1
            if count > MAX_RECIPE_INVENTORY_RECORDS:
                return None
            name = os.path.basename(path)
            if not (
                name == target + ".bb"
                or name.startswith(target + "_")
                and name.endswith(".bb")
            ):
                continue
            if mask and mask.search(path):
                continue
            candidates.add(os.path.realpath(path))
    if len(candidates) != 1:
        return None
    path = candidates.pop()
    datastore = self.tinfoil.parse_recipe_file(path)
    if datastore.getVar("PN") != target:
        return None
    if recipe != target and recipe not in (datastore.getVar("PROVIDES") or "").split():
        return None
    if config.getVar("REQUIRED_VERSION_" + target) or config.getVar(
        "REQUIRED_VERSION:" + target
    ):
        return None
    self.recipe_files[recipe] = path
    self.inspection_datastores = cache
    cache[recipe] = datastore
    return datastore


def _tinfoil_get_variable(self, name, recipe):
    datastore = self.tinfoil.config_data
    if recipe is not None:
        datastore = self._fast_recipe_data(recipe)
        if datastore is None:
            self._ensure_recipes()
            datastore = self.tinfoil.parse_recipe(recipe)
    value = datastore.getVar(name)
    unexpanded_value = datastore.getVar(name, False)
    operations = self._variable_operations(datastore, name)
    active_overrides = [
        override
        for override in str(datastore.getVar("OVERRIDES") or "").split(":")
        if override
    ]
    return {
        "recipe": recipe,
        "value": None if value is None else str(value),
        "unexpanded_value": None if unexpanded_value is None else str(unexpanded_value),
        "provenance": next(
            (
                f"{operation['file']}:{operation['line']}"
                if operation["line"] is not None
                else operation["file"]
                for operation in reversed(operations)
                if operation["file"] is not None
            ),
            None,
        ),
        "operations": operations,
        "active_overrides": active_overrides,
    }


def _tinfoil_get_rootfs_sources(self, recipe):
    """Return only BitBake-expanded paths for one exact image recipe."""
    self._ensure_recipes()
    datastore = self.tinfoil.parse_recipe(recipe)
    return {
        name.lower(): (
            None if datastore.getVar(name) is None else str(datastore.getVar(name))
        )
        for name in ("IMAGE_MANIFEST", "PKGDATA_DIR", "IMAGE_ROOTFS")
    }


def _tinfoil_get_dependencies(self, recipe):
    self._ensure_recipes()
    datastore = self.tinfoil.parse_recipe(recipe)
    build = (datastore.getVar("DEPENDS") or "").split()
    runtime = (
        datastore.getVar(f"RDEPENDS:{recipe}") or datastore.getVar("RDEPENDS") or ""
    ).split()
    return {"build": build, "runtime": runtime}


def _tinfoil_get_dependency_graph(self, recipe):
    if self.active:
        raise RuntimeError("dependency graphs are unavailable during an active build")
    self._ensure_recipes()
    event_mask = [
        "bb.event.DepTreeGenerated",
        "bb.command.CommandCompleted",
        "bb.command.CommandFailed",
        "bb.command.CommandExit",
        "logging.LogRecord",
    ]
    self.tinfoil.set_event_mask(event_mask)
    default_task = self.tinfoil.config_data.getVar("BB_DEFAULT_TASK") or "build"
    graph_data = None
    try:
        self.tinfoil.run_command(
            "generateDepTreeEvent",
            [recipe],
            default_task,
            handle_events=False,
        )
        deadline = time.monotonic() + 120
        while time.monotonic() < deadline:
            event = self.tinfoil.wait_event(0.25)
            if event is None:
                continue
            kind = type(event).__name__
            if kind == "DepTreeGenerated":
                graph_data = getattr(event, "_depgraph", None)
            elif kind in ("CommandFailed", "CommandExit"):
                raise RuntimeError(f"BitBake {kind} while generating dependencies")
            elif kind == "CommandCompleted":
                if graph_data is None:
                    raise RuntimeError(
                        "BitBake completed dependency generation without a graph"
                    )
                return dependency_graph_from_deptree(recipe, graph_data)
        raise RuntimeError("BitBake dependency generation timed out after 120 seconds")
    finally:
        self.tinfoil.set_event_mask(self.EVENT_MASK)


def _tinfoil_preferred_recipe_file(self, recipe):
    if recipe in self.recipe_files:
        return self.recipe_files[recipe]
    try:
        best = self.tinfoil.run_command("findBestProvider", recipe) or ()
    except Exception:
        best = ()
    if isinstance(best, (list, tuple)) and len(best) > 3 and isinstance(best[3], str):
        return best[3]
    providers = self.tinfoil.run_command("findProviders", "") or ()
    preferred = (
        providers[1]
        if isinstance(providers, (list, tuple))
        and len(providers) > 1
        and isinstance(providers[1], dict)
        else {}
    )
    preferred_data = preferred.get(recipe)
    if (
        isinstance(preferred_data, (list, tuple))
        and len(preferred_data) > 1
        and isinstance(preferred_data[1], str)
    ):
        return preferred_data[1]
    recipes = self.tinfoil.run_command("getRecipes", "") or []
    for name, paths in recipes:
        if name == recipe:
            candidates = sorted(path for path in paths if isinstance(path, str))
            if candidates:
                return candidates[0]
    raise RuntimeError(f"no provider file is available for {recipe}")


def _tinfoil_get_recipe_sources(self, recipe):
    self._ensure_recipes()
    recipe_file = self._preferred_recipe_file(recipe)
    appends = self.tinfoil.get_file_appends(recipe_file) or []
    return [recipe_file, *appends]


def _tinfoil_get_recipe_metadata(self, recipe):
    datastore = self._fast_recipe_data(recipe)
    if datastore is None:
        self._ensure_recipes()
    recipe_file = self._preferred_recipe_file(recipe)
    appends = list(self.tinfoil.get_file_appends(recipe_file) or [])
    if datastore is None:
        datastore = self.tinfoil.parse_recipe_file(recipe_file)
    tasks = datastore.getVar("__BBTASKS") or []
    packages = (datastore.getVar("PACKAGES") or "").split()
    source_uri = (datastore.getVar("SRC_URI") or "").split()
    patch_uris = [
        value
        for value in source_uri
        if value.split(";", 1)[0].endswith((".patch", ".diff"))
    ]
    patches = []
    try:
        fetch = importlib.import_module("bb.fetch2").Fetch(source_uri, datastore)
    except Exception:
        fetch = None
    for value in patch_uris:
        uri = value.split(";", 1)[0]
        if uri.startswith("file://") and fetch is not None:
            try:
                patches.append(fetch.localpath(value))
                continue
            except Exception:
                pass
        patches.append(value)
    return {
        "recipe": recipe,
        "workspace_status": None,
        "build_status": None,
        "tasks": sorted(str(task) for task in tasks),
        "sources": [recipe_file, *appends],
        "patches": patches,
        "packages": packages,
        "history": None,
    }


TinfoilConnection.get_variable = _tinfoil_get_variable
TinfoilConnection._fast_recipe_data = _tinfoil_fast_recipe_data
TinfoilConnection.get_rootfs_sources = _tinfoil_get_rootfs_sources
TinfoilConnection.get_dependencies = _tinfoil_get_dependencies
TinfoilConnection.get_dependency_graph = _tinfoil_get_dependency_graph
TinfoilConnection._preferred_recipe_file = _tinfoil_preferred_recipe_file
TinfoilConnection.get_recipe_sources = _tinfoil_get_recipe_sources
TinfoilConnection.get_recipe_metadata = _tinfoil_get_recipe_metadata
