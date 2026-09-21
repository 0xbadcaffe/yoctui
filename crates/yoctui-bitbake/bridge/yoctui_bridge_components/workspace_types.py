def workspace_data(version):
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
    variables = {key: os.environ[key] for key in keys if key in os.environ}
    release = os.environ.get("DISTRO_VERSION") or os.environ.get(
        "OECORE_DISTRO_VERSION"
    )
    return {
        "type": "workspace",
        "data": {
            "build_dir": os.environ.get("BUILDDIR", os.getcwd()),
            "source_dir": os.environ.get("COREBASE"),
            "variables": variables,
            "variable_provenance": configured_variable_provenance(),
            "variable_provenance_chain": configured_variable_provenance_chain(),
            "bitbake_version": version,
            "release": release,
            "layers": [],
            "recipes": [],
        },
    }


def configured_variable_provenance():
    """Accept bridge-provided provenance without interpreting Yocto metadata locally."""
    try:
        raw = json.loads(os.environ.get("YOCTUI_VARIABLE_PROVENANCE_JSON", "{}"))
    except json.JSONDecodeError:
        return {}
    if not isinstance(raw, dict):
        return {}
    return {
        name: provenance
        for name, provenance in raw.items()
        if isinstance(name, str) and isinstance(provenance, str)
    }


def configured_variable_provenance_chain():
    try:
        raw = json.loads(os.environ.get("YOCTUI_VARIABLE_PROVENANCE_CHAIN_JSON", "{}"))
    except json.JSONDecodeError:
        return {}
    if not isinstance(raw, dict):
        return {}
    return {
        name: chain
        for name, chain in raw.items()
        if isinstance(name, str)
        and isinstance(chain, list)
        and all(isinstance(source, str) for source in chain)
    }


def typed_workspace(response):
    if not isinstance(response, dict):
        raise ServerUnavailable(
            "BitBake server returned an unsupported workspace response"
        )

    def optional_string(name):
        value = response.get(name)
        if value is None or isinstance(value, str):
            return value
        raise ServerUnavailable(f"BitBake server returned malformed {name} data")

    def string_map(name):
        value = response.get(name, {})
        if isinstance(value, dict) and all(
            isinstance(key, str) and isinstance(item, str)
            for key, item in value.items()
        ):
            return value
        raise ServerUnavailable(f"BitBake server returned malformed {name} data")

    def string_list_map(name):
        value = response.get(name, {})
        if isinstance(value, dict) and all(
            isinstance(key, str)
            and isinstance(items, list)
            and all(isinstance(item, str) for item in items)
            for key, items in value.items()
        ):
            return value
        raise ServerUnavailable(f"BitBake server returned malformed {name} data")

    return {
        "build_dir": optional_string("build_dir"),
        "source_dir": optional_string("source_dir"),
        "variables": string_map("variables"),
        "variable_provenance": string_map("variable_provenance"),
        "variable_provenance_chain": string_list_map("variable_provenance_chain"),
        "bitbake_version": optional_string("bitbake_version"),
        "release": optional_string("release"),
        "layers": typed_layers(response.get("layers", [])),
        "recipes": typed_recipes(response.get("recipes", [])),
    }


def configured_layers():
    values = []
    for path in os.environ.get("BBLAYERS", "").split():
        values.append(
            {
                "name": os.path.basename(path.rstrip("/")) or path,
                "path": path,
                "priority": None,
            }
        )
    return values


def configured_recipes():
    raw = os.environ.get("YOCTUI_RECIPES_JSON", "[]")
    try:
        recipes = json.loads(raw)
        if isinstance(recipes, list) and all(
            isinstance(item, dict) and isinstance(item.get("name"), str)
            for item in recipes
        ):
            return [
                {
                    "name": item["name"],
                    "version": item.get("version"),
                    "layer": item.get("layer"),
                }
                for item in recipes
            ]
    except json.JSONDecodeError:
        pass
    return []


def bitbake_recipes(filter_value):
    """Ask BitBake for its parsed recipe inventory when no server API is available."""
    try:
        result = subprocess.run(
            ["bitbake", "-s"],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            timeout=120,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired):
        return None

    if result.returncode != 0:
        return None
    recipes = []
    for line in result.stdout.splitlines():
        match = re.match(r"^([A-Za-z0-9_.+-]+)\s*:\s*(\S+)", line)
        if match and (
            filter_value is None or filter_value.lower() in match.group(1).lower()
        ):
            recipes.append(
                {"name": match.group(1), "version": match.group(2), "layer": None}
            )
    return recipes


def bitbake_layer_recipes(filter_value):
    try:
        result = subprocess.run(
            ["bitbake-layers", "show-recipes"],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            timeout=120,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired):
        return None
    if result.returncode != 0:
        return None
    recipes = []
    current = None
    for line in result.stdout.splitlines():
        heading = re.match(r"^([A-Za-z0-9_.+-]+):$", line)
        if heading:
            current = heading.group(1)
            continue
        detail = re.match(r"^\s+([A-Za-z0-9_.+-]+)\s+(\S+)", line)
        if (
            current
            and detail
            and (filter_value is None or filter_value.lower() in current.lower())
        ):
            recipes.append(
                {"name": current, "version": detail.group(2), "layer": detail.group(1)}
            )
            current = None
    return recipes


def bitbake_layers():
    try:
        result = subprocess.run(
            ["bitbake-layers", "show-layers"],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            timeout=120,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired):
        return None
    if result.returncode != 0:
        return None
    layers = []
    for line in result.stdout.splitlines():
        match = re.match(r"^(\S+)\s+(\S+)\s+(\d+)\s*$", line)
        if match:
            layers.append(
                {
                    "name": match.group(1),
                    "path": match.group(2),
                    "priority": int(match.group(3)),
                }
            )
    return layers


def typed_recipes(response):
    if not isinstance(response, list):
        raise ServerUnavailable(
            "BitBake server returned an unsupported recipe response"
        )
    if not all(
        isinstance(recipe, dict)
        and isinstance(recipe.get("name"), str)
        and (recipe.get("version") is None or isinstance(recipe.get("version"), str))
        and (recipe.get("layer") is None or isinstance(recipe.get("layer"), str))
        and (
            recipe.get("preferred_version") is None
            or isinstance(recipe.get("preferred_version"), str)
        )
        and (recipe.get("file") is None or isinstance(recipe.get("file"), str))
        and (
            recipe.get("append_count") is None
            or isinstance(recipe.get("append_count"), int)
        )
        for recipe in response
    ):
        raise ServerUnavailable("BitBake server returned malformed recipe data")
    return [
        {
            "name": recipe["name"],
            "version": recipe.get("version"),
            "layer": recipe.get("layer"),
            "preferred_version": recipe.get("preferred_version"),
            "file": recipe.get("file"),
            "append_count": recipe.get("append_count"),
        }
        for recipe in response
    ]


def typed_recipe_metadata(response):
    if not isinstance(response, dict) or not isinstance(response.get("recipe"), str):
        raise ServerUnavailable("BitBake server returned malformed recipe metadata")
    list_fields = ("tasks", "sources", "patches", "packages", "history")
    if any(
        response.get(field) is not None
        and (
            not isinstance(response.get(field), list)
            or not all(isinstance(value, str) for value in response[field])
        )
        for field in list_fields
    ):
        raise ServerUnavailable("BitBake server returned malformed recipe metadata")
    if response.get("workspace_status") not in (None, "clean", "modified"):
        raise ServerUnavailable("BitBake server returned an invalid workspace status")
    if response.get("build_status") not in (
        None,
        "idle",
        "queued",
        "running",
        "succeeded",
        "failed",
        "cancelled",
    ):
        raise ServerUnavailable(
            "BitBake server returned an invalid recipe build status"
        )
    return {
        "recipe": response["recipe"],
        "workspace_status": response.get("workspace_status"),
        "build_status": response.get("build_status"),
        **{field: response.get(field) for field in list_fields},
    }


def typed_layers(response):
    if not isinstance(response, list):
        raise ServerUnavailable("BitBake server returned an unsupported layer response")
    if not all(
        isinstance(layer, dict)
        and isinstance(layer.get("name"), str)
        and isinstance(layer.get("path"), str)
        and (layer.get("priority") is None or isinstance(layer.get("priority"), int))
        for layer in response
    ):
        raise ServerUnavailable("BitBake server returned malformed layer data")
    return [
        {
            "name": layer["name"],
            "path": layer["path"],
            "priority": layer.get("priority"),
        }
        for layer in response
    ]
