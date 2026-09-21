#[derive(Debug, Clone)]
pub struct PackageDataAdapter {
    build_dir: PathBuf,
    tool: Option<PathBuf>,
    pkgdata_dir: Option<PathBuf>,
    timeout: Duration,
    argument_batch: usize,
    compatibility: Option<DaemonCompatibilitySnapshot>,
    expected_generation: Option<u64>,
}

impl PackageDataAdapter {
    pub fn new(build_dir: PathBuf) -> Self {
        Self {
            build_dir,
            tool: None,
            pkgdata_dir: None,
            timeout: PACKAGE_COMMAND_TIMEOUT,
            argument_batch: PACKAGE_ARGUMENT_BATCH,
            compatibility: None,
            expected_generation: None,
        }
    }

    pub fn with_paths(build_dir: PathBuf, tool: PathBuf, pkgdata_dir: PathBuf) -> Self {
        Self {
            build_dir,
            tool: Some(tool),
            pkgdata_dir: Some(pkgdata_dir),
            timeout: PACKAGE_COMMAND_TIMEOUT,
            argument_batch: PACKAGE_ARGUMENT_BATCH,
            compatibility: None,
            expected_generation: None,
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_pkgdata_dir(mut self, pkgdata_dir: PathBuf) -> Self {
        self.pkgdata_dir = Some(pkgdata_dir);
        self
    }

    pub fn with_compatibility(
        mut self,
        compatibility: DaemonCompatibilitySnapshot,
        expected_generation: u64,
    ) -> Result<Self, PackageDataAdapterError> {
        let compatibility = compatibility
            .normalize()
            .map_err(|error| PackageDataAdapterError::InvalidRequest(error.to_string()))?;
        if compatibility.snapshot.generation != expected_generation {
            return Err(PackageDataAdapterError::StaleCapability {
                expected: expected_generation,
                actual: compatibility.snapshot.generation,
            });
        }
        self.compatibility = Some(compatibility);
        self.expected_generation = Some(expected_generation);
        Ok(self)
    }

    #[cfg(test)]
    fn with_argument_batch(mut self, argument_batch: usize) -> Self {
        self.argument_batch = argument_batch.max(1);
        self
    }

    pub async fn inventory(
        &self,
        request: PackageInventoryRequest,
    ) -> Result<PackageInventoryResponse, PackageDataAdapterError> {
        self.inventory_with_cancellation(request, PackageDataCancellation::default())
            .await
    }

    pub async fn inventory_with_cancellation(
        &self,
        request: PackageInventoryRequest,
        cancellation: PackageDataCancellation,
    ) -> Result<PackageInventoryResponse, PackageDataAdapterError> {
        validate_inventory_request(request)?;
        let context = self.context().await?;
        let (identities, mut limitations) = self.list_packages(&context, &cancellation).await?;
        let mut summaries = identities
            .iter()
            .cloned()
            .map(unavailable_summary)
            .collect::<BTreeMap<_, _>>();

        for chunk in identities.chunks(self.argument_batch) {
            if cancellation.is_cancelled() {
                return Err(PackageDataAdapterError::Cancelled);
            }
            let arguments = [
                vec![OsString::from("-e"), OsString::from("LICENSE")],
                chunk
                    .iter()
                    .map(|identity| OsString::from(&identity.name))
                    .collect(),
            ]
            .concat();
            let spec = context.command(
                CapabilityId::PkgDataPackageInfo,
                PKGDATA_PACKAGE_INFO_IMPLEMENTATION,
                "package-info",
                arguments,
            )?;
            let output =
                run_package_command(spec, &context.build_dir, self.timeout, &cancellation).await?;
            if output.truncated {
                push_limitation(
                    &mut limitations,
                    format!("package-info output was limited to {MAX_PACKAGE_OUTPUT_BYTES} bytes"),
                );
            }
            parse_package_info(&output.stdout, &mut summaries, &mut limitations)?;
        }

        for identity in &identities {
            if summaries
                .get(identity)
                .is_some_and(summary_has_no_information)
            {
                push_limitation(
                    &mut limitations,
                    format!("metadata was unavailable for package {}", identity.name),
                );
            }
        }
        if !identities.is_empty() {
            push_limitation(
                &mut limitations,
                "provider recipe paths are unavailable from oe-pkgdata-util package records".into(),
            );
            push_limitation(
                &mut limitations,
                "image membership is unavailable until an authoritative image manifest is selected"
                    .into(),
            );
        }
        let (packages, report) =
            normalize_package_summaries(summaries.into_values().collect(), MAX_PACKAGE_RECORDS);
        append_normalization_limitations(&mut limitations, &report);
        Ok(PackageInventoryResponse {
            request,
            packages,
            limitations,
        })
    }

    pub async fn detail(
        &self,
        request: PackageDetailRequest,
    ) -> Result<PackageDetailResponse, PackageDataAdapterError> {
        self.detail_with_cancellation(request, PackageDataCancellation::default())
            .await
    }

    pub async fn detail_with_cancellation(
        &self,
        request: PackageDetailRequest,
        cancellation: PackageDataCancellation,
    ) -> Result<PackageDetailResponse, PackageDataAdapterError> {
        validate_detail_request(&request)?;
        let context = self.context().await?;
        let mut limitations = Vec::new();
        let files_spec = context.command(
            CapabilityId::PkgDataListPackageFiles,
            PKGDATA_LIST_PACKAGE_FILES_IMPLEMENTATION,
            "list-pkg-files",
            [OsString::from("-r"), OsString::from(&request.identity.name)],
        )?;
        let files_output =
            run_package_command(files_spec, &context.build_dir, self.timeout, &cancellation)
                .await?;
        if files_output.truncated {
            push_limitation(
                &mut limitations,
                format!("package file output was limited to {MAX_PACKAGE_OUTPUT_BYTES} bytes"),
            );
        }
        let files = parse_package_files(&request.identity, &files_output.stdout, &mut limitations)?;

        let (inventory, inventory_limitations) =
            self.list_packages(&context, &cancellation).await?;
        for limitation in inventory_limitations {
            push_limitation(&mut limitations, limitation);
        }
        if !inventory.contains(&request.identity) {
            return Err(PackageDataAdapterError::InvalidRequest(format!(
                "package {} is not present in the authoritative runtime inventory",
                request.identity.name
            )));
        }
        let mut dependencies = BTreeMap::new();
        for chunk in inventory.chunks(self.argument_batch) {
            let arguments = [
                vec![OsString::from("RDEPENDS"), OsString::from("-n")],
                chunk
                    .iter()
                    .map(|identity| OsString::from(&identity.name))
                    .collect(),
            ]
            .concat();
            let spec = context.command(
                CapabilityId::PkgDataReadValue,
                PKGDATA_READ_VALUE_IMPLEMENTATION,
                "read-value",
                arguments,
            )?;
            let output =
                run_package_command(spec, &context.build_dir, self.timeout, &cancellation).await?;
            if output.truncated {
                push_limitation(
                    &mut limitations,
                    format!(
                        "runtime dependency output was limited to {MAX_PACKAGE_OUTPUT_BYTES} bytes"
                    ),
                );
            }
            parse_runtime_dependencies(&output.stdout, &mut dependencies, &mut limitations)?;
        }
        let runtime_dependencies = dependencies
            .get(&request.identity)
            .cloned()
            .unwrap_or_default();
        if !dependencies.contains_key(&request.identity) {
            push_limitation(
                &mut limitations,
                format!(
                    "runtime dependency data was unavailable for {}",
                    request.identity.name
                ),
            );
        }
        let reverse_dependencies = dependencies
            .iter()
            .filter_map(|(identity, values)| {
                values
                    .contains(&request.identity)
                    .then_some(identity.clone())
            })
            .collect();
        let detail = PackageDetail {
            identity: request.identity.clone(),
            files: PackageField::Available(files),
            runtime_dependencies: if dependencies.contains_key(&request.identity) {
                PackageField::Available(runtime_dependencies)
            } else {
                PackageField::Unavailable
            },
            reverse_dependencies: PackageField::Available(reverse_dependencies),
        };
        let (detail, report) = normalize_package_detail(&request.identity, detail);
        append_normalization_limitations(&mut limitations, &report);
        let detail = detail.ok_or_else(|| {
            PackageDataAdapterError::Malformed(
                "normalized package detail did not match the request".into(),
            )
        })?;
        Ok(PackageDetailResponse {
            request,
            detail,
            limitations,
        })
    }

    async fn context(&self) -> Result<PackageDataContext, PackageDataAdapterError> {
        let build_dir = canonical_directory(&self.build_dir)
            .await
            .map_err(|_| PackageDataAdapterError::BuildDirectory(self.build_dir.clone()))?;
        let compatibility = self.compatibility.clone().ok_or_else(|| {
            PackageDataAdapterError::CapabilityUnavailable {
                capability: CapabilityId::PkgDataGenerated,
                reason: "the current environment capability snapshot is unavailable".into(),
            }
        })?;
        if self.expected_generation != Some(compatibility.snapshot.generation) {
            return Err(PackageDataAdapterError::StaleCapability {
                expected: self.expected_generation.unwrap_or_default(),
                actual: compatibility.snapshot.generation,
            });
        }
        if compatibility
            .snapshot
            .environment
            .build_directory
            .value()
            .map(PathBuf::as_path)
            != Some(build_dir.as_path())
        {
            return Err(PackageDataAdapterError::CapabilityEnvironmentMismatch);
        }
        let generated = compatibility
            .snapshot
            .capability(CapabilityId::PkgDataGenerated);
        if !generated.is_some_and(|record| record.state.is_enabled()) {
            return Err(PackageDataAdapterError::MissingPkgdata(
                self.pkgdata_dir
                    .clone()
                    .unwrap_or_else(|| build_dir.join("tmp/pkgdata")),
            ));
        }
        let pkgdata_path = self
            .pkgdata_dir
            .clone()
            .unwrap_or_else(|| build_dir.join("tmp/pkgdata"));
        let pkgdata_dir = canonical_directory(&pkgdata_path)
            .await
            .map_err(|_| PackageDataAdapterError::MissingPkgdata(pkgdata_path.clone()))?;
        if self.pkgdata_dir.is_none() && !pkgdata_dir.starts_with(&build_dir) {
            return Err(PackageDataAdapterError::PathEscape(pkgdata_dir));
        }
        let detected_tool = compatibility
            .snapshot
            .environment
            .available_tools
            .value()
            .and_then(|tools| tools.iter().find(|tool| tool.id == "oe-pkgdata-util"))
            .map(|tool| tool.executable.clone())
            .ok_or_else(|| PackageDataAdapterError::MissingTool(build_dir.clone()))?;
        if self
            .tool
            .as_ref()
            .is_some_and(|tool| tool != &detected_tool)
        {
            return Err(PackageDataAdapterError::CapabilityEnvironmentMismatch);
        }
        if !detected_tool.exists() {
            return Err(PackageDataAdapterError::MissingTool(detected_tool));
        }
        let tool = canonical_regular_file(&detected_tool).await?;
        Ok(PackageDataContext {
            build_dir,
            pkgdata_dir,
            tool,
            compatibility,
        })
    }

    async fn list_packages(
        &self,
        context: &PackageDataContext,
        cancellation: &PackageDataCancellation,
    ) -> Result<(Vec<PackageIdentity>, Vec<String>), PackageDataAdapterError> {
        let spec = context.command(
            CapabilityId::PkgDataListPackages,
            PKGDATA_LIST_PACKAGES_IMPLEMENTATION,
            "list-pkgs",
            [OsString::from("-r")],
        )?;
        let output =
            match run_package_command(spec, &context.build_dir, self.timeout, cancellation).await {
                Ok(output) => output,
                Err(PackageDataAdapterError::NonZero { message, .. })
                    if message.contains("No packages found") =>
                {
                    return Ok((Vec::new(), Vec::new()));
                }
                Err(error) => return Err(error),
            };
        let mut limitations = Vec::new();
        if output.truncated {
            push_limitation(
                &mut limitations,
                format!("package inventory output was limited to {MAX_PACKAGE_OUTPUT_BYTES} bytes"),
            );
        }
        let identities = parse_package_list(&output.stdout, &mut limitations)?;
        Ok((identities, limitations))
    }
}
