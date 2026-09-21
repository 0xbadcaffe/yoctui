def _tinfoil_start_build(self, targets, task, force=False):
    if self.active:
        raise RuntimeError("a BitBake build is already active")
    self._reset_for_build()
    self.task_recipe_identities = None
    self.tinfoil.set_event_mask(self.EVENT_MASK)
    selected_task = task or self.tinfoil.config_data.getVar("BB_DEFAULT_TASK")
    self.active = True
    try:
        if force:
            # BitBake's setConfig command coerces values to strings and
            # later tests this configuration field by truthiness.
            self.tinfoil.run_command("setConfig", "force", "1")
            self.force_active = True
        self.tinfoil.run_command(
            "buildTargets", targets, selected_task, handle_events=False
        )
    except Exception:
        self.active = False
        if self.force_active:
            self.tinfoil.run_command("setConfig", "force", "")
            self.force_active = False
        raise

def _tinfoil_cancel_build(self):
    if not self.active:
        raise RuntimeError("no BitBake build is active")
    # Cancellation is an explicit request to stop this runqueue.  Some
    # maintained BitBake generations let stateShutdown drain running work
    # for longer than Yoctui's bounded cancellation contract.  Use the
    # supported cooker force-shutdown command here; this remains a typed
    # Tinfoil/server operation and does not signal or kill an arbitrary
    # host process.
    self.tinfoil.run_command("stateForceShutdown", handle_events=False)

def _tinfoil_drain_events(self):
    events = []
    first = True
    while len(events) < MAX_NATIVE_EVENTS_PER_POLL:
        # A short first wait pumps the event socket after the runqueue
        # becomes idle. Pure zero-timeout polling can leave the final
        # BuildCompleted record unread until another server command.
        event = self.tinfoil.wait_event(0.01 if first else 0)
        first = False
        if event is None:
            break
        if (
            type(event).__name__ == "BuildStarted"
            and self.task_recipe_identities is None
        ):
            # BuildStarted follows buildTaskData: the recipe cache is now
            # initialized. getRecipes is a read-only cache query, not a
            # parse or a per-task metadata operation. Keep virtual paths
            # intact so native/multilib variants remain distinct.
            # Disable run_command's default native-event drain.
            try:
                self.task_recipe_identities = task_recipe_identity_index(
                    self.tinfoil.run_command("getRecipes", "", handle_events=False)
                )
            except Exception as error:
                self.task_recipe_identities = {}
                print(
                    f"Runqueue recipe identity unavailable: {str(error)[:512]}. Aggregate task statistics remain available.",
                    file=sys.stderr,
                )
        if task_recipe(event) is None and self.task_recipe_identities:
            task_file = event_value(event, "taskfile")
            recipe = (
                self.task_recipe_identities.get(task_file)
                if isinstance(task_file, str)
                else None
            )
            if recipe is not None:
                event.recipe = recipe
        events.append(event)
        if type(event).__name__ == "BuildCompleted":
            self.active = False
    if not self.active and self.force_active:
        self.tinfoil.run_command("setConfig", "force", "")
        self.force_active = False
    return events

def _tinfoil_shutdown(self):
    if self.tinfoil is not None:
        if self.force_active:
            self.tinfoil.run_command("setConfig", "force", "")
            self.force_active = False
        self.tinfoil.shutdown()
        self.tinfoil = None
    self.active = False

def _tinfoil_terminate_server(self):
    """Terminate through the process-server interface used by bitbake -m."""
    if self.tinfoil is None or self.tinfoil.server_connection is None:
        raise RuntimeError("BitBake server connection is unavailable")
    connection = self.tinfoil.server_connection
    connection.connection.terminateServer()
    connection.terminate()
    try:
        self.tinfoil_module._server_connections.remove(connection)
    except (AttributeError, ValueError):
        pass
    self.tinfoil.server_connection = None
    self.tinfoil = None
    self.active = False

TinfoilConnection.start_build = _tinfoil_start_build
TinfoilConnection.cancel_build = _tinfoil_cancel_build
TinfoilConnection.drain_events = _tinfoil_drain_events
TinfoilConnection.shutdown = _tinfoil_shutdown
TinfoilConnection.terminate_server = _tinfoil_terminate_server
