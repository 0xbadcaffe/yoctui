use super::*;

pub(super) use super::platform_inspection_operation::initialized_platform_environment as initialized_metadata_environment;

const PROCESS_BACKEND_MESSAGE: &str =
    "Kernel and firmware inspection require --backend bridge and an attached Yoctui daemon.";

pub(crate) fn metadata_backend_start_required(
    backend_kind: &Backend,
    authoritative: bool,
) -> Result<bool, &'static str> {
    if authoritative {
        return Ok(false);
    }
    match backend_kind {
        Backend::Bridge => Ok(true),
        Backend::Process => Err(PROCESS_BACKEND_MESSAGE),
    }
}
