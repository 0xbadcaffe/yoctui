use super::*;

#[test]
fn compatibility_probe_spawn_retry_classifies_only_text_file_busy_as_transient() {
    assert!(is_transient_probe_spawn_error(
        &io::Error::from_raw_os_error(libc::ETXTBSY)
    ));
    assert!(!is_transient_probe_spawn_error(
        &io::Error::from_raw_os_error(libc::ENOENT)
    ));
}
