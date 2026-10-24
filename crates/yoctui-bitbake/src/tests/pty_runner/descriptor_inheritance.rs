use super::*;

#[test]
fn unrelated_children_do_not_inherit_pty_descriptors() {
    let (master, slave) = open_pty(PtyDimensions {
        columns: 80,
        rows: 24,
    })
    .unwrap();
    let output = std::process::Command::new("/usr/bin/python3")
        .args([
            "-c",
            "import os, sys\nfor fd in map(int, sys.argv[1:]):\n try: os.fstat(fd)\n except OSError: continue\n raise SystemExit('inherited PTY descriptor ' + str(fd))",
        ])
        .arg(master.as_raw_fd().to_string())
        .arg(slave.as_raw_fd().to_string())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
