use super::*;

#[tokio::test]
async fn bitbake_connection_tolerates_scheduler_delay_without_false_disconnect() {
    let root = std::env::temp_dir().join(format!(
        "yoctui-bitbake-delayed-events-{}-{}",
        std::process::id(),
        NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
    ));
    let build = root.join("build");
    let python = root.join("python");
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
 def __init__(self): self.poll = 0
 def inspect_workspace(self):
  return {{"build_dir": {build:?}, "source_dir": None, "variables": {{}}, "variable_provenance": {{}}, "variable_provenance_chain": {{}}, "bitbake_version": "2.18.0", "release": "6.0.2", "layers": [], "recipes": []}}
 def list_recipes(self, filter_value): return []
 def list_layers(self): return []
 def start_build(self, targets, task, force=False): self.poll = 0
 def drain_events(self):
  self.poll += 1
  if self.poll == 1: return [{{"type": "build_started"}}]
  if self.poll == 2:
   time.sleep(0.25)
   return [{{"type": "task_started", "recipe": "base-files", "task": "do_compile", "pid": 41}}]
  if self.poll == 3: return [{{"type": "build_completed", "success": True, "exit_code": 0}}]
  return []
 def shutdown(self): pass
class Server:
 def connect(self): return Connection()
server = Server()
"#,
            build = build.display().to_string(),
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
    supervisor
        .start(
            build,
            BuildRequest {
                targets: vec!["base-files".into()],
                task: None,
                force: false,
            },
        )
        .unwrap();

    let deadline = Instant::now() + Duration::from_secs(5);
    let mut saw_started = false;
    let mut saw_task = false;
    let mut saw_terminal = false;
    while Instant::now() < deadline && !saw_terminal {
        while let Some(event) = supervisor.try_event() {
            match event {
                DaemonBitBakeEvent::Backend { event, .. } => match *event {
                    BackendEvent::BuildStarted => saw_started = true,
                    BackendEvent::TaskStarted { .. } => saw_task = true,
                    BackendEvent::BuildCompleted { success: true, .. } => saw_terminal = true,
                    BackendEvent::Disconnected => {
                        panic!("scheduler delay was misclassified as a disconnect")
                    }
                    _ => {}
                },
                DaemonBitBakeEvent::Failed { message, .. } => panic!("{message}"),
            }
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    assert!(saw_started && saw_task && saw_terminal);
    fs::remove_dir_all(root).unwrap();
}
