impl App {
    pub fn filtered_packages(&self) -> Vec<&PackageSummary> {
        let query = self.package_query.to_ascii_lowercase();
        self.package_inventory
            .packages()
            .unwrap_or_default()
            .iter()
            .filter(|package| {
                query.is_empty()
                    || [
                        Some(package.identity.name.as_str()),
                        package.recipe.available().map(String::as_str),
                        package.version.available().map(String::as_str),
                        package.license.available().map(String::as_str),
                        package.provider.available().and_then(|path| path.to_str()),
                    ]
                    .into_iter()
                    .flatten()
                    .any(|value| value.to_ascii_lowercase().contains(&query))
            })
            .collect()
    }
    pub fn selected_package(&self) -> Option<&PackageSummary> {
        let selected = self.package_selection.as_ref()?;
        self.filtered_packages()
            .into_iter()
            .find(|package| &package.identity == selected)
    }
    pub fn selected_package_detail(&self) -> Option<&PackageDetailState> {
        self.package_selection
            .as_ref()
            .and_then(|identity| self.package_details.get(identity))
    }
    pub fn selected_package_dependencies(&self) -> Option<&[PackageIdentity]> {
        let detail = self.selected_package_detail()?.detail()?;
        if self.package_dependency_reverse {
            detail.reverse_dependencies.available().map(Vec::as_slice)
        } else {
            detail.runtime_dependencies.available().map(Vec::as_slice)
        }
    }
    pub fn selected_package_dependency(&self) -> Option<&PackageIdentity> {
        self.selected_package_dependencies()?
            .get(self.package_dependency_selection)
    }
    pub fn filtered_image_artifacts(&self) -> Vec<&ImageArtifact> {
        self.image_artifacts
            .artifacts()
            .unwrap_or_default()
            .iter()
            .filter(|artifact| artifact.matches_query(&self.image_artifact_query))
            .collect()
    }
    pub fn selected_image_artifact(&self) -> Option<&ImageArtifact> {
        let selected = self.image_artifact_selection.as_ref()?;
        self.filtered_image_artifacts()
            .into_iter()
            .find(|artifact| &artifact.identity == selected)
    }
    pub fn filtered_sdk_artifacts(&self) -> Vec<&SdkArtifact> {
        self.sdk_artifacts
            .artifacts()
            .unwrap_or_default()
            .iter()
            .filter(|artifact| artifact.matches_query(&self.sdk_artifact_query))
            .collect()
    }
    pub fn selected_sdk_artifact(&self) -> Option<&SdkArtifact> {
        let selected = self.sdk_artifact_selection.as_ref()?;
        self.filtered_sdk_artifacts()
            .into_iter()
            .find(|artifact| &artifact.identity == selected)
    }
    pub fn sdk_session(&self, id: SdkSessionId) -> Option<&SdkSession> {
        self.sdk_sessions.iter().find(|session| session.id == id)
    }
    pub fn active_sdk_session(&self) -> Option<&SdkSession> {
        self.sdk_sessions.iter().rev().find(|session| {
            self.background_jobs
                .get(session.background_job_id)
                .is_some_and(|job| !job.status.is_terminal())
        })
    }
    pub fn latest_sdk_session(&self) -> Option<&SdkSession> {
        self.sdk_sessions.back()
    }
    pub fn test_session(&self, id: TestSessionId) -> Option<&TestSession> {
        self.test_sessions.iter().find(|session| session.id == id)
    }
    pub fn active_test_session(&self) -> Option<&TestSession> {
        self.test_sessions.iter().rev().find(|session| {
            session.background_job_id.is_none()
                || session.background_job_id.is_some_and(|job_id| {
                    self.background_jobs
                        .get(job_id)
                        .is_some_and(|job| !job.status.is_terminal())
                })
        })
    }
    pub fn latest_test_session(&self) -> Option<&TestSession> {
        self.test_sessions.back()
    }
    pub fn filtered_test_results(&self) -> Vec<&TestResultRecord> {
        let query = self.test_result_query.to_ascii_lowercase();
        self.test_results
            .records()
            .iter()
            .filter(|record| {
                query.is_empty()
                    || [
                        record.identity.path.to_str(),
                        Some(record.identity.fingerprint.as_str()),
                        record.machine.as_deref(),
                        record.image.as_deref(),
                        record.revision.as_deref(),
                    ]
                    .into_iter()
                    .flatten()
                    .chain(
                        record
                            .metadata
                            .iter()
                            .flat_map(|entry| [entry.key.as_str(), entry.value.as_str()]),
                    )
                    .any(|value| value.to_ascii_lowercase().contains(&query))
            })
            .collect()
    }
    pub fn selected_test_result(&self) -> Option<&TestResultRecord> {
        let selected = self.test_result_selection.as_ref()?;
        self.filtered_test_results()
            .into_iter()
            .find(|record| &record.identity == selected)
    }
    pub fn selected_test_case(&self) -> Option<&TestCaseRecord> {
        let identity = self.test_case_selection.as_ref()?;
        self.selected_test_result()?.case(identity)
    }
    pub fn test_comparison_transitions(&self) -> &[TestCaseTransition] {
        match &self.test_comparison {
            TestComparisonState::Available { comparison, .. }
            | TestComparisonState::Partial { comparison, .. } => &comparison.transitions,
            _ => &[],
        }
    }
    pub fn selected_test_transition(&self) -> Option<&TestCaseTransition> {
        let selected = self.test_comparison_selection.as_ref()?;
        self.test_comparison_transitions()
            .iter()
            .find(|transition| &transition.identity == selected)
    }
    pub fn qemu_session(&self, id: QemuSessionId) -> Option<&QemuSession> {
        self.qemu_sessions.iter().find(|session| session.id == id)
    }
    pub fn active_qemu_session(&self) -> Option<&QemuSession> {
        self.qemu_sessions.iter().rev().find(|session| {
            self.background_jobs
                .get(session.background_job_id)
                .is_some_and(|job| !job.status.is_terminal())
        })
    }
    pub fn latest_qemu_session(&self) -> Option<&QemuSession> {
        self.qemu_sessions.back()
    }
    pub fn wic_session(&self, id: WicSessionId) -> Option<&WicSession> {
        self.wic_sessions.iter().find(|session| session.id == id)
    }
    pub fn active_wic_session(&self) -> Option<&WicSession> {
        self.wic_sessions.iter().rev().find(|session| {
            self.background_jobs
                .get(session.background_job_id)
                .is_some_and(|job| !job.status.is_terminal())
        })
    }
    pub fn latest_wic_session(&self) -> Option<&WicSession> {
        self.wic_sessions.back()
    }
    pub fn wic_output_rows(&self) -> &[WicOutput] {
        match &self.wic_outputs {
            WicOutputInventoryState::Available { outputs, .. }
            | WicOutputInventoryState::Partial { outputs, .. } => outputs,
            _ => &[],
        }
    }
    pub fn selected_wic_output(&self) -> Option<&WicOutput> {
        let selected = self.wic_output_selection.as_ref()?;
        self.wic_output_rows()
            .iter()
            .find(|output| &output.identity == selected)
    }
    pub fn wic_device_rows(&self) -> &[WicDevice] {
        match &self.wic_devices {
            WicDeviceInventoryState::Available { devices, .. }
            | WicDeviceInventoryState::Partial { devices, .. } => devices,
            _ => &[],
        }
    }
    pub fn selected_wic_device(&self) -> Option<&WicDevice> {
        let selected = self.wic_device_selection.as_ref()?;
        self.wic_device_rows()
            .iter()
            .find(|device| &device.identity == selected)
    }
    pub fn selected_wic_write_image(&self) -> Result<WicOutputIdentity, String> {
        if self.wic_output_selection.is_some() {
            let output = self
                .selected_wic_output()
                .ok_or_else(|| "The selected generated Wic output is stale.".to_owned())?;
            if !matches!(output.kind, WicOutputKind::Wic | WicOutputKind::Direct)
                || !is_uncompressed_wic_path(&output.identity.path)
            {
                return Err("Select an uncompressed generated .wic or .direct image first.".into());
            }
            return Ok(output.identity.clone());
        }
        let artifact = self
            .selected_image_artifact()
            .ok_or_else(|| "Select a deployed Wic or generated Wic output first.".to_owned())?;
        if artifact.kind != ImageArtifactKind::Wic
            || !is_uncompressed_wic_path(&artifact.identity.path)
        {
            return Err("Select an uncompressed deployed .wic or .direct image first.".into());
        }
        let size_bytes = artifact
            .size_bytes
            .available()
            .copied()
            .ok_or_else(|| "The selected Wic image size is unavailable.".to_owned())?;
        let modified_unix_seconds = artifact
            .modified_unix_seconds
            .available()
            .copied()
            .ok_or_else(|| "The selected Wic image timestamp is unavailable.".to_owned())?;
        Ok(WicOutputIdentity {
            path: artifact.identity.path.clone(),
            size_bytes,
            modified_unix_seconds,
        })
    }
    pub fn wic_device_write_unavailable_reason(&self) -> Option<String> {
        if self.active_wic_session().is_some() {
            return Some("A managed Wic operation is already active.".into());
        }
        match &self.wic_capability {
            WicCapability::Available { executable, .. }
            | WicCapability::MissingKickstarts { executable }
                if executable.is_absolute() => {}
            WicCapability::NotInspected => {
                return Some("Wic capability has not been inspected.".into());
            }
            WicCapability::MissingTool => return Some("wic is not available.".into()),
            WicCapability::Failed { message } => {
                return Some(format!("Wic capability inspection failed: {message}"));
            }
            WicCapability::Available { .. } | WicCapability::MissingKickstarts { .. } => {
                return Some("The inspected Wic executable identity is invalid.".into());
            }
        }
        self.selected_wic_write_image().err()
    }
    pub fn wic_create_unavailable_reason(&self) -> Option<String> {
        if self.active_wic_session().is_some() {
            return Some("A managed Wic operation is already active.".into());
        }
        let Some(artifact) = self.selected_image_artifact() else {
            return Some("Select a deployed image artifact first.".into());
        };
        let WicCapability::Available {
            kickstarts,
            image_targets,
            ..
        } = &self.wic_capability
        else {
            return Some(match &self.wic_capability {
                WicCapability::NotInspected => "Wic capability has not been inspected.".into(),
                WicCapability::MissingTool => "wic is not available.".into(),
                WicCapability::MissingKickstarts { .. } => {
                    "No Wic kickstarts are available.".into()
                }
                WicCapability::Failed { message } => {
                    format!("Wic capability inspection failed: {message}")
                }
                WicCapability::Available { .. } => unreachable!(),
            });
        };
        if kickstarts.is_empty()
            || !image_targets
                .iter()
                .any(|target| target == &artifact.identity.image)
        {
            return Some("The selected image is not in the inspected Wic capability.".into());
        }
        None
    }
    pub fn qemu_launch_unavailable_reason(&self) -> Option<String> {
        if self.active_qemu_session().is_some() {
            return Some("A managed runqemu session is already active.".into());
        }
        let Some(artifact) = self.selected_image_artifact() else {
            return Some("Select a deployed image artifact first.".into());
        };
        if !matches!(
            artifact.kind,
            ImageArtifactKind::RootFilesystem | ImageArtifactKind::Wic
        ) {
            return Some("runqemu requires a root filesystem or Wic artifact.".into());
        }
        match &self.qemu_capability {
            QemuCapability::NotInspected => {
                Some("runqemu capability has not been inspected.".into())
            }
            QemuCapability::MissingTool => Some("runqemu is not available.".into()),
            QemuCapability::MissingCompatibleImage => {
                Some("No compatible deployed runqemu image is available.".into())
            }
            QemuCapability::Failed { message } => {
                Some(format!("runqemu capability inspection failed: {message}"))
            }
            QemuCapability::Available {
                executable: _,
                compatible_images,
            } if !compatible_images.contains(&artifact.identity) => {
                Some("The selected artifact is not in the inspected runqemu capability.".into())
            }
            QemuCapability::Available { .. } => None,
        }
    }
}
