use super::*;
use crate::{CapabilityReason, CapabilityRecord, YoctoEnvironmentIdentity};

const REQUIRED: &[&str] = &[
    "oe-init-build-env",
    "bitbake",
    "devtool",
    "recipetool",
    "bitbake-layers",
    "runqemu",
    "wic",
    "kas",
    "oe-pkgdata-util",
    "bitbake-getvar",
    "bitbake-diffsigs",
    "bitbake-dumpsig",
    "oe-find-native-sysroot",
    "sstate-cache-management.sh",
    "buildhistory-diff",
    "yocto-check-layer",
    "yocto-layer",
    "yocto-bsp",
    "yocto-kernel",
    "pybootchartgui",
    "toaster",
    "resulttool",
    "oe-selftest",
    "bitbake-selftest",
];

fn reason(message: &str) -> CapabilityReason {
    CapabilityReason::new("test.unavailable", message, None).unwrap()
}

mod compatibility_utilities_catalog_covers_every_registered_executable;

mod compatibility_utilities_unknown_snapshot_never_uses_host_path;

mod compatibility_utilities_preserves_partial_and_unavailable_reasons;
