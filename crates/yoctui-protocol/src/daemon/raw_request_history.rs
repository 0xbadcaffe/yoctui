#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawExecutionRequestData {
    pub schema_version: u16,
    pub request_id: String,
    pub catalog_version: u16,
    pub command_id: String,
    pub parameters: Vec<RawExecutionParameterData>,
    pub additional_arguments: Vec<String>,
    pub interaction: RawInteractionData,
    pub safety: RawSafetyData,
    pub capability_generation: u64,
    pub build_directory: String,
    pub preview_digest: String,
}

impl RawExecutionRequestData {
    pub fn validate(&self) -> Result<(), RawExecutionProtocolError> {
        validate_raw_schema(self.schema_version)?;
        validate_raw_identity(&self.request_id, "raw-request:", "request")?;
        validate_raw_catalog_id(&self.command_id, "command")?;
        if self.catalog_version == 0 || self.capability_generation == 0 {
            return Err(RawExecutionProtocolError::InvalidAuthority);
        }
        if self.parameters.len() > MAX_RAW_EXECUTION_PARAMETERS {
            return Err(RawExecutionProtocolError::TooManyParameters);
        }
        let mut parameter_ids = std::collections::BTreeSet::new();
        for parameter in &self.parameters {
            parameter.validate()?;
            if !parameter_ids.insert(&parameter.id) {
                return Err(RawExecutionProtocolError::DuplicateParameter);
            }
        }
        validate_raw_arguments(&self.additional_arguments)?;
        validate_raw_absolute_path(&self.build_directory)?;
        if self.preview_digest.len() != 64
            || !self
                .preview_digest
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(RawExecutionProtocolError::InvalidPreviewDigest);
        }
        if matches!(self.interaction, RawInteractionData::Unknown)
            || matches!(self.safety, RawSafetyData::Unknown)
        {
            return Err(RawExecutionProtocolError::UnknownRequiredVariant);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawExecutionParameterData {
    pub id: String,
    pub value: RawParameterValueData,
}

impl RawExecutionParameterData {
    fn validate(&self) -> Result<(), RawExecutionProtocolError> {
        validate_raw_catalog_id(&self.id, "parameter")?;
        let bytes = match &self.value {
            RawParameterValueData::Recipe(value)
            | RawParameterValueData::Image(value)
            | RawParameterValueData::Target(value)
            | RawParameterValueData::Task(value)
            | RawParameterValueData::UserInterface(value)
            | RawParameterValueData::File(value)
            | RawParameterValueData::Text(value)
            | RawParameterValueData::Multiconfig(value) => value.len(),
            RawParameterValueData::Integer(_) => 1,
            RawParameterValueData::Unknown => {
                return Err(RawExecutionProtocolError::UnknownRequiredVariant);
            }
        };
        if bytes == 0 || bytes > MAX_RAW_EXECUTION_PARAMETER_BYTES {
            return Err(RawExecutionProtocolError::InvalidParameter);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum RawParameterValueData {
    Recipe(String),
    Image(String),
    Target(String),
    Task(String),
    UserInterface(String),
    File(String),
    Integer(u32),
    Text(String),
    Multiconfig(String),
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawInteractionData {
    NoninteractiveJob,
    InteractivePty,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawSafetyData {
    Inspection,
    Build,
    MetadataMutation,
    Destructive,
    ServerLifecycle,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum RawExecutionOwnerData {
    Job(String),
    Pty(String),
    #[serde(other)]
    Unknown,
}

impl RawExecutionOwnerData {
    fn validate(&self) -> Result<(), RawExecutionProtocolError> {
        match self {
            Self::Job(id) => validate_raw_identity(id, "raw-job:", "job"),
            Self::Pty(id) => validate_raw_identity(id, "raw-session:", "session"),
            Self::Unknown => Err(RawExecutionProtocolError::UnknownRequiredVariant),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawOutputStreamData {
    Stdout,
    Stderr,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawOutputChunkData {
    pub schema_version: u16,
    pub stream_id: String,
    pub stream: RawOutputStreamData,
    pub sequence: u64,
    pub text: String,
    pub truncated_bytes: u64,
    pub dropped_lines: u64,
}

impl RawOutputChunkData {
    pub fn validate(&self) -> Result<(), RawExecutionProtocolError> {
        validate_raw_schema(self.schema_version)?;
        validate_raw_identity(&self.stream_id, "raw-stream:", "stream")?;
        if self.sequence == 0
            || self.text.len() > MAX_RAW_EXECUTION_OUTPUT_CHUNK_BYTES
            || matches!(self.stream, RawOutputStreamData::Unknown)
        {
            return Err(RawExecutionProtocolError::InvalidOutputChunk);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawExecutionOutcomeData {
    Succeeded,
    Failed,
    Cancelled,
    Lost,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawExecutionResultData {
    pub schema_version: u16,
    pub outcome: RawExecutionOutcomeData,
    pub exit_code: Option<i32>,
    pub message: Option<String>,
    pub elapsed_ms: u64,
    pub durable_reference: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawHistoryRecordData {
    pub schema_version: u16,
    pub request_id: String,
    pub catalog_version: u16,
    pub command_id: String,
    pub parameters: Vec<RawExecutionParameterData>,
    pub interaction: RawInteractionData,
    pub started_unix_ms: u64,
    pub ended_unix_ms: u64,
    pub outcome: RawExecutionOutcomeData,
    pub exit_code: Option<i32>,
    pub durable_reference: Option<String>,
}

impl RawHistoryRecordData {
    pub fn from_terminal(
        execution: &RawExecutionSnapshotData,
    ) -> Result<Self, RawExecutionProtocolError> {
        execution.validate()?;
        let RawExecutionPhaseData::Terminal(outcome) = execution.phase else {
            return Err(RawExecutionProtocolError::HistoryRequiresTerminal);
        };
        let result = execution
            .result
            .as_ref()
            .ok_or(RawExecutionProtocolError::HistoryRequiresTerminal)?;
        let started_unix_ms = execution
            .started_unix_ms
            .unwrap_or(execution.queued_unix_ms);
        let record = Self {
            schema_version: RAW_HISTORY_SCHEMA_VERSION,
            request_id: execution.request.request_id.clone(),
            catalog_version: execution.request.catalog_version,
            command_id: execution.request.command_id.clone(),
            parameters: execution
                .request
                .parameters
                .iter()
                .filter(|parameter| {
                    !matches!(
                        &parameter.value,
                        RawParameterValueData::File(_) | RawParameterValueData::Text(_)
                    )
                })
                .cloned()
                .collect(),
            interaction: execution.request.interaction,
            started_unix_ms,
            ended_unix_ms: started_unix_ms.saturating_add(result.elapsed_ms),
            outcome,
            exit_code: result.exit_code,
            durable_reference: result.durable_reference.clone(),
        };
        record.validate()?;
        Ok(record)
    }

    pub fn validate(&self) -> Result<(), RawExecutionProtocolError> {
        if self.schema_version != RAW_HISTORY_SCHEMA_VERSION {
            return Err(RawExecutionProtocolError::UnsupportedHistorySchema(
                self.schema_version,
            ));
        }
        validate_raw_identity(&self.request_id, "raw-request:", "request")?;
        validate_raw_catalog_id(&self.command_id, "command")?;
        if self.catalog_version == 0
            || self.ended_unix_ms < self.started_unix_ms
            || self.parameters.len() > MAX_RAW_EXECUTION_PARAMETERS
            || matches!(self.interaction, RawInteractionData::Unknown)
            || matches!(self.outcome, RawExecutionOutcomeData::Unknown)
        {
            return Err(RawExecutionProtocolError::InvalidHistoryRecord);
        }
        let mut parameter_ids = std::collections::BTreeSet::new();
        for parameter in &self.parameters {
            parameter.validate()?;
            if !parameter_ids.insert(&parameter.id) {
                return Err(RawExecutionProtocolError::DuplicateParameter);
            }
        }
        if let Some(reference) = &self.durable_reference {
            validate_raw_identity(reference, "raw-durable:", "durable reference")?;
        }
        match (self.outcome, self.exit_code) {
            (RawExecutionOutcomeData::Succeeded, Some(0))
            | (RawExecutionOutcomeData::Failed, Some(_))
            | (RawExecutionOutcomeData::Failed, None)
            | (RawExecutionOutcomeData::Cancelled, Some(_))
            | (RawExecutionOutcomeData::Cancelled, None)
            | (RawExecutionOutcomeData::Lost, None) => Ok(()),
            _ => Err(RawExecutionProtocolError::InvalidHistoryRecord),
        }
    }
}

pub fn validate_raw_history_records(
    records: &[RawHistoryRecordData],
) -> Result<(), RawExecutionProtocolError> {
    if records.len() > MAX_RAW_HISTORY_RECORDS {
        return Err(RawExecutionProtocolError::TooManyHistoryRecords);
    }
    let mut request_ids = std::collections::BTreeSet::new();
    let mut previous_end = u64::MAX;
    for record in records {
        record.validate()?;
        if !request_ids.insert(&record.request_id) || record.ended_unix_ms > previous_end {
            return Err(RawExecutionProtocolError::InvalidHistoryOrder);
        }
        previous_end = record.ended_unix_ms;
    }
    let bytes = serde_json::to_vec(records)
        .map_err(|_| RawExecutionProtocolError::InvalidHistoryRecord)?
        .len();
    if bytes > MAX_RAW_HISTORY_AGGREGATE_BYTES {
        return Err(RawExecutionProtocolError::HistoryTooLarge);
    }
    Ok(())
}

pub fn remember_raw_history(snapshot: &mut DaemonSnapshot, record: RawHistoryRecordData) {
    snapshot
        .raw_history
        .retain(|current| current.request_id != record.request_id);
    let insertion = snapshot
        .raw_history
        .partition_point(|current| current.ended_unix_ms >= record.ended_unix_ms);
    snapshot.raw_history.insert(insertion, record);
    snapshot.raw_history.truncate(MAX_RAW_HISTORY_RECORDS);
    while serde_json::to_vec(&snapshot.raw_history)
        .is_ok_and(|bytes| bytes.len() > MAX_RAW_HISTORY_AGGREGATE_BYTES)
    {
        snapshot.raw_history.pop();
    }
}

impl RawExecutionResultData {
    pub fn validate(&self) -> Result<(), RawExecutionProtocolError> {
        validate_raw_schema(self.schema_version)?;
        if matches!(self.outcome, RawExecutionOutcomeData::Unknown) {
            return Err(RawExecutionProtocolError::UnknownRequiredVariant);
        }
        if self
            .message
            .as_ref()
            .is_some_and(|message| message.len() > MAX_RAW_EXECUTION_MESSAGE_BYTES)
        {
            return Err(RawExecutionProtocolError::ResultMessageTooLong);
        }
        if let Some(reference) = &self.durable_reference {
            validate_raw_identity(reference, "raw-durable:", "durable reference")?;
        }
        if matches!(self.outcome, RawExecutionOutcomeData::Lost) && self.exit_code.is_some() {
            return Err(RawExecutionProtocolError::InvalidResult);
        }
        if matches!(self.outcome, RawExecutionOutcomeData::Succeeded) && self.exit_code != Some(0) {
            return Err(RawExecutionProtocolError::InvalidResult);
        }
        Ok(())
    }
}
