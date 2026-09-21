#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SdkOperation {
    Publish(SdkPublishRequest),
    Native(SdkNativeRequest),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SdkSessionId(pub u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdkSession {
    pub id: SdkSessionId,
    pub background_job_id: BackgroundJobId,
    pub operation: SdkOperation,
    pub exit_code: Option<i32>,
    pub error_detail: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SdkOutputStream {
    Stdout,
    Stderr,
}

