class TinfoilConnection:
    """Thin production adapter around BitBake's supported Tinfoil API."""

    EVENT_MASK = [
        "bb.event.BuildStarted",
        "bb.event.BuildCompleted",
        "bb.event.ParseStarted",
        "bb.event.ParseProgress",
        "bb.event.ParseCompleted",
        "bb.event.ProcessStarted",
        "bb.event.ProcessProgress",
        "bb.event.ProcessFinished",
        "bb.command.CommandCompleted",
        "bb.command.CommandFailed",
        "bb.command.CommandExit",
        "bb.build.TaskStarted",
        "bb.build.TaskSucceeded",
        "bb.build.TaskFailed",
        "bb.build.TaskFailedSilent",
        "bb.build.TaskProgress",
        "bb.runqueue.runQueueTaskStarted",
        "bb.runqueue.sceneQueueTaskStarted",
        "logging.LogRecord",
    ]

    native_event_stream = True

    def __init__(self, module):
        self.module = module
        self.tinfoil_module = importlib.import_module("bb.tinfoil")
        self.tinfoil = None
        self.recipes_parsed = False
        self.active = False
        self._prepare()

    def _prepare(self):
        self.tinfoil = self.tinfoil_module.Tinfoil(
            output=sys.stderr, tracking=True, setup_logging=True
        )
        self.tinfoil.prepare(config_only=True, quiet=2)
        self.recipes_parsed = False
        self.active = False
        self.recipe_files = {}
        self.task_recipe_identities = None
        self.force_active = False

    def _ensure_recipes(self):
        if not self.recipes_parsed:
            self.tinfoil.parse_recipes()
            self.recipes_parsed = True

    def _reset_for_build(self):
        # Metadata queries parse recipes synchronously. A fresh config-only
        # connection lets the subsequent build expose its real parse events.
        if self.recipes_parsed:
            self.tinfoil.shutdown()
            self._prepare()

    def _variable_operations(self, datastore, name):
        try:
            history = datastore.varhistory.variable(name) or []
        except Exception:
            return []
        operations = []
        for item in history:
            if not isinstance(item, dict) or "flag" in item:
                continue
            path = item.get("file")
            line = item.get("line")
            detail = item.get("detail")
            operations.append(
                {
                    "operation": str(item.get("op") or "set"),
                    "file": path if isinstance(path, str) else None,
                    "line": line
                    if isinstance(line, int) and not isinstance(line, bool)
                    else None,
                    "value": None if detail is None else str(detail),
                }
            )
        return operations

    def _variable_provenance(self, datastore, name):
        sources = []
        for operation in self._variable_operations(datastore, name):
            path = operation["file"]
            line = operation["line"]
            if path is not None:
                sources.append(f"{path}:{line}" if line is not None else path)
        return sources[-1] if sources else None

    def _layers(self):
        priorities = self.tinfoil.run_command("getLayerPriorities") or []
        configured = (self.tinfoil.config_data.getVar("BBLAYERS") or "").split()
        layers = []
        for collection, _pattern, regex, priority in priorities:
            path = next(
                (
                    candidate
                    for candidate in configured
                    if re.match(regex, candidate.rstrip("/") + "/")
                ),
                None,
            )
            if path is None:
                path = regex.removeprefix("^").removesuffix("/")
            layers.append(
                {
                    "name": str(collection),
                    "path": path,
                    "priority": int(priority),
                }
            )
        return layers

    def inspect_workspace(self):
        keys = (
            "MACHINE",
            "DISTRO",
            "BBLAYERS",
            "DL_DIR",
            "SSTATE_DIR",
            "BB_NO_NETWORK",
            "BB_FETCH_PREMIRRORONLY",
            "TMPDIR",
            "DEPLOY_DIR_IMAGE",
            "PKGDATA_DIR",
            "IMAGE_MANIFEST",
            "IMAGE_ROOTFS",
            "WKS_FILE",
            "WKS_FILES",
            "WKS_SEARCH_PATH",
            "WKS_FILES_DIR",
            "PACKAGE_CLASSES",
            "BB_NUMBER_THREADS",
            "PARALLEL_MAKE",
        )
        variables = {}
        provenance = {}
        for key in keys:
            value = self.tinfoil.config_data.getVar(key)
            if value is None and key in ("BB_NO_NETWORK", "BB_FETCH_PREMIRRORONLY"):
                value = "0"
            if value is not None:
                variables[key] = str(value)
            source = self._variable_provenance(self.tinfoil.config_data, key)
            if source is not None:
                provenance[key] = source
        return {
            "build_dir": self.tinfoil.config_data.getVar("TOPDIR"),
            "source_dir": self.tinfoil.config_data.getVar("COREBASE"),
            "variables": variables,
            "variable_provenance": provenance,
            "variable_provenance_chain": {},
            "bitbake_version": getattr(self.module, "__version__", None),
            "release": self.tinfoil.config_data.getVar("DISTRO_VERSION"),
            "layers": self._layers(),
            "recipes": [],
        }

    def list_layers(self):
        return self._layers()

    def _layer_for_path(self, path, layers):
        matches = [
            layer
            for layer in layers
            if path == layer["path"] or path.startswith(layer["path"].rstrip("/") + "/")
        ]
        if not matches:
            return None
        return max(matches, key=lambda layer: len(layer["path"]))["name"]

    def list_recipes(self, filter_value):
        self._ensure_recipes()
        recipes = self.tinfoil.run_command("getRecipes", "") or []
        versions = self.tinfoil.run_command("getRecipeVersions", "") or {}
        try:
            providers = self.tinfoil.run_command("findProviders", "") or ()
            preferred = (
                providers[1]
                if isinstance(providers, (list, tuple))
                and len(providers) > 1
                and isinstance(providers[1], dict)
                else {}
            )
        except Exception:
            preferred = {}
        try:
            all_appends = self.tinfoil.run_command("getAllAppends", "") or []
        except Exception:
            all_appends = []
        layers = self._layers()
        result = []
        for name, paths in recipes:
            if filter_value is not None and filter_value.lower() not in name.lower():
                continue
            recipe_paths = sorted(path for path in paths if isinstance(path, str))
            preferred_data = preferred.get(name)
            path = (
                preferred_data[1]
                if isinstance(preferred_data, (list, tuple))
                and len(preferred_data) > 1
                and isinstance(preferred_data[1], str)
                else recipe_paths[0]
                if recipe_paths
                else None
            )
            version_data = versions.get(path) if path is not None else None
            version = (
                str(version_data[1])
                if isinstance(version_data, (list, tuple)) and len(version_data) > 1
                else None
            )
            append_count = None
            if path is not None and isinstance(all_appends, (list, tuple)):
                basename = os.path.basename(path)
                append_count = sum(
                    1
                    for item in all_appends
                    if isinstance(item, (list, tuple))
                    and len(item) > 1
                    and isinstance(item[0], str)
                    and (
                        item[0] == basename
                        or (
                            "%" in item[0]
                            and item[0].startswith(basename[: item[0].index("%")])
                        )
                    )
                )
            if path is not None:
                self.recipe_files[str(name)] = path
            result.append(
                {
                    "name": str(name),
                    "version": version,
                    "layer": self._layer_for_path(path, layers)
                    if path is not None
                    else None,
                    "preferred_version": None,
                    "file": path,
                    "append_count": append_count,
                }
            )
        return result
