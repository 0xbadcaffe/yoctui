use super::*;

#[test]
fn poky_clone_retries_only_text_file_busy_spawn_errors() {
    assert!(is_transient_spawn_error(
        &std::io::Error::from_raw_os_error(libc::ETXTBSY,)
    ));
    assert!(!is_transient_spawn_error(
        &std::io::Error::from_raw_os_error(libc::ENOENT),
    ));
}
