#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RootfsNormalizationReport {
    pub duplicate_packages: usize,
    pub invalid_packages: usize,
    pub truncated_packages: usize,
    pub duplicate_entries: usize,
    pub invalid_entries: usize,
    pub orphan_entries: usize,
    pub truncated_entries: usize,
    pub truncated_depth: usize,
    pub invalid_limitations: usize,
    pub truncated_limitations: usize,
    pub arithmetic_overflow: usize,
}

impl RootfsNormalizationReport {
    pub fn is_partial(&self) -> bool {
        self != &Self::default()
    }
}

pub fn normalize_rootfs_composition(
    request: &RootfsCompositionRequest,
    mut composition: RootfsComposition,
) -> (Option<RootfsComposition>, RootfsNormalizationReport) {
    let mut report = RootfsNormalizationReport::default();
    if request.validate().is_err()
        || composition.image.validate().is_err()
        || composition.image != request.image
    {
        report.invalid_entries = 1;
        return (None, report);
    }
    normalize_package_authority(&mut composition.installed_packages, &mut report);
    normalize_filesystem_authority(&mut composition.filesystem_tree, &mut report);
    normalize_system_authority(&mut composition.system_inventory, &mut report);
    (Some(composition), report)
}

fn normalize_system_authority(
    authority: &mut RootfsAuthority<RootfsSystemInventory>,
    report: &mut RootfsNormalizationReport,
) {
    let normalize = |inventory: &mut RootfsSystemInventory,
                     report: &mut RootfsNormalizationReport| {
        inventory.udev_rules.retain(|rule| {
            let valid = rule.name.ends_with(".rules")
                && rootfs_text_is_valid(&rule.name)
                && rule.logical_path.validate().is_ok()
                && rule
                    .overridden_by
                    .as_ref()
                    .is_none_or(|path| path.validate().is_ok())
                && rule.preview.len() <= MAX_ROOTFS_SYSTEM_PREVIEW_BYTES
                && rule.limitation.as_deref().is_none_or(rootfs_text_is_valid);
            if !valid {
                report.invalid_entries += 1;
            }
            valid
        });
        inventory.udev_rules.sort();
        inventory
            .udev_rules
            .dedup_by(|left, right| left.logical_path == right.logical_path);
        if inventory.udev_rules.len() > MAX_ROOTFS_UDEV_RULES {
            report.invalid_entries += inventory.udev_rules.len() - MAX_ROOTFS_UDEV_RULES;
            inventory.udev_rules.truncate(MAX_ROOTFS_UDEV_RULES);
        }
        inventory.systemd_services.retain(|service| {
            let valid = !service.name.is_empty()
                && rootfs_text_is_valid(&service.name)
                && service.logical_path.validate().is_ok()
                && service.host_path.is_absolute()
                && service.preview.len() <= MAX_ROOTFS_SYSTEM_PREVIEW_BYTES;
            if !valid {
                report.invalid_entries += 1;
            }
            valid
        });
        inventory.systemd_services.sort();
        inventory
            .systemd_services
            .dedup_by(|left, right| left.name == right.name);
        inventory.dbus_services.retain(|service| {
            let valid = !service.name.is_empty()
                && rootfs_text_is_valid(&service.name)
                && service.logical_path.validate().is_ok()
                && service.host_path.is_absolute()
                && service.preview.len() <= MAX_ROOTFS_SYSTEM_PREVIEW_BYTES;
            if !valid {
                report.invalid_entries += 1;
            }
            valid
        });
        inventory.dbus_services.sort();
        inventory
            .dbus_services
            .dedup_by(|left, right| left.name == right.name);
    };
    match authority {
        RootfsAuthority::Available(value) => normalize(value, report),
        RootfsAuthority::Partial { value, limitations } => {
            normalize(value, report);
            normalize_limitations(limitations, report);
        }
        RootfsAuthority::Unavailable { reason } => normalize_reason(reason, report),
    }
}

fn normalize_package_authority(
    authority: &mut RootfsAuthority<RootfsPackageInventory>,
    report: &mut RootfsNormalizationReport,
) {
    match authority {
        RootfsAuthority::Available(inventory) => normalize_packages(inventory, report),
        RootfsAuthority::Partial { value, limitations } => {
            normalize_packages(value, report);
            normalize_limitations(limitations, report);
        }
        RootfsAuthority::Unavailable { reason } => normalize_reason(reason, report),
    }
}

fn normalize_packages(
    inventory: &mut RootfsPackageInventory,
    report: &mut RootfsNormalizationReport,
) {
    inventory.packages.sort();
    let mut normalized = BTreeMap::<PackageIdentity, RootfsInstalledPackage>::new();
    for package in inventory.packages.drain(..) {
        if package.identity.validate().is_err()
            || package.category.is_empty()
            || !rootfs_text_is_valid(&package.category)
            || package
                .recipe
                .as_deref()
                .is_some_and(|value| !rootfs_text_is_valid(value))
        {
            report.invalid_packages += 1;
            continue;
        }
        if let Some(existing) = normalized.get(&package.identity) {
            report.duplicate_packages += 1;
            if &package < existing {
                normalized.insert(package.identity.clone(), package);
            }
        } else {
            if normalized.len() == MAX_ROOTFS_PACKAGES {
                report.truncated_packages += 1;
            } else {
                normalized.insert(package.identity.clone(), package);
            }
        }
    }
    inventory.packages = normalized.into_values().collect();
}

fn normalize_filesystem_authority(
    authority: &mut RootfsAuthority<RootfsFilesystemTree>,
    report: &mut RootfsNormalizationReport,
) {
    match authority {
        RootfsAuthority::Available(tree) => normalize_entries(tree, report),
        RootfsAuthority::Partial { value, limitations } => {
            normalize_entries(value, report);
            normalize_limitations(limitations, report);
        }
        RootfsAuthority::Unavailable { reason } => normalize_reason(reason, report),
    }
}

fn normalize_entries(tree: &mut RootfsFilesystemTree, report: &mut RootfsNormalizationReport) {
    tree.entries.sort();
    let mut normalized = BTreeMap::<RootfsPathIdentity, RootfsEntry>::new();
    for entry in tree.entries.drain(..) {
        if entry.identity.validate().is_err()
            || entry
                .package
                .as_ref()
                .is_some_and(|package| package.validate().is_err())
        {
            report.invalid_entries += 1;
            continue;
        }
        if entry.identity.depth() > MAX_ROOTFS_DEPTH {
            report.truncated_depth += 1;
            continue;
        }
        if let Some(existing) = normalized.get(&entry.identity) {
            report.duplicate_entries += 1;
            if &entry < existing {
                normalized.insert(entry.identity.clone(), entry);
            }
        } else {
            if normalized.len() == MAX_ROOTFS_ENTRIES {
                report.truncated_entries += 1;
            } else {
                normalized.insert(entry.identity.clone(), entry);
            }
        }
    }
    let identities = normalized.keys().cloned().collect::<BTreeSet<_>>();
    report.orphan_entries += normalized
        .keys()
        .filter(|identity| {
            identity.0 != Path::new("/")
                && identity
                    .0
                    .parent()
                    .map(|parent| !identities.contains(&RootfsPathIdentity(parent.into())))
                    .unwrap_or(true)
        })
        .count();
    tree.entries = normalized.into_values().collect();
}

fn normalize_limitations(values: &mut Vec<String>, report: &mut RootfsNormalizationReport) {
    let before = values.len();
    values.retain(|value| !value.is_empty() && rootfs_text_is_valid(value));
    report.invalid_limitations += before - values.len();
    values.sort();
    values.dedup();
    if values.len() > MAX_ROOTFS_LIMITATIONS {
        report.truncated_limitations += values.len() - MAX_ROOTFS_LIMITATIONS;
        values.truncate(MAX_ROOTFS_LIMITATIONS);
    }
}

fn normalize_reason(reason: &mut String, report: &mut RootfsNormalizationReport) {
    if reason.is_empty() || !rootfs_text_is_valid(reason) {
        *reason = "authority unavailable without a valid reason".into();
        report.invalid_limitations += 1;
    }
}
