//! Client origin.
use super::*;

pub(crate) fn client_access_origin() -> ClientAccessOrigin {
    let values = ["SSH_CONNECTION", "SSH_CLIENT"]
        .into_iter()
        .filter_map(env::var_os)
        .map(|value| value.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    client_access_origin_from_values(values.iter().map(String::as_str))
}

pub(crate) fn client_access_origin_from_values<'a>(
    values: impl IntoIterator<Item = &'a str>,
) -> ClientAccessOrigin {
    let mut ssh_environment_present = false;
    for value in values {
        ssh_environment_present = true;
        let origin = parse_ssh_access_origin(value);
        if matches!(origin, ClientAccessOrigin::Ssh { .. }) {
            return origin;
        }
    }
    if ssh_environment_present {
        ClientAccessOrigin::SshUnknown
    } else {
        ClientAccessOrigin::Local
    }
}

pub(crate) fn parse_ssh_access_origin(value: &str) -> ClientAccessOrigin {
    let Some(client_ip) = value.split_whitespace().next() else {
        return ClientAccessOrigin::SshUnknown;
    };
    if client_ip.parse::<std::net::IpAddr>().is_ok() {
        ClientAccessOrigin::Ssh {
            client_ip: client_ip.into(),
        }
    } else {
        ClientAccessOrigin::SshUnknown
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StartupMetadataAuthority {
    DaemonSnapshot,
    OfflineFiles,
}

pub(crate) fn startup_metadata_authority(daemon_attached: bool) -> StartupMetadataAuthority {
    if daemon_attached {
        StartupMetadataAuthority::DaemonSnapshot
    } else {
        StartupMetadataAuthority::OfflineFiles
    }
}
