#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WicCreateCommandSpec {
    executable: PathBuf,
    arguments: Vec<OsString>,
}

impl WicCreateCommandSpec {
    pub fn from_preview(
        preview: &WicCreatePreview,
        capability: &WicCapability,
    ) -> Result<Self, WicAdapterError> {
        preview
            .request
            .validate()
            .map_err(|message| WicAdapterError::InvalidRequest(message.into()))?;
        let (inspected_executable, inspected_kickstart) = capability
            .resolve(&preview.request.kickstart, &preview.request.image)
            .map_err(|message| WicAdapterError::InvalidRequest(message.into()))?;
        if inspected_kickstart != &preview.kickstart
            || preview.argv.first().map(PathBuf::as_path) != Some(inspected_executable)
        {
            return Err(WicAdapterError::PreviewMismatch);
        }
        let executable = regular_executable(inspected_executable)?;
        if let Some(path) = &preview.request.kickstart.path {
            regular_canonical(path).map_err(|_| WicAdapterError::UnsafeKickstart(path.clone()))?;
        }
        canonical_directory(&preview.request.output_directory)?;
        let expected = create_arguments(&preview.request);
        if preview
            .argv
            .iter()
            .skip(1)
            .map(|argument| argument.as_os_str())
            .ne(expected.iter().map(OsString::as_os_str))
        {
            return Err(WicAdapterError::PreviewMismatch);
        }
        Ok(Self {
            executable,
            arguments: expected,
        })
    }

    pub fn executable(&self) -> &Path {
        &self.executable
    }

    pub fn arguments(&self) -> &[OsString] {
        &self.arguments
    }
}

fn create_arguments(request: &WicCreateRequest) -> Vec<OsString> {
    let mut arguments = vec![
        "create".into(),
        request.kickstart.argument().into_os_string(),
        "-e".into(),
        request.image.clone().into(),
        "-o".into(),
        request.output_directory.as_os_str().to_owned(),
    ];
    if request.generate_bmap {
        arguments.push("--bmap".into());
    }
    if let Some(compression) = request.compression.argument() {
        arguments.extend(["--compress-with".into(), compression.into()]);
    }
    arguments
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WicDeviceInventoryResponse {
    pub request: WicDeviceInventoryRequest,
    pub devices: Vec<WicDevice>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct WicDeviceInspector {
    lsblk_program: PathBuf,
    validate_device_nodes: bool,
    inspection_timeout: Duration,
    unwritable_device_nodes: BTreeSet<PathBuf>,
}

impl Default for WicDeviceInspector {
    fn default() -> Self {
        Self {
            lsblk_program: "lsblk".into(),
            validate_device_nodes: true,
            inspection_timeout: WIC_DEVICE_INSPECTION_TIMEOUT,
            unwritable_device_nodes: BTreeSet::new(),
        }
    }
}

impl WicDeviceInspector {
    pub fn with_program(program: PathBuf) -> Self {
        Self {
            lsblk_program: program,
            ..Self::default()
        }
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    #[doc(hidden)]
    pub fn without_device_node_validation_for_tests(mut self) -> Self {
        self.validate_device_nodes = false;
        self
    }

    #[cfg(test)]
    fn with_inspection_timeout(mut self, timeout: Duration) -> Self {
        self.inspection_timeout = timeout;
        self
    }

    #[cfg(test)]
    fn with_unwritable_device(mut self, path: PathBuf) -> Self {
        self.unwritable_device_nodes.insert(path);
        self
    }

    pub async fn discover(
        &self,
        request: WicDeviceInventoryRequest,
    ) -> Result<WicDeviceInventoryResponse, WicAdapterError> {
        request
            .validate()
            .map_err(|message| WicAdapterError::InvalidRequest(message.into()))?;
        validate_wic_image(&request.image)?;
        let executable = resolve_executable(&self.lsblk_program)
            .map_err(WicAdapterError::DeviceDiscovery)?
            .ok_or_else(|| WicAdapterError::MissingDeviceTool(self.lsblk_program.clone()))?;
        let bytes = run_lsblk(&executable, self.inspection_timeout).await?;
        let (devices, limitations) = parse_lsblk_devices(
            &bytes,
            &request.image,
            self.validate_device_nodes,
            &self.unwritable_device_nodes,
        )?;
        Ok(WicDeviceInventoryResponse {
            request,
            devices,
            limitations,
        })
    }

    pub async fn command_for(
        &self,
        request: &WicWriteRequest,
    ) -> Result<WicWriteCommandSpec, WicAdapterError> {
        let inventory = self
            .discover(WicDeviceInventoryRequest {
                generation: 1,
                image: request.image.clone(),
            })
            .await?;
        let device = inventory
            .devices
            .iter()
            .find(|device| device.identity == request.device)
            .ok_or(WicAdapterError::StaleDevice)?;
        device
            .eligible_for(&request.image)
            .map_err(|_| WicAdapterError::UnsafeDevice(request.device.path.clone()))?;
        let executable = regular_executable(&request.executable)?;
        if executable != request.executable {
            return Err(WicAdapterError::UnsafeExecutable(
                request.executable.clone(),
            ));
        }
        Ok(WicWriteCommandSpec {
            executable,
            arguments: vec![
                "write".into(),
                request.image.path.as_os_str().to_owned(),
                request.device.path.as_os_str().to_owned(),
            ],
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WicWriteCommandSpec {
    executable: PathBuf,
    arguments: Vec<OsString>,
}

impl WicWriteCommandSpec {
    pub fn executable(&self) -> &Path {
        &self.executable
    }

    pub fn arguments(&self) -> &[OsString] {
        &self.arguments
    }
}

#[derive(Debug, Deserialize)]
struct LsblkDocument {
    blockdevices: Vec<LsblkNode>,
}

#[derive(Debug, Deserialize)]
struct LsblkNode {
    path: serde_json::Value,
    #[serde(rename = "type")]
    kind: serde_json::Value,
    #[serde(rename = "maj:min")]
    major_minor: serde_json::Value,
    size: serde_json::Value,
    model: serde_json::Value,
    serial: serde_json::Value,
    tran: serde_json::Value,
    rm: serde_json::Value,
    ro: serde_json::Value,
    mountpoints: serde_json::Value,
    #[serde(default)]
    children: Vec<LsblkNode>,
}
