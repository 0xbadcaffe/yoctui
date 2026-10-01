use super::*;
use std::{
    fs,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);

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
