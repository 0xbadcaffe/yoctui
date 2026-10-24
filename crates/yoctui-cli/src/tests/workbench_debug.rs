use super::*;
use yoctui_model::WorkbenchDebugConfig;

#[test]
fn workbench_gdb_preparation_rejects_source_files_and_missing_debuggers() {
    let (_temp, root, source) = fixture();
    let mut config = WorkbenchDebugConfig {
        local: true,
        executable: source.display().to_string(),
        ..Default::default()
    };
    let prepare = |config| {
        execute(R::PrepareDebug {
            session: 1,
            generation: 1,
            root: root.clone(),
            config,
            breakpoints: vec![],
        })
    };
    assert!(matches!(
        prepare(config.clone()),
        W::DebugPrepared { result: Err(_), .. }
    ));
    fs::write(&source, b"\x7fELFfixture").unwrap();
    config.gdb = "/missing/toolchain/gdb".into();
    assert!(matches!(
        prepare(config),
        W::DebugPrepared { result: Err(_), .. }
    ));
}

#[test]
#[ignore = "requires host cc and GDB with ptrace; run explicitly for live validation"]
fn workbench_gdb_hits_a_real_breakpoint_with_spaces_and_source_path_mapping() {
    let (_temp, root, _) = fixture();
    let source = root.join("program with spaces.c");
    let binary = root.join("program with spaces");
    fs::write(&source,"int helper(int n) {\n  volatile int result = n + 7;\n  return result;\n}\nint main(void) { return helper(35); }\n").unwrap();
    let result = std::process::Command::new("cc")
        .args(["-g3", "-O0"])
        .arg(format!(
            "-fdebug-prefix-map={}=/compiled/source",
            root.display()
        ))
        .arg(&source)
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let config = WorkbenchDebugConfig {
        local: true,
        executable: binary.display().to_string(),
        source_prefix: "/compiled/source".into(),
        ..Default::default()
    };
    let W::DebugPrepared {
        result: Ok(request),
        ..
    } = execute(R::PrepareDebug {
        session: 1,
        generation: 1,
        root: root.clone(),
        config,
        breakpoints: vec![(source, 2)],
    })
    else {
        panic!("debug preparation failed")
    };
    let output = std::process::Command::new(&request.program)
        .args(&request.arguments)
        .args([
            "--batch",
            "-ex",
            "run",
            "-ex",
            "bt",
            "-ex",
            "info registers",
            "-ex",
            "list",
            "-ex",
            "continue",
        ])
        .current_dir(&request.cwd)
        .output()
        .unwrap();
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    if let Some(path) = std::env::var_os("YOCTUI_WORKBENCH_GDB_CAPTURE") {
        fs::write(path, &text).unwrap();
    }
    assert!(output.status.success(), "{text}");
    assert!(
        text.contains("Breakpoint 1, helper") && text.contains("#0") && text.contains("#1"),
        "{text}"
    );
    assert!(
        text.contains("volatile int result") && (text.contains("rip") || text.contains(" pc ")),
        "{text}"
    );
}
