use super::*;

#[tokio::test]
async fn daemon_cancellation_times_out_to_one_terminal_event() {
    let root = std::env::temp_dir().join(format!(
        "yoctui-daemon-cancellation-timeout-{}-{}",
        std::process::id(),
        NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
    ));
    let build = root.join("build");
    let python = root.join("python");
    let cancelled = root.join("cancelled");
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
 def inspect_workspace(self):
  return {{"build_dir": {build:?}, "source_dir": None, "variables": {{}}, "variable_provenance": {{}}, "variable_provenance_chain": {{}}, "bitbake_version": "2.18.0", "release": "6.0.2", "layers": [], "recipes": []}}
 def list_recipes(self, filter_value): return []
 def list_layers(self): return []
 def start_build(self, targets, task, force=False): pass
 def cancel_build(self):
  open({cancelled:?}, "w", encoding="utf-8").write("cancelled")
  time.sleep(30)
 def drain_events(self): return [{{"type": "parse_progress", "current": 1, "total": 2}}]
 def terminate_server(self): pass
 def shutdown(self): pass
class Server:
 def connect(self): return Connection()
server = Server()
"#,
            build = build.display().to_string(),
            cancelled = cancelled.display().to_string(),
        ),
    )
    .unwrap();

    let mut supervisor = DaemonBitBakeSupervisor::new(Default::default())
        .with_bridge_environment(BTreeMap::from([(
            "PYTHONPATH".into(),
            python.display().to_string(),
        )]))
        .with_cancellation_terminal_timeout(Duration::from_millis(40));
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

    let deadline = Instant::now() + Duration::from_secs(5);
    let mut sent = false;
    let mut terminals = 0;
    while Instant::now() < deadline && terminals == 0 {
        while let Some(event) = supervisor.try_event() {
            if matches!(
                &event,
                DaemonBitBakeEvent::Backend {
                    event,
                    ..
                } if matches!(event.as_ref(), BackendEvent::ParseProgress { .. })
            ) && !sent
            {
                supervisor.cancel(job_id).unwrap();
                sent = true;
            } else if matches!(
                &event,
                DaemonBitBakeEvent::Backend {
                    event,
                    ..
                } if matches!(event.as_ref(), BackendEvent::BuildCompleted {
                    success: false,
                    exit_code: Some(130)
                })
            ) {
                terminals += 1;
            }
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    assert!(sent, "cancellation was never submitted");
    assert_eq!(terminals, 1);
    assert_eq!(fs::read_to_string(cancelled).unwrap(), "cancelled");
    assert!(supervisor.try_event().is_none());
    assert!(supervisor.cancel(job_id).is_err());
    fs::remove_dir_all(&root).unwrap();
}
