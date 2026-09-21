fn scan_manifest(
    build: &Path,
    manifest_path: &Path,
    pkgdata_directory: Option<&Path>,
    cancellation: &RootfsCompositionCancellation,
    deadline: Instant,
    limitations: &mut Vec<String>,
) -> Result<RootfsAuthority<RootfsPackageInventory>, RootfsCompositionAdapterError> {
    check_control(cancellation, deadline)?;
    let manifest = canonical_regular_file(manifest_path, build)?;
    let metadata = fs::metadata(&manifest)
        .map_err(|error| RootfsCompositionAdapterError::Io(error.to_string()))?;
    if metadata.len() > MAX_MANIFEST_BYTES {
        return Ok(RootfsAuthority::Unavailable {
            reason: format!(
                "the selected image manifest exceeds the {MAX_MANIFEST_BYTES}-byte safety bound"
            ),
        });
    }
    let text = fs::read_to_string(&manifest)
        .map_err(|error| RootfsCompositionAdapterError::Io(error.to_string()))?;
    let mut package_names = BTreeSet::new();
    let mut package_limitations = Vec::new();
    for line in text.lines() {
        check_control(cancellation, deadline)?;
        let Some(name) = line.split_whitespace().next() else {
            continue;
        };
        let identity = PackageIdentity::new(name);
        if identity.validate().is_err() {
            push_limitation(
                &mut package_limitations,
                format!("manifest contained an invalid package identity: {name}"),
            );
            continue;
        }
        if package_names.len() == MAX_ROOTFS_PACKAGES && !package_names.contains(name) {
            push_limitation(
                &mut package_limitations,
                format!("image manifest was limited to {MAX_ROOTFS_PACKAGES} packages"),
            );
            break;
        }
        package_names.insert(name.to_owned());
    }

    let pkgdata = match pkgdata_directory {
        Some(path) if source_is_missing(path)? => {
            push_limitation(
                &mut package_limitations,
                "PKGDATA_DIR has been cleaned; package metadata is unavailable".into(),
            );
            None
        }
        Some(path) => Some(canonical_directory(path, Some(build))?),
        None => {
            push_limitation(
                &mut package_limitations,
                "PKGDATA_DIR was not reported; package size, recipe, category, and file counts are unavailable".into(),
            );
            None
        }
    };
    let mut pkgdata_bytes = 0_u64;
    let mut packages = Vec::with_capacity(package_names.len());
    for name in package_names {
        check_control(cancellation, deadline)?;
        let mut package = RootfsInstalledPackage {
            identity: PackageIdentity::new(&name),
            recipe: None,
            category: "uncategorized".into(),
            installed_size_bytes: 0,
            file_count: 0,
        };
        if let Some(pkgdata) = &pkgdata {
            match read_installed_pkgdata(pkgdata, &name, &mut pkgdata_bytes) {
                Ok(values) => populate_package(&mut package, &values, &mut package_limitations),
                Err(RootfsCompositionAdapterError::InvalidSource(_)) => push_limitation(
                    &mut package_limitations,
                    format!("generated pkgdata was unavailable for installed package {name}"),
                ),
                Err(RootfsCompositionAdapterError::ResourceLimit(message)) => push_limitation(
                    &mut package_limitations,
                    format!(
                        "generated pkgdata was limited for installed package {name}: {message}"
                    ),
                ),
                Err(error) => return Err(error),
            }
        }
        packages.push(package);
    }
    limitations.extend(package_limitations.iter().cloned());
    let inventory = RootfsPackageInventory { packages };
    if package_limitations.is_empty() {
        Ok(RootfsAuthority::Available(inventory))
    } else {
        Ok(RootfsAuthority::Partial {
            value: inventory,
            limitations: package_limitations,
        })
    }
}

fn read_installed_pkgdata(
    root: &Path,
    installed_name: &str,
    total_bytes: &mut u64,
) -> Result<PkgdataValues, RootfsCompositionAdapterError> {
    // Only Yocto's generated, single-hop reverse mapping may be a symlink.
    // Never canonicalize an arbitrary link before checking its target shape.
    let runtime = canonical_directory(&root.join("runtime"), Some(root))?;
    let reverse = root.join("runtime-reverse");
    let mut package_name = installed_name.to_owned();
    if !source_is_missing(&reverse)? {
        let reverse = canonical_directory(&reverse, Some(root))?;
        let mapping = reverse.join(installed_name);
        if !source_is_missing(&mapping)? {
            let invalid = || RootfsCompositionAdapterError::InvalidSource(mapping.clone());
            let target = fs::read_link(&mapping).map_err(|_| invalid())?;
            let parts: Vec<_> = target.components().collect();
            use std::path::Component;
            let [
                Component::ParentDir,
                Component::Normal(directory),
                Component::Normal(name),
            ] = parts.as_slice()
            else {
                return Err(invalid());
            };
            if *directory != std::ffi::OsStr::new("runtime") {
                return Err(invalid());
            }
            package_name = name.to_str().ok_or_else(invalid)?.to_owned();
            PackageIdentity::new(&package_name)
                .validate()
                .map_err(|_| invalid())?;
        }
    }
    let path = runtime.join(&package_name);
    let values = read_pkgdata(&path, &runtime, &package_name, total_bytes)?;
    // PKG is expressed in the original runtime-record namespace. A renamed
    // mapping must corroborate the manifest identity; absence is not a guess.
    if values.conflicting_package_name
        || values
            .package_name
            .as_deref()
            .is_some_and(|name| name != installed_name)
        || (package_name != installed_name && values.package_name.is_none())
    {
        return Err(RootfsCompositionAdapterError::InvalidSource(path));
    }
    Ok(values)
}

fn read_pkgdata(
    path: &Path,
    root: &Path,
    package_name: &str,
    total_bytes: &mut u64,
) -> Result<PkgdataValues, RootfsCompositionAdapterError> {
    let path = canonical_regular_file(path, root)?;
    let length = fs::metadata(&path)
        .map_err(|error| RootfsCompositionAdapterError::Io(error.to_string()))?
        .len();
    if length > MAX_PKGDATA_FILE_BYTES
        || total_bytes.saturating_add(length) > MAX_PKGDATA_TOTAL_BYTES
    {
        return Err(RootfsCompositionAdapterError::ResourceLimit(format!(
            "generated pkgdata exceeded its {MAX_PKGDATA_FILE_BYTES}-byte file or {MAX_PKGDATA_TOTAL_BYTES}-byte total bound"
        )));
    }
    *total_bytes += length;
    let file = fs::File::open(path)
        .map_err(|error| RootfsCompositionAdapterError::Io(error.to_string()))?;
    let mut reader = BufReader::new(file);
    let mut bytes = Vec::new();
    let mut values = PkgdataValues::default();
    loop {
        bytes.clear();
        let read = reader
            .read_until(b'\n', &mut bytes)
            .map_err(|error| RootfsCompositionAdapterError::Io(error.to_string()))?;
        if read == 0 {
            break;
        }
        if bytes.len() > MAX_PKGDATA_LINE_BYTES {
            return Err(RootfsCompositionAdapterError::ResourceLimit(format!(
                "generated pkgdata contained a line over the {MAX_PKGDATA_LINE_BYTES}-byte bound"
            )));
        }
        let line = std::str::from_utf8(&bytes)
            .map_err(|_| {
                RootfsCompositionAdapterError::Io("generated pkgdata is not UTF-8".into())
            })?
            .trim_end_matches(['\r', '\n']);
        let Some((key, raw_value)) = line.split_once(':') else {
            continue;
        };
        match key.trim() {
            "PN" => values.recipe = Some(raw_value.trim().to_owned()),
            "PKG" => {
                let name = scoped_pkgdata_value(raw_value, package_name);
                if values
                    .package_name
                    .as_deref()
                    .is_some_and(|old| old != name)
                {
                    values.conflicting_package_name = true;
                }
                values.package_name = Some(name.to_owned());
            }
            "SECTION" => values.category = Some(raw_value.trim().to_owned()),
            "PKGSIZE" => {
                values.installed_size = scoped_pkgdata_value(raw_value, package_name)
                    .parse::<u64>()
                    .ok();
            }
            "FILES_INFO" => {
                values.file_count =
                    count_json_object_entries(scoped_pkgdata_value(raw_value, package_name));
                values.files_info_seen = true;
            }
            _ => {}
        }
    }
    Ok(values)
}

#[derive(Debug, Default, PartialEq, Eq)]
struct PkgdataValues {
    recipe: Option<String>,
    package_name: Option<String>,
    conflicting_package_name: bool,
    category: Option<String>,
    installed_size: Option<u64>,
    file_count: Option<u64>,
    files_info_seen: bool,
}

fn scoped_pkgdata_value<'a>(raw_value: &'a str, package_name: &str) -> &'a str {
    let value = raw_value.trim();
    value
        .strip_prefix(package_name)
        .and_then(|value| value.strip_prefix(':'))
        .map_or(value, str::trim)
}

fn count_json_object_entries(value: &str) -> Option<u64> {
    struct EntryCountVisitor;

    impl<'de> Visitor<'de> for EntryCountVisitor {
        type Value = u64;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a generated pkgdata FILES_INFO object")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>,
        {
            let mut count = 0_u64;
            while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {
                count = count.saturating_add(1);
            }
            Ok(count)
        }
    }

    let mut deserializer = serde_json::Deserializer::from_str(value);
    let count = deserializer.deserialize_map(EntryCountVisitor).ok()?;
    deserializer.end().ok()?;
    Some(count)
}
