use super::*;

#[tokio::test]
async fn daemon_cancellation_preempts_slow_inventory() {
    let root = std::env::temp_dir().join(format!(
        "yoctui-cancel-inventory-{}-{}",
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
            r#"import os, time
__version__ = "2.18.0"
class Connection:
 native_event_stream = True
 def inspect_workspace(self):
  return {{"build_dir": {build:?}, "source_dir": None, "variables": {{}}, "variable_provenance": {{}}, "variable_provenance_chain": {{}}, "bitbake_version": "2.18.0", "release": "6.0.2", "layers": [], "recipes": []}}
 def list_recipes(self, filter_value):
  open("inventory-started", "w").write(str(os.getpid()))
  time.sleep(30)
  return []
 def list_layers(self): return []
 def start_build(self, targets, task, force=False): open("build-started", "w").close()
 def drain_events(self): return []
 def terminate_server(self): pass
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
    let job_id = supervisor
        .start(
            build.clone(),
            BuildRequest {
                targets: vec!["fixture-image".into()],
                task: None,
                force: false,
            },
        )
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while !build.join("inventory-started").exists() && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(build.join("inventory-started").exists());
    supervisor.cancel(job_id).unwrap();
    let deadline = Instant::now() + Duration::from_secs(1);
    let mut terminal = false;
    while !terminal && Instant::now() < deadline {
        while let Some(event) = supervisor.try_event() {
            match event {
                DaemonBitBakeEvent::Backend { event, .. } => {
                    if let BackendEvent::BuildCompleted { success, exit_code } = *event {
                        assert!(!success);
                        assert_eq!(exit_code, Some(130));
                        assert!(!terminal, "duplicate cancellation terminal");
                        terminal = true;
                    }
                }
                DaemonBitBakeEvent::Failed { message, .. } => panic!("{message}"),
            }
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(terminal, "cancellation waited for recipe inventory");
    assert!(!build.join("build-started").exists());
    assert!(supervisor.cancel(job_id).is_err());
    let pid: i32 = fs::read_to_string(build.join("inventory-started"))
        .unwrap()
        .parse()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    while unsafe { libc::kill(pid, 0) } == 0 && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(unsafe { libc::kill(pid, 0) }, -1, "bridge was not reaped");
    assert!(supervisor.try_event().is_none());
    fs::remove_dir_all(root).unwrap();
}
