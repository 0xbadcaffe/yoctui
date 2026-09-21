use super::*;

#[test]
fn priority_rejects_values_outside_the_portable_nice_range() {
    assert_eq!(
        lower_process_priority(u32::MAX, -1).unwrap_err().kind(),
        io::ErrorKind::InvalidInput
    );
    assert_eq!(
        lower_process_priority(u32::MAX, 20).unwrap_err().kind(),
        io::ErrorKind::InvalidInput
    );
}
