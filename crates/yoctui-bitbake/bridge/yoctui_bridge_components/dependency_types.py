def typed_dependencies(response):
    if not isinstance(response, dict):
        raise ServerUnavailable(
            "BitBake server returned an unsupported dependency response"
        )
    build = response.get("build", [])
    runtime = response.get("runtime", [])
    if not all(
        isinstance(values, list) and all(isinstance(value, str) for value in values)
        for values in (build, runtime)
    ):
        raise ServerUnavailable("BitBake server returned malformed dependency data")
    return {"build": build, "runtime": runtime}


def dependency_node_id(value):
    if not isinstance(value, dict):
        raise ServerUnavailable(
            "BitBake server returned a malformed dependency identity"
        )
    recipe = value.get("recipe")
    task = value.get("task")
    if (
        not isinstance(recipe, str)
        or not recipe
        or len(recipe) > 512
        or any(ord(character) < 32 for character in recipe)
        or (
            task is not None
            and (
                not isinstance(task, str)
                or not task
                or len(task) > 512
                or any(ord(character) < 32 for character in task)
            )
        )
    ):
        raise ServerUnavailable(
            "BitBake server returned a malformed dependency identity"
        )
    result = {"recipe": recipe}
    if task is not None:
        result["task"] = task
    return result


def dependency_task_id(value):
    if not isinstance(value, str) or "." not in value:
        raise ServerUnavailable("BitBake server returned a malformed task dependency")
    recipe, task = value.rsplit(".", 1)
    return dependency_node_id({"recipe": recipe, "task": task})


def typed_dependency_graph(response, requested_recipe):
    if not isinstance(response, dict):
        raise ServerUnavailable(
            "BitBake server returned an unsupported dependency graph response"
        )
    root = dependency_node_id(response.get("root"))
    if root != {"recipe": requested_recipe}:
        raise ServerUnavailable(
            "BitBake server returned a dependency graph for a different root"
        )
    raw_nodes = response.get("nodes", [])
    raw_edges = response.get("edges", [])
    limitations = response.get("limitations", [])
    if (
        not isinstance(raw_nodes, list)
        or not isinstance(raw_edges, list)
        or not isinstance(limitations, list)
        or not all(
            isinstance(value, str) and len(value) <= 512 for value in limitations
        )
    ):
        raise ServerUnavailable(
            "BitBake server returned malformed dependency graph data"
        )

    normalized_nodes = {}
    dropped_paths = 0
    for raw_node in raw_nodes:
        if not isinstance(raw_node, dict):
            raise ServerUnavailable(
                "BitBake server returned a malformed dependency node"
            )
        identity = dependency_node_id(raw_node.get("id"))
        provider = raw_node.get("provider")
        log = raw_node.get("log")
        if provider is not None and (
            not isinstance(provider, str) or not os.path.isabs(provider)
        ):
            provider = None
            dropped_paths += 1
        if log is not None and (not isinstance(log, str) or not os.path.isabs(log)):
            log = None
            dropped_paths += 1
        key = (identity["recipe"], identity.get("task"))
        candidate = {"id": identity}
        if provider is not None:
            candidate["provider"] = provider
        if log is not None:
            candidate["log"] = log
        previous = normalized_nodes.get(key)
        if previous is None or json.dumps(candidate, sort_keys=True) < json.dumps(
            previous, sort_keys=True
        ):
            normalized_nodes[key] = candidate

    edge_values = set()
    for raw_edge in raw_edges:
        if not isinstance(raw_edge, dict):
            raise ServerUnavailable(
                "BitBake server returned a malformed dependency edge"
            )
        source = dependency_node_id(raw_edge.get("from"))
        destination = dependency_node_id(raw_edge.get("to"))
        kind = raw_edge.get("kind")
        if kind not in ("build", "runtime", "task"):
            raise ServerUnavailable(
                "BitBake server returned an unknown dependency edge kind"
            )
        source_key = (source["recipe"], source.get("task"))
        destination_key = (destination["recipe"], destination.get("task"))
        if source_key == destination_key:
            continue
        edge_values.add((source_key, destination_key, kind))
        normalized_nodes.setdefault(source_key, {"id": source})
        normalized_nodes.setdefault(destination_key, {"id": destination})

    normalized_nodes.setdefault((requested_recipe, None), {"id": root})
    sorted_nodes = [
        normalized_nodes[key]
        for key in sorted(
            normalized_nodes, key=lambda value: (value[0], value[1] or "")
        )
    ]
    dropped_nodes = max(0, len(sorted_nodes) - MAX_DEPENDENCY_NODES)
    sorted_nodes = sorted_nodes[:MAX_DEPENDENCY_NODES]
    retained = {(node["id"]["recipe"], node["id"].get("task")) for node in sorted_nodes}
    if (requested_recipe, None) not in retained:
        sorted_nodes[-1] = {"id": root}
        sorted_nodes.sort(
            key=lambda node: (node["id"]["recipe"], node["id"].get("task") or "")
        )
        retained = {
            (node["id"]["recipe"], node["id"].get("task")) for node in sorted_nodes
        }

    sorted_edges = [
        {
            "from": {
                "recipe": source[0],
                **({"task": source[1]} if source[1] is not None else {}),
            },
            "to": {
                "recipe": destination[0],
                **({"task": destination[1]} if destination[1] is not None else {}),
            },
            "kind": kind,
        }
        for source, destination, kind in sorted(
            edge_values,
            key=lambda value: (
                value[0][0],
                value[0][1] or "",
                value[1][0],
                value[1][1] or "",
                value[2],
            ),
        )
        if source in retained and destination in retained
    ]
    dropped_edges = len(edge_values) - len(sorted_edges)
    if len(sorted_edges) > MAX_DEPENDENCY_EDGES:
        dropped_edges += len(sorted_edges) - MAX_DEPENDENCY_EDGES
        sorted_edges = sorted_edges[:MAX_DEPENDENCY_EDGES]
    limitations = list(dict.fromkeys(limitations))
    if dropped_paths:
        limitations.append(
            f"Dropped {dropped_paths} non-absolute provider or log paths."
        )
    if dropped_nodes or dropped_edges:
        limitations.append(
            f"Dependency graph bounds dropped {dropped_nodes} nodes and {dropped_edges} edges."
        )
    return {
        "root": root,
        "nodes": sorted_nodes,
        "edges": sorted_edges,
        "limitations": limitations,
    }


def dependency_graph_from_deptree(recipe, response):
    if not isinstance(response, dict):
        raise ServerUnavailable("BitBake returned malformed dependency tree data")
    recipes = response.get("pn")
    build_dependencies = response.get("depends")
    runtime_dependencies = response.get("rdepends-pn")
    task_dependencies = response.get("tdepends")
    providers = response.get("providermap", {})
    if not all(
        isinstance(value, dict)
        for value in (
            recipes,
            build_dependencies,
            runtime_dependencies,
            task_dependencies,
            providers,
        )
    ):
        raise ServerUnavailable("BitBake returned incomplete dependency tree data")

    def provider_name(value):
        provider = providers.get(value)
        if (
            isinstance(provider, (list, tuple))
            and provider
            and isinstance(provider[0], str)
        ):
            return provider[0]
        return value

    nodes = []
    edges = []
    for name, metadata in recipes.items():
        if not isinstance(name, str) or not isinstance(metadata, dict):
            raise ServerUnavailable("BitBake returned malformed recipe dependency data")
        provider = metadata.get("filename")
        nodes.append(
            {
                "id": {"recipe": name},
                "provider": provider if isinstance(provider, str) else None,
            }
        )
    for kind, dependencies in (
        ("build", build_dependencies),
        ("runtime", runtime_dependencies),
    ):
        for source, destinations in dependencies.items():
            if not isinstance(source, str) or not isinstance(
                destinations, (list, tuple)
            ):
                raise ServerUnavailable(
                    "BitBake returned malformed recipe dependency edges"
                )
            for destination in destinations:
                if not isinstance(destination, str):
                    raise ServerUnavailable(
                        "BitBake returned malformed recipe dependency edges"
                    )
                edges.append(
                    {
                        "from": {"recipe": source},
                        "to": {"recipe": provider_name(destination)},
                        "kind": kind,
                    }
                )
    for source, destinations in task_dependencies.items():
        source_id = dependency_task_id(source)
        nodes.append({"id": source_id})
        if not isinstance(destinations, (list, tuple)):
            raise ServerUnavailable("BitBake returned malformed task dependency edges")
        for destination in destinations:
            destination_id = dependency_task_id(destination)
            nodes.append({"id": destination_id})
            edges.append({"from": source_id, "to": destination_id, "kind": "task"})
    return typed_dependency_graph(
        {
            "root": {"recipe": recipe},
            "nodes": nodes,
            "edges": edges,
            "limitations": [],
        },
        recipe,
    )


def typed_layer_relationships(response):
    fields = ("compatible", "depends", "overlays", "appends")
    if not isinstance(response, list) or not all(
        isinstance(layer, dict)
        and isinstance(layer.get("name"), str)
        and (layer.get("priority") is None or isinstance(layer.get("priority"), int))
        and all(
            isinstance(layer.get(field, []), list)
            and all(isinstance(value, str) for value in layer.get(field, []))
            for field in fields
        )
        for layer in response
    ):
        raise ServerUnavailable(
            "BitBake server returned malformed layer relationship data"
        )
    return [
        {
            "name": layer["name"],
            "priority": layer.get("priority"),
            **{field: layer.get(field, []) for field in fields},
        }
        for layer in response
    ]
