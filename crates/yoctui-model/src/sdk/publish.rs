#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SdkPublishDraft {
    pub destination: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdkPublishRequest {
    pub executable: PathBuf,
    pub artifact: SdkArtifactIdentity,
    pub destination: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdkPublishPreview {
    pub request: SdkPublishRequest,
    pub argv: Vec<PathBuf>,
}

impl SdkPublishPreview {
    pub fn new(
        executable: PathBuf,
        artifact: SdkArtifactIdentity,
        destination: PathBuf,
    ) -> Result<Self, &'static str> {
        if !absolute_normal_path(&executable)
            || !absolute_normal_path(&destination)
            || artifact.validate().is_err()
        {
            return Err("SDK publication preview identity is invalid");
        }
        let request = SdkPublishRequest {
            executable: executable.clone(),
            artifact,
            destination,
        };
        let argv = vec![
            executable,
            request.artifact.path.clone(),
            request.destination.clone(),
        ];
        Ok(Self { request, argv })
    }
}
