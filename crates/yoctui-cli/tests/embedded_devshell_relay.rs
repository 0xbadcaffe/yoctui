//! Exercise the installed relay path with an owned fake BitBake terminal handoff.
use std::{fs, os::unix::fs::PermissionsExt, time::Duration};

async fn fake_handoff(wrapper_exit: i32) {
    let root = tempfile::tempdir().unwrap();
    let build = root.path().join("selected build");
    let task = build.join("tmp/work/helper/sources");
    fs::create_dir_all(&task).unwrap();
    let runtime = root.path().join("private runtime");
    fs::create_dir(&runtime).unwrap();
    fs::set_permissions(&runtime, fs::Permissions::from_mode(0o700)).unwrap();
    fs::create_dir(runtime.join("yoctui")).unwrap();
    fs::set_permissions(runtime.join("yoctui"), fs::Permissions::from_mode(0o700)).unwrap();
    let phonehome = build.join("oe-gnome-terminal-phonehome");
    let wrapper = task.join("generated devshell wrapper");
    let bitbake = root.path().join("fake-bitbake");
    fs::write(&phonehome, "#!/bin/sh\nexec \"$2\"\n").unwrap();
    fs::write(
        &wrapper,
        format!("#!/bin/sh\nprintf 'OWNED_DEVSHELL_WRAPPER\\n'\npwd\nexit {wrapper_exit}\n"),
    )
    .unwrap();
    fs::write(
        &bitbake,
        r#"#!/usr/bin/python3
import os, shlex, subprocess, sys, tempfile
from pathlib import Path
assert sys.argv[1:] == ['qemu-helper-native', '-c', 'devshell']
assert os.environ['OE_TERMINAL'] == 'custom'
additions = os.environ['BB_ENV_PASSTHROUGH_ADDITIONS'].split()
assert all(item in additions for item in ['OE_TERMINAL', 'OE_TERMINAL_CUSTOMCMD'])
build = Path.cwd()
task = build / 'tmp/work/helper/sources'
with tempfile.NamedTemporaryFile() as pidfile:
    command = shlex.join([str(build / 'oe-gnome-terminal-phonehome'), pidfile.name,
                          str(task / 'generated devshell wrapper')])
    argv = shlex.split(os.environ['OE_TERMINAL_CUSTOMCMD'].replace('{command}', command))
    result = subprocess.run(argv, cwd=task, timeout=5)
sys.exit(result.returncode)
"#,
    )
    .unwrap();
    for path in [&phonehome, &wrapper, &bitbake] {
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let child = tokio::process::Command::new(env!("CARGO_BIN_EXE_yoctui"))
        .arg("__menuconfig-relay")
        .arg("--bitbake")
        .arg(&bitbake)
        .args(["--", "qemu-helper-native", "-c", "devshell"])
        .current_dir(&build)
        .env("XDG_RUNTIME_DIR", &runtime)
        .env("BB_ENV_PASSTHROUGH_ADDITIONS", "MACHINE DISTRO")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let relay_pid = child.id().unwrap();
    let output = tokio::time::timeout(Duration::from_secs(10), child.wait_with_output())
        .await
        .unwrap()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.success(),
        wrapper_exit == 0,
        "{stdout}\n{stderr}"
    );
    assert!(
        stdout.contains("OWNED_DEVSHELL_WRAPPER"),
        "{stdout}\n{stderr}"
    );
    assert!(stdout.contains(task.to_str().unwrap()), "{stdout}");
    if wrapper_exit != 0 {
        assert!(stderr.contains("menuconfig exited"), "{stderr}");
    }
    assert!(
        !root
            .path()
            .join(format!(
                "private runtime/yoctui/menuconfig-{relay_pid}.sock"
            ))
            .exists()
    );
    assert!(
        phonehome.is_file() && wrapper.is_file(),
        "source fixtures must not be deleted by cleanup"
    );
}

#[tokio::test]
async fn embedded_devshell_fake_handoff_runs_in_the_selected_task_directory_and_cleans_socket() {
    fake_handoff(0).await;
}

#[tokio::test]
async fn embedded_devshell_fake_handoff_preserves_failure_and_cleans_socket() {
    fake_handoff(7).await;
}
