use super::*;

#[tokio::test]
async fn daemon_job_identity_cancellation_reaches_only_the_exact_bridge_owner() {
    let root = std::env::temp_dir().join(format!(
        "yoctui-job-identity-{}-{}",
        std::process::id(),
        NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
    ));
    let python = root.join("python");
    fs::create_dir_all(&python).unwrap();
    fs::write(python.join("bb.py"), r#"import os
__version__ = "2.18.0"
class Connection:
 native_event_stream = True
 def __init__(self): self.cancelled = False
 def inspect_workspace(self):
  return {"build_dir": os.environ["YOCTUI_TEST_BUILD"], "source_dir": None, "variables": {}, "variable_provenance": {}, "variable_provenance_chain": {}, "bitbake_version": "2.18.0", "release": "6.0.2", "layers": [], "recipes": []}
 def list_recipes(self, filter_value): return []
 def list_layers(self): return []
 def start_build(self, targets, task, force=False):
  open(os.environ["YOCTUI_TEST_MARKER"], "w").write("started")
 def cancel_build(self):
  self.cancelled = True
  open(os.environ["YOCTUI_TEST_MARKER"], "w").write("cancelled")
 def drain_events(self):
  if self.cancelled: return [{"type": "build_completed", "success": False, "exit_code": 1}]
  return [{"type": "parse_progress", "parsed": 1, "total": 10}]
 def terminate_server(self): pass
 def shutdown(self): pass
class Server:
 def __init__(self): self.connection = Connection()
 def connect(self): return self.connection
server = Server()
"#).unwrap();
    let ids = crate::daemon_job_ids::DaemonJobIds::default();
    let mut owners = Vec::new();
    for index in 0..2 {
        let build = root.join(format!("build-{index}"));
        fs::create_dir(&build).unwrap();
        let build = build.canonicalize().unwrap();
        let marker = root.join(format!("marker-{index}"));
        let mut owner =
            DaemonBitBakeSupervisor::new(ids.clone()).with_bridge_environment(BTreeMap::from([
                ("PYTHONPATH".into(), python.display().to_string()),
                ("YOCTUI_TEST_BUILD".into(), build.display().to_string()),
                ("YOCTUI_TEST_MARKER".into(), marker.display().to_string()),
            ]));
        owner
            .replace_compatibility(Some(cancellation_authority(&build)))
            .unwrap();
        let job = owner
            .start(
                build,
                BuildRequest {
                    targets: vec!["base-files".into()],
                    task: None,
                    force: false,
                },
            )
            .unwrap();
        owners.push((owner, job, marker));
    }
    assert_ne!(owners[0].1, owners[1].1);
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut ready = [false; 2];
    while !ready.iter().all(|value| *value) {
        for (index, (owner, _, _)) in owners.iter_mut().enumerate() {
            while let Some(event) = owner.try_event() {
                match event {
                    DaemonBitBakeEvent::Backend { event, .. } => match *event {
                        BackendEvent::ParseProgress { .. } => ready[index] = true,
                        BackendEvent::BuildCompleted { .. } => {
                            panic!("unexpected early terminal")
                        }
                        _ => {}
                    },
                    DaemonBitBakeEvent::Failed { message, .. } => panic!("{message}"),
                }
            }
        }
        assert!(
            Instant::now() < deadline,
            "both isolated bridges must start"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let first = owners[0].1;
    let second = owners[1].1;
    assert!(owners[0].0.cancel(second).is_err());
    assert!(owners[1].0.cancel(first).is_err());
    owners[1].0.cancel(second).unwrap();

    async fn await_terminal(owner: &mut DaemonBitBakeSupervisor, expected: JobId) {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            while let Some(event) = owner.try_event() {
                match event {
                    DaemonBitBakeEvent::Backend { job_id, event } => {
                        assert_eq!(job_id, expected);
                        if let BackendEvent::BuildCompleted { success, .. } = *event {
                            assert!(!success);
                            return;
                        }
                    }
                    DaemonBitBakeEvent::Failed { message, .. } => panic!("{message}"),
                }
            }
            assert!(
                Instant::now() < deadline,
                "owned bridge cancellation must terminate"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
    await_terminal(&mut owners[1].0, second).await;
    assert_eq!(fs::read_to_string(&owners[1].2).unwrap(), "cancelled");
    assert_eq!(fs::read_to_string(&owners[0].2).unwrap(), "started");
    assert!(owners[0].0.active.contains_key(&first));
    owners[0].0.cancel(first).unwrap();
    await_terminal(&mut owners[0].0, first).await;
    fs::remove_dir_all(root).unwrap();
}
