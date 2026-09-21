use super::*;

#[test]
fn compatibility_utilities_catalog_covers_every_registered_executable() {
    for executable in REQUIRED {
        assert!(
            UTILITY_COMPATIBILITY_CATALOG
                .iter()
                .any(|entry| entry.executables.contains(executable)),
            "missing utility {executable}"
        );
    }
}
