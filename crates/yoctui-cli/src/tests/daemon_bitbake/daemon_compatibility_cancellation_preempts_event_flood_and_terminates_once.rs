use super::*;

#[tokio::test]
async fn daemon_compatibility_cancellation_preempts_event_flood_and_terminates_once() {
    let root = std::env::temp_dir().join(format!(
        "yoctui-daemon-cancellation-{}-{}",
        std::process::id(),
        NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
    ));
    let build = root.join("build");
    let python = root.join("python");
    let marker = root.join("cancelled");
    fs::create_dir_all(&build).unwrap();
    fs::create_dir_all(&python).unwrap();
    let build = build.canonicalize().unwrap();
    fs::write(
        python.join("bb.py"),
        format!(
            r#"import time
__version__ = "2.18.0"
class Connection:
 native_event_stream = True
 def __init__(self): self.cancelled = False
 def inspect_workspace(self):
  return {{"build_dir": {build:?}, "source_dir": None, "variables": {{}}, "variable_provenance": {{}}, "variable_provenance_chain": {{}}, "bitbake_version": "2.18.0", "release": "6.0.2", "layers": [], "recipes": []}}
 def list_recipes(self, filter_value):
  return [{{"name": "busybox", "version": "1.36", "layer": "core", "preferred_version": None, "file": None, "append_count": 0}}]
 def list_layers(self):
  return [{{"name": "core", "path": "/layer", "priority": 5}}]
 def start_build(self, targets, task, force=False): pass
 def cancel_build(self):
  self.cancelled = True
  open({marker:?}, "w", encoding="utf-8").write("cancelled")
 def drain_events(self):
  def events():
   for index in range(10000):
    if self.cancelled:
     yield {{"type": "build_completed", "success": False, "exit_code": 1}}
     yield {{"type": "build_started"}}
     return
    yield {{"type": "parse_progress", "parsed": index, "total": 10000}}
  return events()
 def terminate_server(self): time.sleep(30)
 def shutdown(self): pass
class Server:
 def __init__(self): self.connection = Connection()
 def connect(self): return self.connection
server = Server()
"#,
            build = build.display().to_string(),
            marker = marker.display().to_string(),
        ),
    )
    .unwrap();

    let mut supervisor =
        DaemonBitBakeSupervisor::new(Default::default()).with_bridge_environment(BTreeMap::from([
            ("PYTHONPATH".into(), python.display().to_string()),
        ]));
    supervisor
        .replace_compatibility(Some(cancellation_authority(&build)))
        .unwrap();
    let job_id = supervisor
        .start(
            build,
            BuildRequest {
                targets: vec!["base-files".into()],
                task: None,
                force: false,
            },
        )
        .unwrap();

    let deadline = Instant::now() + Duration::from_secs(10);
    let mut cancellation_sent = false;
    let mut cancellation_started = None;
    let mut cancellation_terminal_latency = None;
    let mut terminal_count = 0;
    let mut late_started = false;
    let mut saw_inventory = false;
    while Instant::now() < deadline && terminal_count == 0 {
        while let Some(event) = supervisor.try_event() {
            match event {
                DaemonBitBakeEvent::Backend { event, .. } => match *event {
                    BackendEvent::Workspace(workspace) => {
                        assert_eq!(workspace.recipes.len(), 1);
                        assert_eq!(workspace.recipes[0].name, "busybox");
                        assert_eq!(workspace.layers.len(), 1);
                        assert_eq!(workspace.layers[0].name, "core");
                        saw_inventory = true;
                    }
                    BackendEvent::ParseProgress { .. } if !cancellation_sent => {
                        supervisor.cancel(job_id).unwrap();
                        cancellation_sent = true;
                        cancellation_started = Some(Instant::now());
                    }
                    BackendEvent::BuildStarted if cancellation_sent => late_started = true,
                    BackendEvent::BuildCompleted { success, .. } => {
                        assert!(!success);
                        cancellation_terminal_latency =
                            cancellation_started.map(|started| started.elapsed());
                        terminal_count += 1;
                    }
                    _ => {}
                },
                DaemonBitBakeEvent::Failed { message, .. } => panic!("{message}"),
            }
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    assert!(cancellation_sent, "event stream never became active");
    assert!(saw_inventory, "daemon workspace omitted metadata inventory");
    assert_eq!(terminal_count, 1);
    assert!(
        cancellation_terminal_latency.is_some_and(|latency| latency < Duration::from_secs(1)),
        "terminal publication waited behind the two-second server cleanup"
    );
    assert!(!late_started);
    assert!(
        supervisor.try_event().is_none(),
        "pre-cancellation native records survived the terminal boundary"
    );
    assert_eq!(fs::read_to_string(&marker).unwrap(), "cancelled");
    assert!(supervisor.cancel(job_id).is_err());
    fs::remove_dir_all(&root).unwrap();
}
