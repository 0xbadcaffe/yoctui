#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RootfsTotals {
    pub installed_package_bytes: u64,
    pub package_reported_files: u64,
    pub filesystem_bytes: u64,
    pub packages: u64,
    pub entries: u64,
    pub files: u64,
    pub directories: u64,
    pub symlinks: u64,
    pub other: u64,
}

impl RootfsComposition {
    pub fn totals(&self) -> (RootfsTotals, bool) {
        let mut totals = RootfsTotals::default();
        let mut overflowed = false;
        if let Some(inventory) = self.package_inventory() {
            totals.packages = inventory.packages.len() as u64;
            for package in &inventory.packages {
                totals.installed_package_bytes = checked_sum(
                    totals.installed_package_bytes,
                    package.installed_size_bytes,
                    &mut overflowed,
                );
                totals.package_reported_files = checked_sum(
                    totals.package_reported_files,
                    package.file_count,
                    &mut overflowed,
                );
            }
        }
        if let Some(tree) = self.filesystem_tree() {
            totals.entries = tree.entries.len() as u64;
            for entry in &tree.entries {
                totals.filesystem_bytes =
                    checked_sum(totals.filesystem_bytes, entry.size_bytes, &mut overflowed);
                match entry.kind {
                    RootfsEntryKind::RegularFile => totals.files += 1,
                    RootfsEntryKind::Directory => totals.directories += 1,
                    RootfsEntryKind::Symlink => totals.symlinks += 1,
                    RootfsEntryKind::Other => totals.other += 1,
                }
            }
        }
        (totals, overflowed)
    }
}

fn checked_sum(current: u64, value: u64, overflowed: &mut bool) -> u64 {
    current.checked_add(value).unwrap_or_else(|| {
        *overflowed = true;
        u64::MAX
    })
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RootfsGroupIdentity {
    Category(String),
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootfsGroupRow {
    pub identity: RootfsGroupIdentity,
    pub installed_size_bytes: u64,
    pub package_count: u64,
    pub percent_basis_points: u16,
    pub members: Vec<PackageIdentity>,
}

impl RootfsPackageInventory {
    pub fn grouped(&self, max_groups: usize) -> Vec<RootfsGroupRow> {
        let mut grouped = BTreeMap::<String, (u64, Vec<PackageIdentity>)>::new();
        let mut total = 0_u64;
        for package in &self.packages {
            total = total.saturating_add(package.installed_size_bytes);
            let group = grouped.entry(package.category.clone()).or_default();
            group.0 = group.0.saturating_add(package.installed_size_bytes);
            group.1.push(package.identity.clone());
        }
        let mut rows = grouped
            .into_iter()
            .map(|(category, (bytes, mut members))| {
                members.sort();
                RootfsGroupRow {
                    identity: RootfsGroupIdentity::Category(category),
                    installed_size_bytes: bytes,
                    package_count: members.len() as u64,
                    percent_basis_points: percentage_basis_points(bytes, total),
                    members,
                }
            })
            .collect::<Vec<_>>();
        rows.sort_by_key(|row| (Reverse(row.installed_size_bytes), row.identity.clone()));
        let limit = max_groups.max(1);
        if rows.len() > limit {
            let retained = limit.saturating_sub(1);
            let remainder = rows.split_off(retained);
            let mut members = remainder
                .iter()
                .flat_map(|row| row.members.iter().cloned())
                .collect::<Vec<_>>();
            members.sort();
            let bytes = remainder.iter().fold(0_u64, |sum, row| {
                sum.saturating_add(row.installed_size_bytes)
            });
            rows.push(RootfsGroupRow {
                identity: RootfsGroupIdentity::Other,
                installed_size_bytes: bytes,
                package_count: members.len() as u64,
                percent_basis_points: percentage_basis_points(bytes, total),
                members,
            });
        }
        rows
    }
}

impl RootfsFilesystemTree {
    pub fn children(&self, parent: &RootfsPathIdentity) -> Vec<&RootfsEntry> {
        self.entries
            .iter()
            .filter(|entry| {
                entry.identity.0 != parent.0
                    && entry.identity.0.parent() == Some(parent.0.as_path())
            })
            .collect()
    }

    pub fn descendants(
        &self,
        parent: &RootfsPathIdentity,
        max_rows: usize,
    ) -> (Vec<&RootfsEntry>, usize) {
        let mut values = self
            .entries
            .iter()
            .filter(|entry| entry.identity.0 != parent.0 && entry.identity.0.starts_with(&parent.0))
            .collect::<Vec<_>>();
        let limit = max_rows.max(1);
        let truncated = values.len().saturating_sub(limit);
        values.truncate(limit);
        (values, truncated)
    }
}

fn percentage_basis_points(value: u64, total: u64) -> u16 {
    if total == 0 {
        0
    } else {
        ((u128::from(value) * 10_000) / u128::from(total)) as u16
    }
}

