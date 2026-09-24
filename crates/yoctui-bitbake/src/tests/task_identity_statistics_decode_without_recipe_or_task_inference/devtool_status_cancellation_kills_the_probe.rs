use super::*;

#[cfg(unix)]
#[tokio::test]
async fn devtool_status_cancellation_kills_the_probe() {
    let root = fixture_script("devtool-status-cancel");
    fs::create_dir_all(&root).unwrap();
    let pid_file = root.join("probe.pid");
    let devtool = root.join("devtool");
    let git = root.join("git");
    fs::write(
        &devtool,
        format!(
            "#!/bin/sh\nprintf '%s' \"$$\" > '{}'\nexec sleep 60\n",
            pid_file.display()
        ),
    )
    .unwrap();
    fs::write(&git, "#!/bin/sh\nexit 0\n").unwrap();
    for script in [&devtool, &git] {
        let mut permissions = fs::metadata(script).unwrap().permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(script, permissions).unwrap();
    }
    let identity = RecipeIdentity {
        name: "busybox".into(),
        file: "/layers/core/busybox_1.0.bb".into(),
    };
    let compatibility = devtool_compatibility(&root, &devtool);
    let inspector = DevtoolInspector::with_programs(devtool, git);
    let build_dir = root.clone();
    let worker = tokio::spawn(async move {
        inspector
            .inspect_with_compatibility(&build_dir, identity, &compatibility, 1)
            .await
    });

    let pid = tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            if let Ok(value) = fs::read_to_string(&pid_file) {
                break value.parse::<u32>().unwrap();
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("Devtool probe did not start");
    worker.abort();
    let _ = worker.await;

    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while Path::new(&format!("/proc/{pid}")).exists() {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("cancelled Devtool probe remained alive");
    fs::remove_dir_all(root).unwrap();
}
