//! Rootfs mapping.
use super::*;

pub fn backend_event_from_rootfs_data(
    data: yoctui_protocol::rootfs::RootfsCompositionData,
) -> Result<BackendEvent, String> {
    use yoctui_protocol::rootfs::{RootfsAuthorityData as WireAuthority, RootfsEntryKindData};

    data.validate().map_err(|error| error.to_string())?;
    let image = yoctui_model::ImageArtifactIdentity {
        machine: data.request.image.machine,
        image: data.request.image.image,
        path: data.request.image.path.into(),
    };
    let request = yoctui_model::RootfsCompositionRequest {
        generation: data.request.generation,
        image: image.clone(),
    };
    let installed_packages = match data.installed_packages {
        WireAuthority::Available { records } => {
            yoctui_model::RootfsAuthority::Available(yoctui_model::RootfsPackageInventory {
                packages: records
                    .into_iter()
                    .map(|record| yoctui_model::RootfsInstalledPackage {
                        identity: yoctui_model::PackageIdentity::new(record.name),
                        recipe: record.recipe,
                        category: record.category,
                        installed_size_bytes: record.installed_size_bytes,
                        file_count: record.file_count,
                    })
                    .collect(),
            })
        }
        WireAuthority::Partial {
            records,
            limitations,
        } => yoctui_model::RootfsAuthority::Partial {
            value: yoctui_model::RootfsPackageInventory {
                packages: records
                    .into_iter()
                    .map(|record| yoctui_model::RootfsInstalledPackage {
                        identity: yoctui_model::PackageIdentity::new(record.name),
                        recipe: record.recipe,
                        category: record.category,
                        installed_size_bytes: record.installed_size_bytes,
                        file_count: record.file_count,
                    })
                    .collect(),
            },
            limitations,
        },
        WireAuthority::Unavailable { reason } => {
            yoctui_model::RootfsAuthority::Unavailable { reason }
        }
    };
    let convert_entries = |records: Vec<yoctui_protocol::rootfs::RootfsEntryData>| {
        yoctui_model::RootfsFilesystemTree {
            entries: records
                .into_iter()
                .map(|record| yoctui_model::RootfsEntry {
                    identity: yoctui_model::RootfsPathIdentity(record.path.into()),
                    kind: match record.kind {
                        RootfsEntryKindData::Directory => yoctui_model::RootfsEntryKind::Directory,
                        RootfsEntryKindData::RegularFile => {
                            yoctui_model::RootfsEntryKind::RegularFile
                        }
                        RootfsEntryKindData::Symlink => yoctui_model::RootfsEntryKind::Symlink,
                        RootfsEntryKindData::Other => yoctui_model::RootfsEntryKind::Other,
                        RootfsEntryKindData::Unknown => {
                            unreachable!("validated rootfs wire data rejects unknown kinds")
                        }
                    },
                    size_bytes: record.size_bytes,
                    package: record.package.map(yoctui_model::PackageIdentity::new),
                })
                .collect(),
        }
    };
    let filesystem_tree = match data.filesystem_entries {
        WireAuthority::Available { records } => {
            yoctui_model::RootfsAuthority::Available(convert_entries(records))
        }
        WireAuthority::Partial {
            records,
            limitations,
        } => yoctui_model::RootfsAuthority::Partial {
            value: convert_entries(records),
            limitations,
        },
        WireAuthority::Unavailable { reason } => {
            yoctui_model::RootfsAuthority::Unavailable { reason }
        }
    };
    Ok(BackendEvent::RootfsComposition {
        request,
        composition: yoctui_model::RootfsComposition {
            image,
            installed_packages,
            filesystem_tree,
            system_inventory: yoctui_model::RootfsAuthority::Unavailable {
                reason: "offline system inventory is client-local and was not included in this wire snapshot".into(),
            },
            root_directory: None,
        },
        limitations: data.limitations,
    })
}
