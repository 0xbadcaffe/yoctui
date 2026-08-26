use serde::{Deserialize, Serialize};
use std::path::{Component, Path};
use thiserror::Error;

pub const ROOTFS_COMPOSITION_SCHEMA_VERSION: u16 = 1;
pub const MAX_ROOTFS_WIRE_PACKAGES: usize = 8_192;
pub const MAX_ROOTFS_WIRE_ENTRIES: usize = 65_536;
pub const MAX_ROOTFS_WIRE_LIMITATIONS: usize = 64;
pub const MAX_ROOTFS_WIRE_TEXT_BYTES: usize = 512;
pub const MAX_ROOTFS_WIRE_PATH_BYTES: usize = 4_096;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RootfsImageIdentityData {
    pub machine: String,
    pub image: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RootfsCompositionRequestData {
    pub generation: u64,
    pub image: RootfsImageIdentityData,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RootfsInstalledPackageData {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recipe: Option<String>,
    pub category: String,
    pub installed_size_bytes: u64,
    pub file_count: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RootfsEntryKindData {
    Directory,
    RegularFile,
    Symlink,
    Other,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RootfsEntryData {
    pub path: String,
    pub kind: RootfsEntryKindData,
    pub size_bytes: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum RootfsAuthorityData<T> {
    Available {
        records: T,
    },
    Partial {
        records: T,
        limitations: Vec<String>,
    },
    Unavailable {
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RootfsCompositionData {
    pub schema_version: u16,
    pub request: RootfsCompositionRequestData,
    pub installed_packages: RootfsAuthorityData<Vec<RootfsInstalledPackageData>>,
    pub filesystem_entries: RootfsAuthorityData<Vec<RootfsEntryData>>,
    #[serde(default)]
    pub limitations: Vec<String>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RootfsProtocolError {
    #[error("unsupported rootfs composition schema {0}")]
    UnsupportedSchema(u16),
    #[error("invalid rootfs composition request")]
    InvalidRequest,
    #[error("invalid rootfs composition record")]
    InvalidRecord,
    #[error("rootfs composition collection exceeds its protocol bound")]
    TooManyRecords,
    #[error("rootfs composition limitations exceed their protocol bound")]
    TooManyLimitations,
    #[error("unknown required rootfs composition variant")]
    UnknownRequiredVariant,
}

impl RootfsCompositionData {
    pub fn validate(&self) -> Result<(), RootfsProtocolError> {
        if self.schema_version != ROOTFS_COMPOSITION_SCHEMA_VERSION {
            return Err(RootfsProtocolError::UnsupportedSchema(self.schema_version));
        }
        validate_request(&self.request)?;
        validate_authority(
            &self.installed_packages,
            MAX_ROOTFS_WIRE_PACKAGES,
            |package| {
                valid_token(&package.name)
                    && valid_text(&package.category)
                    && !package.category.is_empty()
                    && package.recipe.as_deref().is_none_or(valid_text)
            },
        )?;
        validate_authority(&self.filesystem_entries, MAX_ROOTFS_WIRE_ENTRIES, |entry| {
            entry.kind != RootfsEntryKindData::Unknown
                && valid_logical_path(&entry.path)
                && entry.package.as_deref().is_none_or(valid_token)
        })?;
        validate_limitations(&self.limitations)
    }
}

fn validate_request(request: &RootfsCompositionRequestData) -> Result<(), RootfsProtocolError> {
    if request.generation == 0
        || !valid_token(&request.image.machine)
        || !valid_token(&request.image.image)
        || !valid_host_path(&request.image.path)
    {
        Err(RootfsProtocolError::InvalidRequest)
    } else {
        Ok(())
    }
}

fn validate_authority<T>(
    authority: &RootfsAuthorityData<Vec<T>>,
    maximum: usize,
    valid_record: impl Fn(&T) -> bool,
) -> Result<(), RootfsProtocolError> {
    match authority {
        RootfsAuthorityData::Available { records } => {
            validate_records(records, maximum, valid_record)
        }
        RootfsAuthorityData::Partial {
            records,
            limitations,
        } => {
            validate_records(records, maximum, valid_record)?;
            validate_limitations(limitations)
        }
        RootfsAuthorityData::Unavailable { reason } => {
            if reason.is_empty() || !valid_text(reason) {
                Err(RootfsProtocolError::InvalidRecord)
            } else {
                Ok(())
            }
        }
    }
}

fn validate_records<T>(
    records: &[T],
    maximum: usize,
    valid_record: impl Fn(&T) -> bool,
) -> Result<(), RootfsProtocolError> {
    if records.len() > maximum {
        Err(RootfsProtocolError::TooManyRecords)
    } else if records.iter().all(valid_record) {
        Ok(())
    } else {
        Err(RootfsProtocolError::InvalidRecord)
    }
}

fn validate_limitations(values: &[String]) -> Result<(), RootfsProtocolError> {
    if values.len() > MAX_ROOTFS_WIRE_LIMITATIONS {
        Err(RootfsProtocolError::TooManyLimitations)
    } else if values
        .iter()
        .all(|value| !value.is_empty() && valid_text(value))
    {
        Ok(())
    } else {
        Err(RootfsProtocolError::InvalidRecord)
    }
}

fn valid_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_ROOTFS_WIRE_TEXT_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'+'))
}

fn valid_text(value: &str) -> bool {
    value.len() <= MAX_ROOTFS_WIRE_TEXT_BYTES && !value.chars().any(char::is_control)
}

fn valid_host_path(value: &str) -> bool {
    let path = Path::new(value);
    value.len() <= MAX_ROOTFS_WIRE_PATH_BYTES
        && path.is_absolute()
        && path != Path::new("/")
        && !path.components().any(|component| {
            matches!(
                component,
                Component::CurDir | Component::ParentDir | Component::Prefix(_)
            )
        })
}

fn valid_logical_path(value: &str) -> bool {
    let path = Path::new(value);
    value.len() <= MAX_ROOTFS_WIRE_PATH_BYTES
        && path.is_absolute()
        && !path.components().any(|component| {
            matches!(
                component,
                Component::CurDir | Component::ParentDir | Component::Prefix(_)
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> RootfsCompositionData {
        RootfsCompositionData {
            schema_version: ROOTFS_COMPOSITION_SCHEMA_VERSION,
            request: RootfsCompositionRequestData {
                generation: 9,
                image: RootfsImageIdentityData {
                    machine: "qemux86-64".into(),
                    image: "core-image-minimal".into(),
                    path: "/build/tmp/deploy/images/qemux86-64/image.ext4".into(),
                },
            },
            installed_packages: RootfsAuthorityData::Partial {
                records: vec![RootfsInstalledPackageData {
                    name: "busybox".into(),
                    recipe: Some("busybox".into()),
                    category: "base".into(),
                    installed_size_bytes: 1_024,
                    file_count: 12,
                }],
                limitations: vec!["versions unavailable".into()],
            },
            filesystem_entries: RootfsAuthorityData::Available {
                records: vec![RootfsEntryData {
                    path: "/usr/bin/busybox".into(),
                    kind: RootfsEntryKindData::RegularFile,
                    size_bytes: 1_024,
                    package: Some("busybox".into()),
                }],
            },
            limitations: Vec::new(),
        }
    }

    #[test]
    fn ux_rootfs_protocol_round_trips_separate_authorities_and_exact_correlation() {
        let value = data();
        value.validate().unwrap();
        let encoded = serde_json::to_vec(&value).unwrap();
        let decoded: RootfsCompositionData = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, value);
        assert_eq!(decoded.request.generation, 9);
        assert_eq!(decoded.request.image.image, "core-image-minimal");
    }

    #[test]
    fn ux_rootfs_protocol_rejects_unknown_variants_paths_bounds_and_schema() {
        let mut value = data();
        value.schema_version = 2;
        assert_eq!(
            value.validate(),
            Err(RootfsProtocolError::UnsupportedSchema(2))
        );
        value = data();
        if let RootfsAuthorityData::Available { records } = &mut value.filesystem_entries {
            records[0].kind = RootfsEntryKindData::Unknown;
        }
        assert_eq!(value.validate(), Err(RootfsProtocolError::InvalidRecord));
        if let RootfsAuthorityData::Available { records } = &mut value.filesystem_entries {
            records[0].kind = RootfsEntryKindData::RegularFile;
            records[0].path = "../escape".into();
        }
        assert_eq!(value.validate(), Err(RootfsProtocolError::InvalidRecord));

        value = data();
        value.limitations = vec!["x".into(); MAX_ROOTFS_WIRE_LIMITATIONS + 1];
        assert_eq!(
            value.validate(),
            Err(RootfsProtocolError::TooManyLimitations)
        );
    }
}
