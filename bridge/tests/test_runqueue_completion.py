"""Runqueue terminal events must complete tasks without a worker event pair."""

from .support import (
    BRIDGE,
    Path,
    SimpleNamespace,
    importlib,
    json,
    run_bridge,
    tempfile,
    unittest,
)


class RunqueueCompletionTests(unittest.TestCase):
    def setUp(self) -> None:
        spec = importlib.util.spec_from_file_location("runqueue_completion", BRIDGE)
        assert spec is not None and spec.loader is not None
        self.bridge = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.bridge)

    def test_completion_events_are_subscribed_without_removing_worker_failures(self):
        mask = self.bridge.TinfoilConnection.EVENT_MASK
        for event in (
            "bb.runqueue.runQueueTaskCompleted",
            "bb.runqueue.sceneQueueTaskCompleted",
            "bb.build.TaskSucceeded",
            "bb.build.TaskFailed",
            "bb.build.TaskFailedSilent",
        ):
            with self.subTest(event=event):
                self.assertIn(event, mask)

    def test_runqueue_and_scene_completion_normalize_to_existing_typed_records(self):
        for kind, task in (
            ("runQueueTaskCompleted", "do_build"),
            ("sceneQueueTaskCompleted", "do_populate_sysroot_setscene"),
        ):
            with self.subTest(kind=kind):
                event = type(kind, (), {"recipe": "actual-pn", "taskname": task})()
                self.assertEqual(
                    self.bridge.normalize_event(event),
                    {
                        "type": "task_completed",
                        "recipe": "actual-pn",
                        "task": task,
                        "success": True,
                    },
                )

    def test_unresolved_completion_never_guesses_identity_or_success(self):
        stats = {"completed": 4, "total": 4, "active": 0, "failed": 0}
        for kind in ("runQueueTaskCompleted", "sceneQueueTaskCompleted"):
            for identity in ({}, {"recipe": ""}, {"recipe": 9}, {"taskname": ""}):
                event = {
                    "type": kind,
                    "taskname": "do_build",
                    "taskfile": "/layers/not-the-pn_git.bb",
                    **identity,
                }
                with self.subTest(kind=kind, identity=identity):
                    self.assertEqual(self.bridge.normalize_event(event), None)
                    self.assertEqual(
                        self.bridge.normalize_event({**event, "stats": stats}),
                        {"type": "task_stats", "stats": stats},
                    )
        self.assertIsNone(
            self.bridge.normalize_event(
                {"type": "runQueueTaskCompleted", "stats": {**stats, "total": -1}}
            )
        )

    def test_worker_failure_and_explicit_unsuccessful_completion_stay_failed(self):
        for kind in ("TaskFailed", "TaskFailedSilent", "runQueueTaskCompleted"):
            with self.subTest(kind=kind):
                event = {
                    "type": kind,
                    "recipe": "busybox",
                    "task": "do_compile",
                    "success": False,
                }
                self.assertFalse(self.bridge.normalize_event(event)["success"])

    def test_native_noexec_completion_uses_the_recipe_cache_once_without_worker(self):
        def event(kind, **fields):
            return type(kind, (), fields)()

        recipe_file = "/layers/not-the-image-name_git.bb"
        pending = [
            event("BuildStarted"),
            event(
                "runQueueTaskStarted",
                taskfile=recipe_file,
                taskname="do_build",
                noexec=True,
            ),
            event("runQueueTaskCompleted", taskfile=recipe_file, taskname="do_build"),
            event("BuildCompleted", _failures=0, _interrupted=0),
        ]
        calls, masks = [], []

        def command(name, *args, **kwargs):
            calls.append((name, args))
            self.assertEqual(kwargs, {"handle_events": False})
            if name == "getRecipes":
                return [("actual-image-pn", [recipe_file])]
            self.assertEqual(name, "buildTargets")

        connection = object.__new__(self.bridge.TinfoilConnection)
        connection.active = False
        connection.recipes_parsed = False
        connection.force_active = False
        connection.task_recipe_identities = None
        connection.tinfoil = SimpleNamespace(
            run_command=command,
            set_event_mask=masks.append,
            wait_event=lambda _: pending.pop(0) if pending else None,
        )
        connection.start_build(["actual-image-pn"], "build")
        rows = [self.bridge.normalize_event(raw) for raw in connection.drain_events()]
        self.assertIn("bb.runqueue.runQueueTaskCompleted", masks[0])
        self.assertEqual(
            [row["type"] for row in rows],
            ["build_started", "task_queued", "task_completed", "build_completed"],
        )
        self.assertEqual(rows[2]["recipe"], "actual-image-pn")
        self.assertTrue(rows[2]["success"])
        self.assertEqual(
            calls,
            [("buildTargets", (["actual-image-pn"], "build")), ("getRecipes", ("",))],
        )
        self.assertFalse(connection.active)

    def test_fake_process_emits_queue_completion_before_successful_build_result(self):
        with tempfile.TemporaryDirectory() as directory:
            Path(directory, "bb.py").write_text(
                """__version__ = "2.19.1"
class runQueueTaskStarted:
 recipe = "image"; taskname = "do_build"; noexec = True
class runQueueTaskCompleted:
 recipe = "image"; taskname = "do_build"
class BuildCompleted:
 _failures = 0; _interrupted = 0
class Connection:
 def start_build(self, targets, task): pass
 def drain_events(self):
  if getattr(self, "sent", False): return []
  self.sent = True
  return [runQueueTaskStarted(), runQueueTaskCompleted(), BuildCompleted()]
class Server:
 def connect(self): return Connection()
server = Server()
""",
                encoding="utf-8",
            )
            result = run_bridge(
                b'{"protocol_version":1,"sequence":1,"message":{"type":"start_build","targets":["image"],"task":null}}',
                environment={"PYTHONPATH": directory},
            )
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        rows = [json.loads(line)["message"] for line in result.stdout.splitlines()]
        self.assertEqual(
            [row["type"] for row in rows],
            ["build_started", "task_queued", "task_completed", "build_completed"],
        )
        self.assertEqual(
            rows[2],
            {
                "type": "task_completed",
                "recipe": "image",
                "task": "do_build",
                "success": True,
            },
        )
        self.assertTrue(rows[3]["success"])
        self.assertEqual(rows[3]["exit_code"], 0)
