use super::*;

#[test]
fn cli_control_retries_only_text_file_busy_spawn_failures() {
    assert!(is_transient_spawn_error(&io::Error::from_raw_os_error(
        libc::ETXTBSY,
    )));
    assert!(!is_transient_spawn_error(&io::Error::from_raw_os_error(
        libc::ENOENT,
    )));
}
