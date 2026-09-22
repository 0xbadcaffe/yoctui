def event_value(event, *names, default=None):
    for name in names:
        value = (
            event.get(name) if isinstance(event, dict) else getattr(event, name, None)
        )
        if value is not None:
            return value
    return default


def normalized_nonnegative_integer(value, maximum=None):
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        return None
    if isinstance(value, float) and not math.isfinite(value):
        return None
    if value < 0:
        return None
    normalized = int(value)
    return min(normalized, maximum) if maximum is not None else normalized


def normalized_task_stats(event):
    stats = event_value(event, "stats")
    if stats is None:
        return None
    values = {
        name: event_value(stats, name)
        for name in ("completed", "total", "active", "failed")
    }
    if not all(isinstance(value, int) and value >= 0 for value in values.values()):
        return None
    return values


def task_recipe_identity_index(recipes):
    """Invert initialized PN/file metadata without filename inference."""
    if (
        not isinstance(recipes, (list, tuple))
        or len(recipes) > MAX_RECIPE_INVENTORY_RECORDS
    ):
        raise ValueError("recipe identity record limit or invalid response")
    identities = {}
    size = 0
    count = 0
    for row in recipes:
        if not isinstance(row, (list, tuple)) or len(row) != 2:
            raise ValueError("invalid recipe identity row")
        recipe, paths = row
        if (
            not isinstance(recipe, str)
            or not recipe
            or not isinstance(paths, (list, tuple))
        ):
            raise ValueError("invalid recipe identity metadata")
        for path in paths:
            if not isinstance(path, str) or not path:
                raise ValueError("invalid recipe identity path")
            count += 1
            size += len(recipe.encode()) + len(path.encode())
            if (
                count > MAX_RECIPE_INVENTORY_RECORDS
                or size > MAX_RECIPE_INVENTORY_BYTES
            ):
                raise ValueError("recipe identity resource limit")
            if path in identities and identities[path] != recipe:
                identities[path] = None  # Conflicting authority is unknown.
            else:
                identities[path] = recipe
    return identities


def task_recipe(event):
    recipe = event_value(event, "recipe", "pn")
    if isinstance(recipe, str) and recipe:
        return recipe
    return None


def correlated_task(task_identities_by_pid, pid):
    if task_identities_by_pid is None or pid is None:
        return None
    identity = task_identities_by_pid.get(pid)
    if not isinstance(identity, (list, tuple)) or len(identity) < 2:
        return None
    recipe, task = identity[:2]
    if not all(isinstance(value, str) and value for value in (recipe, task)):
        return None
    log_path = identity[2] if len(identity) > 2 else None
    return recipe, task, log_path if isinstance(log_path, str) else None


def normalize_event(event, task_identities_by_pid=None):
    kind = event_value(event, "type", "event_type")
    if not isinstance(kind, str) and event is not None:
        kind = type(event).__name__
    normalized_kind = kind.lower() if isinstance(kind, str) else None
    recipe = task_recipe(event)
    task = event_value(event, "task", "taskname")
    if normalized_kind in ("buildstarted", "build_started"):
        if task_identities_by_pid is not None:
            task_identities_by_pid.clear()
        return {"type": "build_started"}
    if normalized_kind in ("parsestarted", "parse_started"):
        return {
            "type": "parse_progress",
            "current": 0,
            "total": normalized_nonnegative_integer(event_value(event, "total")),
        }
    if normalized_kind in ("parseprogress", "parse_progress"):
        return {
            "type": "parse_progress",
            "current": normalized_nonnegative_integer(
                event_value(event, "current", "progress")
            ),
            "total": normalized_nonnegative_integer(event_value(event, "total")),
        }
    if normalized_kind in ("processstarted", "process_started"):
        return {
            "type": "parse_progress",
            "current": 0,
            "total": normalized_nonnegative_integer(event_value(event, "total")),
        }
    if normalized_kind in ("processprogress", "process_progress"):
        return {
            "type": "parse_progress",
            "current": normalized_nonnegative_integer(
                event_value(event, "progress"), 100
            ),
            "total": 100,
        }
    if normalized_kind in ("processfinished", "process_finished"):
        return {"type": "parse_progress", "current": 100, "total": 100}
    if normalized_kind in ("parsecompleted", "parse_completed"):
        total = normalized_nonnegative_integer(event_value(event, "total"))
        return {"type": "parse_progress", "current": total, "total": total}
    if normalized_kind in ("buildcompleted", "build_completed"):
        exit_code = event_value(event, "exit_code", "returncode")
        explicit_success = event_value(event, "success")
        failures = event_value(event, "_failures", "failures")
        if failures is None:
            getter = getattr(event, "getFailures", None)
            failures = getter() if callable(getter) else None
        interrupted = event_value(event, "_interrupted", "interrupted", default=0)
        success = (
            bool(explicit_success)
            if explicit_success is not None
            else not bool(failures) and not bool(interrupted)
        )
        if exit_code is None:
            exit_code = 0 if success else 1
        return {
            "type": "build_completed",
            "success": success,
            "exit_code": exit_code if isinstance(exit_code, int) else None,
        }
    if normalized_kind in (
        "tasksucceeded",
        "taskcompleted",
        "task_completed",
        "taskfailed",
        "taskfailedsilent",
    ) and all(isinstance(value, str) for value in (recipe, task)):
        success = normalized_kind not in ("taskfailed", "taskfailedsilent") and bool(
            event_value(event, "success", default=True)
        )
        pid = normalized_nonnegative_integer(
            event_value(event, "taskpid", "pid", "process")
        )
        if task_identities_by_pid is not None and pid is not None:
            task_identities_by_pid.pop(pid, None)
        return {
            "type": "task_completed",
            "recipe": recipe,
            "task": task,
            "success": success,
        }
    if normalized_kind in ("taskstarted", "task_started") and all(
        isinstance(value, str) for value in (recipe, task)
    ):
        pid = normalized_nonnegative_integer(event_value(event, "pid"))
        worker = event_value(event, "worker")
        log_path = event_value(event, "logfile")
        if task_identities_by_pid is not None and pid is not None:
            task_identities_by_pid[pid] = (recipe, task, log_path)
        return {
            "type": "task_started",
            "recipe": recipe,
            "task": task,
            "pid": pid,
            "worker": str(worker) if worker is not None else None,
            "log_path": log_path,
            "stats": normalized_task_stats(event),
        }
    if normalized_kind in ("runqueuetaskstarted", "scenequeuetaskstarted"):
        stats = normalized_task_stats(event)
        if not all(isinstance(value, str) and value for value in (recipe, task)):
            return {"type": "task_stats", "stats": stats} if stats is not None else None
        return {
            "type": "task_queued",
            "recipe": recipe,
            "task": task,
            "worker": None,
            "stats": stats,
        }
    if normalized_kind in ("taskprogress", "task_progress"):
        pid = normalized_nonnegative_integer(event_value(event, "pid"))
        if not all(isinstance(value, str) for value in (recipe, task)):
            identity = correlated_task(task_identities_by_pid, pid)
            if identity is None:
                return None
            recipe, task, _ = identity
        return {
            "type": "task_progress",
            "recipe": recipe,
            "task": task,
            "progress": normalized_nonnegative_integer(
                event_value(event, "progress"), 100
            ),
        }
    message = event_value(event, "message", "msg")
    diagnostic_levels = {
        "warning": "warning",
        "warn": "warning",
        "error": "error",
        "critical": "error",
        "fatal": "error",
    }
    if normalized_kind in ("log", "logrecord", *diagnostic_levels) and isinstance(
        message, str
    ):
        pid = normalized_nonnegative_integer(
            event_value(event, "taskpid", "pid", "process")
        )
        identity = correlated_task(task_identities_by_pid, pid)
        if not all(isinstance(value, str) for value in (recipe, task)):
            if identity is not None:
                recipe, task, _ = identity
        level = event_value(
            event,
            "level",
            "levelname",
            default=diagnostic_levels.get(normalized_kind, "info"),
        )
        level = level.lower() if isinstance(level, str) else "info"
        level = diagnostic_levels.get(level, level)
        return {
            "type": "log",
            "level": level,
            "message": message,
            "recipe": recipe,
            "task": task,
            "path": (
                identity[2]
                if identity is not None and identity[2] is not None
                else event_value(event, "path", "pathname", "filename")
            ),
        }
    if normalized_kind in ("commandcompleted", "command_completed"):
        return None
    if normalized_kind in ("commandfailed", "commandexit", "command_failed"):
        return {
            "type": "build_completed",
            "success": False,
            "exit_code": 1,
        }
    return {"type": "warning", "message": f"unrecognized BitBake event: {kind!r}"}


def emit_adapter_events(adapter):
    for event in adapter.native_events():
        emit(event, adapter.build_correlation_id)
        if event.get("type") == "build_completed":
            adapter.build_correlation_id = None
