use super::*;

fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("yoctui-error-log-{}-{name}", std::process::id()))
}

#[test]
fn error_log_reads_text_and_rejects_oversized_files() {
    let path = temp_path("read");
    std::fs::write(&path, "line one\nline two\n").unwrap();
    assert_eq!(read(&path).unwrap(), "line one\nline two\n");
    std::fs::remove_file(&path).unwrap();

    let path = temp_path("large");
    let file = std::fs::File::create(&path).unwrap();
    file.set_len(MAX_ERROR_LOG_BYTES + 1).unwrap();
    let error = read(&path).unwrap_err().to_string();
    assert!(error.contains("8 MiB"), "{error}");
    std::fs::remove_file(path).unwrap();
}

#[cfg(unix)]
#[test]
fn error_log_rejects_symbolic_links() {
    let target = temp_path("target");
    let link = temp_path("link");
    std::fs::write(&target, "secret").unwrap();
    std::os::unix::fs::symlink(&target, &link).unwrap();
    let error = read(&link).unwrap_err().to_string();
    assert!(error.contains("symbolic link"), "{error}");
    std::fs::remove_file(link).unwrap();
    std::fs::remove_file(target).unwrap();
}
