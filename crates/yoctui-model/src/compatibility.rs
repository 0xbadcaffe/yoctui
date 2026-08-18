use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Component, Path, PathBuf},
};
use thiserror::Error;

pub const MAX_ENVIRONMENT_IDENTITY_TEXT_BYTES: usize = 1_024;
pub const MAX_ENVIRONMENT_IDENTITY_PATH_BYTES: usize = 4_096;
pub const MAX_ENVIRONMENT_SOURCE_ROOTS: usize = 256;
pub const MAX_ENVIRONMENT_LAYER_SERIES: usize = 256;
pub const MAX_ENVIRONMENT_TOOLS: usize = 256;
pub const MAX_LAYER_COMPATIBLE_SERIES: usize = 64;

/// Authoritative origins accepted by the environment identity model.
///
/// There is deliberately no branch-name, directory-name, nearest-tag, or
/// inferred-version variant. Those values may be diagnostics, but cannot
/// become authoritative identity through this type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentityAuthority {
    BackendHandshake,
    BitBakeDatastore,
    BitBakeVersionProbe,
    ConfiguredLayerMetadata,
    ExecutableProbe,
    InitializedEnvironment,
    ProtocolNegotiation,
    ReleaseMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum AuthoritativeValue<T> {
    #[default]
    Unknown,
    Detected {
        value: T,
        authority: IdentityAuthority,
    },
}

impl<T> AuthoritativeValue<T> {
    pub const fn unknown() -> Self {
        Self::Unknown
    }

    pub const fn detected(value: T, authority: IdentityAuthority) -> Self {
        Self::Detected { value, authority }
    }

    pub const fn value(&self) -> Option<&T> {
        match self {
            Self::Unknown => None,
            Self::Detected { value, .. } => Some(value),
        }
    }

    pub const fn authority(&self) -> Option<IdentityAuthority> {
        match self {
            Self::Unknown => None,
            Self::Detected { authority, .. } => Some(*authority),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ReleaseIdentity {
    pub name: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DistroIdentity {
    pub name: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceRootKind {
    CoreBase,
    OpenEmbeddedCore,
    Poky,
    Layer,
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SourceRootIdentity {
    pub kind: SourceRootKind,
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct LayerSeriesIdentity {
    pub layer: String,
    pub root: PathBuf,
    pub compatible_series: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ToolIdentity {
    pub id: String,
    pub executable: PathBuf,
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct BackendIdentity {
    pub name: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ProtocolIdentity {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct YoctoEnvironmentIdentity {
    pub build_directory: AuthoritativeValue<PathBuf>,
    pub source_roots: AuthoritativeValue<Vec<SourceRootIdentity>>,
    pub bitbake_version: AuthoritativeValue<String>,
    pub oe_core: AuthoritativeValue<ReleaseIdentity>,
    pub poky: AuthoritativeValue<ReleaseIdentity>,
    pub distro: AuthoritativeValue<DistroIdentity>,
    pub machine: AuthoritativeValue<String>,
    pub layer_series: AuthoritativeValue<Vec<LayerSeriesIdentity>>,
    pub available_tools: AuthoritativeValue<Vec<ToolIdentity>>,
    pub backend: AuthoritativeValue<BackendIdentity>,
    pub protocol: AuthoritativeValue<ProtocolIdentity>,
}

impl YoctoEnvironmentIdentity {
    pub fn normalize(mut self) -> Result<Self, EnvironmentIdentityError> {
        validate_detected(
            "build_directory",
            &self.build_directory,
            &[
                IdentityAuthority::BackendHandshake,
                IdentityAuthority::BitBakeDatastore,
                IdentityAuthority::InitializedEnvironment,
            ],
            |path| valid_absolute_path(path),
        )?;
        validate_detected(
            "bitbake_version",
            &self.bitbake_version,
            &[
                IdentityAuthority::BackendHandshake,
                IdentityAuthority::BitBakeVersionProbe,
            ],
            |value| valid_text(value),
        )?;
        validate_detected(
            "oe_core",
            &self.oe_core,
            &[
                IdentityAuthority::BitBakeDatastore,
                IdentityAuthority::ConfiguredLayerMetadata,
                IdentityAuthority::ReleaseMetadata,
            ],
            valid_release,
        )?;
        validate_detected(
            "poky",
            &self.poky,
            &[
                IdentityAuthority::BitBakeDatastore,
                IdentityAuthority::ConfiguredLayerMetadata,
                IdentityAuthority::ReleaseMetadata,
            ],
            valid_release,
        )?;
        validate_detected(
            "distro",
            &self.distro,
            &[IdentityAuthority::BitBakeDatastore],
            |value| valid_token(&value.name) && value.version.as_deref().is_none_or(valid_text),
        )?;
        validate_detected(
            "machine",
            &self.machine,
            &[IdentityAuthority::BitBakeDatastore],
            |value| valid_token(value),
        )?;
        validate_detected(
            "backend",
            &self.backend,
            &[IdentityAuthority::BackendHandshake],
            |value| valid_token(&value.name) && value.version.as_deref().is_none_or(valid_text),
        )?;
        validate_detected(
            "protocol",
            &self.protocol,
            &[IdentityAuthority::ProtocolNegotiation],
            |value| valid_token(&value.name) && valid_text(&value.version),
        )?;

        normalize_source_roots(&mut self.source_roots)?;
        normalize_layer_series(&mut self.layer_series)?;
        normalize_tools(&mut self.available_tools)?;
        Ok(self)
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum EnvironmentIdentityError {
    #[error("invalid authoritative environment identity field: {0}")]
    InvalidField(&'static str),
    #[error("invalid authority {authority:?} for environment identity field {field}")]
    InvalidAuthority {
        field: &'static str,
        authority: IdentityAuthority,
    },
    #[error("too many entries in environment identity field {field}: {count} > {limit}")]
    TooManyEntries {
        field: &'static str,
        count: usize,
        limit: usize,
    },
    #[error("conflicting duplicate environment identity for {field}: {key}")]
    ConflictingDuplicate { field: &'static str, key: String },
}

fn validate_detected<T>(
    field: &'static str,
    detected: &AuthoritativeValue<T>,
    authorities: &[IdentityAuthority],
    valid: impl FnOnce(&T) -> bool,
) -> Result<(), EnvironmentIdentityError> {
    let AuthoritativeValue::Detected { value, authority } = detected else {
        return Ok(());
    };
    if !authorities.contains(authority) {
        return Err(EnvironmentIdentityError::InvalidAuthority {
            field,
            authority: *authority,
        });
    }
    if !valid(value) {
        return Err(EnvironmentIdentityError::InvalidField(field));
    }
    Ok(())
}

fn normalize_source_roots(
    value: &mut AuthoritativeValue<Vec<SourceRootIdentity>>,
) -> Result<(), EnvironmentIdentityError> {
    validate_detected(
        "source_roots",
        value,
        &[
            IdentityAuthority::BitBakeDatastore,
            IdentityAuthority::ConfiguredLayerMetadata,
            IdentityAuthority::InitializedEnvironment,
        ],
        |roots| {
            !roots.is_empty()
                && roots.len() <= MAX_ENVIRONMENT_SOURCE_ROOTS
                && roots.iter().all(|root| {
                    valid_absolute_path(&root.path)
                        && match &root.kind {
                            SourceRootKind::Other(label) => valid_token(label),
                            _ => true,
                        }
                })
        },
    )?;
    let AuthoritativeValue::Detected { value: roots, .. } = value else {
        return Ok(());
    };
    roots.sort();
    roots.dedup();
    Ok(())
}

fn normalize_layer_series(
    value: &mut AuthoritativeValue<Vec<LayerSeriesIdentity>>,
) -> Result<(), EnvironmentIdentityError> {
    validate_detected(
        "layer_series",
        value,
        &[IdentityAuthority::ConfiguredLayerMetadata],
        |layers| {
            !layers.is_empty()
                && layers.len() <= MAX_ENVIRONMENT_LAYER_SERIES
                && layers.iter().all(|layer| {
                    valid_token(&layer.layer)
                        && valid_absolute_path(&layer.root)
                        && !layer.compatible_series.is_empty()
                        && layer.compatible_series.len() <= MAX_LAYER_COMPATIBLE_SERIES
                        && layer
                            .compatible_series
                            .iter()
                            .all(|series| valid_token(series))
                })
        },
    )?;
    let AuthoritativeValue::Detected { value: layers, .. } = value else {
        return Ok(());
    };
    for layer in layers.iter_mut() {
        layer.compatible_series.sort();
        layer.compatible_series.dedup();
    }
    reject_conflicts(
        "layer_series",
        layers,
        |layer| layer.layer.clone(),
        |layer| (layer.root.clone(), layer.compatible_series.clone()),
    )?;
    layers.sort();
    layers.dedup();
    Ok(())
}

fn normalize_tools(
    value: &mut AuthoritativeValue<Vec<ToolIdentity>>,
) -> Result<(), EnvironmentIdentityError> {
    validate_detected(
        "available_tools",
        value,
        &[
            IdentityAuthority::ExecutableProbe,
            IdentityAuthority::InitializedEnvironment,
        ],
        |tools| {
            !tools.is_empty()
                && tools.len() <= MAX_ENVIRONMENT_TOOLS
                && tools.iter().all(|tool| {
                    valid_token(&tool.id)
                        && valid_absolute_path(&tool.executable)
                        && tool.version.as_deref().is_none_or(valid_text)
                })
        },
    )?;
    let AuthoritativeValue::Detected { value: tools, .. } = value else {
        return Ok(());
    };
    reject_conflicts(
        "available_tools",
        tools,
        |tool| tool.id.clone(),
        |tool| (tool.executable.clone(), tool.version.clone()),
    )?;
    tools.sort();
    tools.dedup();
    Ok(())
}

fn reject_conflicts<T, V: PartialEq>(
    field: &'static str,
    values: &[T],
    key: impl Fn(&T) -> String,
    identity: impl Fn(&T) -> V,
) -> Result<(), EnvironmentIdentityError> {
    let mut seen = BTreeMap::new();
    for value in values {
        let key = key(value);
        let identity = identity(value);
        if seen.get(&key).is_some_and(|previous| previous != &identity) {
            return Err(EnvironmentIdentityError::ConflictingDuplicate { field, key });
        }
        seen.insert(key, identity);
    }
    Ok(())
}

fn valid_release(value: &ReleaseIdentity) -> bool {
    (value.name.is_some() || value.version.is_some())
        && value.name.as_deref().is_none_or(valid_text)
        && value.version.as_deref().is_none_or(valid_text)
}

fn valid_text(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_ENVIRONMENT_IDENTITY_TEXT_BYTES
        && value.trim() == value
        && !value.chars().any(char::is_control)
}

fn valid_token(value: &str) -> bool {
    valid_text(value) && !value.chars().any(char::is_whitespace)
}

fn valid_absolute_path(path: &Path) -> bool {
    path.is_absolute()
        && path != Path::new("/")
        && path.as_os_str().as_encoded_bytes().len() <= MAX_ENVIRONMENT_IDENTITY_PATH_BYTES
        && path
            .components()
            .all(|component| matches!(component, Component::RootDir | Component::Normal(_)))
}

#[cfg(test)]
mod environment_identity {
    use super::*;

    fn full_identity() -> YoctoEnvironmentIdentity {
        YoctoEnvironmentIdentity {
            build_directory: AuthoritativeValue::detected(
                PathBuf::from("/work/poky/build"),
                IdentityAuthority::InitializedEnvironment,
            ),
            source_roots: AuthoritativeValue::detected(
                vec![
                    SourceRootIdentity {
                        kind: SourceRootKind::Layer,
                        path: "/work/poky/meta-poky".into(),
                    },
                    SourceRootIdentity {
                        kind: SourceRootKind::CoreBase,
                        path: "/work/poky".into(),
                    },
                ],
                IdentityAuthority::ConfiguredLayerMetadata,
            ),
            bitbake_version: AuthoritativeValue::detected(
                "2.8.1".into(),
                IdentityAuthority::BitBakeVersionProbe,
            ),
            oe_core: AuthoritativeValue::detected(
                ReleaseIdentity {
                    name: Some("scarthgap".into()),
                    version: Some("5.0.19".into()),
                },
                IdentityAuthority::ReleaseMetadata,
            ),
            poky: AuthoritativeValue::detected(
                ReleaseIdentity {
                    name: Some("scarthgap".into()),
                    version: Some("5.0.19".into()),
                },
                IdentityAuthority::ReleaseMetadata,
            ),
            distro: AuthoritativeValue::detected(
                DistroIdentity {
                    name: "poky".into(),
                    version: Some("5.0.19".into()),
                },
                IdentityAuthority::BitBakeDatastore,
            ),
            machine: AuthoritativeValue::detected(
                "qemux86-64".into(),
                IdentityAuthority::BitBakeDatastore,
            ),
            layer_series: AuthoritativeValue::detected(
                vec![
                    LayerSeriesIdentity {
                        layer: "meta-poky".into(),
                        root: "/work/poky/meta-poky".into(),
                        compatible_series: vec!["scarthgap".into()],
                    },
                    LayerSeriesIdentity {
                        layer: "core".into(),
                        root: "/work/poky/meta".into(),
                        compatible_series: vec!["scarthgap".into(), "nanbield".into()],
                    },
                ],
                IdentityAuthority::ConfiguredLayerMetadata,
            ),
            available_tools: AuthoritativeValue::detected(
                vec![ToolIdentity {
                    id: "bitbake".into(),
                    executable: "/work/poky/bitbake/bin/bitbake".into(),
                    version: Some("2.8.1".into()),
                }],
                IdentityAuthority::ExecutableProbe,
            ),
            backend: AuthoritativeValue::detected(
                BackendIdentity {
                    name: "tinfoil".into(),
                    version: Some("1".into()),
                },
                IdentityAuthority::BackendHandshake,
            ),
            protocol: AuthoritativeValue::detected(
                ProtocolIdentity {
                    name: "yoctui-daemon".into(),
                    version: "1.0".into(),
                },
                IdentityAuthority::ProtocolNegotiation,
            ),
        }
    }

    #[test]
    fn environment_identity_normalizes_deterministically_without_collapsing_mixed_series() {
        let normalized = full_identity().normalize().unwrap();
        let roots = normalized.source_roots.value().unwrap();
        assert_eq!(roots[0].kind, SourceRootKind::CoreBase);
        let layers = normalized.layer_series.value().unwrap();
        assert_eq!(layers[0].layer, "core");
        assert_eq!(
            layers[0].compatible_series,
            ["nanbield".to_string(), "scarthgap".to_string()]
        );
        assert_ne!(layers[0].compatible_series, layers[1].compatible_series);
    }

    #[test]
    fn environment_identity_preserves_unknown_for_every_partial_field() {
        let identity = YoctoEnvironmentIdentity {
            machine: AuthoritativeValue::detected(
                "qemuarm64".into(),
                IdentityAuthority::BitBakeDatastore,
            ),
            ..YoctoEnvironmentIdentity::default()
        }
        .normalize()
        .unwrap();
        assert_eq!(
            identity.machine.value().map(String::as_str),
            Some("qemuarm64")
        );
        assert_eq!(identity.bitbake_version, AuthoritativeValue::Unknown);
        assert_eq!(identity.poky, AuthoritativeValue::Unknown);
        assert_eq!(identity.available_tools, AuthoritativeValue::Unknown);
    }

    #[test]
    fn environment_identity_rejects_weak_or_wrong_authority() {
        let mut identity = full_identity();
        identity.bitbake_version =
            AuthoritativeValue::detected("2.8.1".into(), IdentityAuthority::ReleaseMetadata);
        assert!(matches!(
            identity.normalize(),
            Err(EnvironmentIdentityError::InvalidAuthority {
                field: "bitbake_version",
                ..
            })
        ));
    }

    #[test]
    fn environment_identity_rejects_invalid_paths_text_and_empty_detected_collections() {
        let mut relative = full_identity();
        relative.build_directory = AuthoritativeValue::detected(
            "relative/build".into(),
            IdentityAuthority::InitializedEnvironment,
        );
        assert_eq!(
            relative.normalize(),
            Err(EnvironmentIdentityError::InvalidField("build_directory"))
        );

        let mut control = full_identity();
        control.machine =
            AuthoritativeValue::detected("qemu\narm".into(), IdentityAuthority::BitBakeDatastore);
        assert_eq!(
            control.normalize(),
            Err(EnvironmentIdentityError::InvalidField("machine"))
        );

        let mut empty = full_identity();
        empty.available_tools =
            AuthoritativeValue::detected(Vec::new(), IdentityAuthority::ExecutableProbe);
        assert_eq!(
            empty.normalize(),
            Err(EnvironmentIdentityError::InvalidField("available_tools"))
        );
    }

    #[test]
    fn environment_identity_rejects_conflicting_duplicate_tools_and_layers() {
        let mut tools = full_identity();
        let AuthoritativeValue::Detected { value, .. } = &mut tools.available_tools else {
            unreachable!();
        };
        value.push(ToolIdentity {
            id: "bitbake".into(),
            executable: "/other/bin/bitbake".into(),
            version: Some("2.8.1".into()),
        });
        assert!(matches!(
            tools.normalize(),
            Err(EnvironmentIdentityError::ConflictingDuplicate {
                field: "available_tools",
                ..
            })
        ));

        let mut layers = full_identity();
        let AuthoritativeValue::Detected { value, .. } = &mut layers.layer_series else {
            unreachable!();
        };
        value.push(LayerSeriesIdentity {
            layer: "core".into(),
            root: "/different/meta".into(),
            compatible_series: vec!["scarthgap".into()],
        });
        assert!(matches!(
            layers.normalize(),
            Err(EnvironmentIdentityError::ConflictingDuplicate {
                field: "layer_series",
                ..
            })
        ));
    }

    #[test]
    fn environment_identity_deduplicates_exact_authoritative_records() {
        let mut identity = full_identity();
        let AuthoritativeValue::Detected { value, .. } = &mut identity.source_roots else {
            unreachable!();
        };
        value.push(value[0].clone());
        let normalized = identity.normalize().unwrap();
        assert_eq!(normalized.source_roots.value().unwrap().len(), 2);
    }
}
