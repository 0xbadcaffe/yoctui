use super::*;
use std::{
    fs,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);

#[tokio::test]
async fn kernel_debug_worker_installs_typed_tool_presence_without_process_launch() {
    use yoctui_model::{Action, App, Effect, KernelDebugAction as A, Screen};
    let mut app = App::new(32, 4096);
    app.onboarding.open = false;
    app.screen = Screen::Kernel;
    let effect = yoctui_model::update(&mut app, Action::KernelDebug(A::Open)).unwrap();
    assert!(matches!(effect, Effect::KernelDebug(_)));
    let mut io = KernelDebugIo::default();
    io.submit(effect);
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while !io.poll(&mut app).await {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(app.kernel_debug.tools.is_some());
    assert!(app.kernel_debug.pending.is_none());
    assert!(app.daemon.pty_sessions.is_empty());
}

#[cfg(unix)]
#[test]
fn kernel_debug_fake_ssh_preserves_typed_argv_without_local_shell_evaluation() {
    use std::os::unix::fs::PermissionsExt;
    let root = std::env::temp_dir().join(format!(
        "yoctui-kernel-debug-argv-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&root).unwrap();
    let program = root.join("fake-ssh");
    fs::write(&program, "#!/bin/sh\nprintf '%s\\n' \"$@\"\n").unwrap();
    fs::set_permissions(&program, fs::Permissions::from_mode(0o700)).unwrap();
    let tools = KernelDebugTools {
        cwd: root.clone(),
        programs: [("ssh".into(), program)].into(),
    };
    let mut draft = KernelDebugDraft::new(KernelDebugTool::Bpftrace);
    draft.host = "example.invalid".into();
    let request = prepare(&draft, &tools).unwrap();
    let output = std::process::Command::new(&request.program)
        .args(&request.arguments)
        .current_dir(&request.cwd)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .collect::<Vec<_>>(),
        request
            .arguments
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        request.arguments.last().unwrap(),
        "exec 'bpftrace' '-e' 'tracepoint:syscalls:sys_enter_openat { @[comm] = count(); }'"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn kernel_debug_discovery_is_presence_only_and_planning_runs_no_tool() {
    let tools = discover().unwrap();
    assert!(tools.cwd.is_dir());
    for program in tools.programs.values() {
        assert!(program.is_absolute());
        assert!(crate::terminal_launcher::executable(
            &fs::metadata(program).unwrap()
        ));
    }
    let mut draft = KernelDebugDraft::new(KernelDebugTool::Strace);
    draft.host = "example.invalid".into();
    draft.pid = "123".into();
    // A nonexistent DNS name is accepted only as a plan, never connected here.
    if tools.programs.contains_key("ssh") {
        let request = prepare(&draft, &tools).unwrap();
        assert!(request.arguments.iter().any(|arg| arg == "example.invalid"));
    }
}

#[test]
fn kernel_debug_preparation_rejects_missing_and_special_offline_files() {
    let root = std::env::temp_dir().join(format!(
        "yoctui-kernel-debug-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&root).unwrap();
    let program = std::env::current_exe().unwrap();
    let tools = KernelDebugTools {
        cwd: root.clone(),
        programs: [("gdb".into(), program)].into(),
    };
    let mut draft = KernelDebugDraft::new(KernelDebugTool::GdbCore);
    draft.symbols = root.join("vmlinux").display().to_string();
    draft.data = root.join("core").display().to_string();
    assert!(prepare(&draft, &tools).is_err());
    fs::write(&draft.symbols, "fixture").unwrap();
    fs::write(&draft.data, "fixture").unwrap();
    assert!(prepare(&draft, &tools).is_ok());
    draft.data = root.display().to_string();
    assert!(prepare(&draft, &tools).is_err());
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&draft.symbols, root.join("link")).unwrap();
        draft.data = root.join("link").display().to_string();
        assert!(prepare(&draft, &tools).is_err());
    }
    let mut unavailable = tools.clone();
    unavailable
        .programs
        .insert("gdb".into(), root.join("missing"));
    assert!(prepare(&draft, &unavailable).is_err());
    fs::remove_dir_all(root).unwrap();
}
