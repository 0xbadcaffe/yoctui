#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum RawExecutionOwner {
    Job(RawJobId),
    Pty(RawSessionId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawExecutionOutcome {
    Succeeded,
    Failed,
    Cancelled,
    Lost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", content = "outcome", rename_all = "snake_case")]
pub enum RawExecutionPhase {
    Queued,
    Starting,
    Running,
    Cancelling,
    Terminal(RawExecutionOutcome),
}

impl RawExecutionPhase {
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Terminal(_))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawAttachmentState {
    Attached,
    Detached,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RawEventCursor {
    pub sequence: u64,
    pub generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawExecutionResult {
    pub outcome: RawExecutionOutcome,
    pub exit_code: Option<i32>,
    pub message: Option<String>,
    pub elapsed_ms: u64,
    pub durable_reference: Option<RawDurableReferenceId>,
}

impl RawExecutionResult {
    pub fn validate(&self) -> Result<(), RawExecutionError> {
        if self
            .message
            .as_ref()
            .is_some_and(|message| message.len() > MAX_RAW_EXECUTION_MESSAGE_BYTES)
        {
            return Err(RawExecutionError::ResultMessageTooLong);
        }
        if let Some(reference) = &self.durable_reference {
            RawDurableReferenceId::new(reference.as_str())?;
        }
        match (self.outcome, self.exit_code) {
            (RawExecutionOutcome::Succeeded, Some(0))
            | (RawExecutionOutcome::Failed, Some(_))
            | (RawExecutionOutcome::Failed, None)
            | (RawExecutionOutcome::Cancelled, Some(_))
            | (RawExecutionOutcome::Cancelled, None)
            | (RawExecutionOutcome::Lost, None) => Ok(()),
            (RawExecutionOutcome::Succeeded, Some(_))
            | (RawExecutionOutcome::Succeeded, None)
            | (RawExecutionOutcome::Lost, Some(_)) => Err(RawExecutionError::InvalidResult),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawExecutionState {
    pub request: RawConfirmedExecutionRequest,
    pub phase: RawExecutionPhase,
    pub attachment: RawAttachmentState,
    pub owner: Option<RawExecutionOwner>,
    pub cancellation_requested: bool,
    pub queued_unix_ms: u64,
    pub started_unix_ms: Option<u64>,
    pub elapsed_ms: u64,
    pub result: Option<RawExecutionResult>,
    pub stdout: RawRetainedOutput,
    pub stderr: RawRetainedOutput,
    pub cursor: RawEventCursor,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawHistoryRecord {
    pub schema_version: u16,
    pub request_id: RawRequestId,
    pub catalog_version: u16,
    pub command: RawCommandId,
    pub parameters: BTreeMap<RawParameterId, RawParameterValue>,
    pub interaction: RawInteractionMode,
    pub started_unix_ms: u64,
    pub ended_unix_ms: u64,
    pub outcome: RawExecutionOutcome,
    pub exit_code: Option<i32>,
    pub durable_reference: Option<RawDurableReferenceId>,
}

impl RawHistoryRecord {
    pub fn from_terminal(execution: &RawExecutionState) -> Result<Self, RawExecutionError> {
        execution.validate()?;
        let RawExecutionPhase::Terminal(outcome) = execution.phase else {
            return Err(RawExecutionError::HistoryRequiresTerminal);
        };
        let result = execution
            .result
            .as_ref()
            .ok_or(RawExecutionError::HistoryRequiresTerminal)?;
        let started_unix_ms = execution
            .started_unix_ms
            .unwrap_or(execution.queued_unix_ms);
        let record = Self {
            schema_version: RAW_HISTORY_SCHEMA_VERSION,
            request_id: execution.request.id.clone(),
            catalog_version: execution.request.catalog_version,
            command: execution.request.command.clone(),
            parameters: execution
                .request
                .parameters
                .iter()
                .filter(|(_, value)| {
                    !matches!(
                        value,
                        RawParameterValue::File(_) | RawParameterValue::Text(_)
                    )
                })
                .map(|(parameter, value)| (parameter.clone(), value.clone()))
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

    pub fn validate(&self) -> Result<(), RawExecutionError> {
        if self.schema_version != RAW_HISTORY_SCHEMA_VERSION {
            return Err(RawExecutionError::UnsupportedHistorySchema(
                self.schema_version,
            ));
        }
        RawRequestId::new(self.request_id.as_str())?;
        RawCommandId::new(self.command.as_str()).map_err(|_| RawExecutionError::InvalidCommand)?;
        if self.catalog_version == 0 || self.ended_unix_ms < self.started_unix_ms {
            return Err(RawExecutionError::InvalidHistoryRecord);
        }
        if self.parameters.len() > MAX_RAW_PARAMETERS {
            return Err(RawExecutionError::TooManyParameters);
        }
        for (parameter, value) in &self.parameters {
            RawParameterId::new(parameter.as_str())
                .map_err(|_| RawExecutionError::InvalidParameterValue)?;
            validate_raw_execution_parameter_value(value)?;
        }
        if let Some(reference) = &self.durable_reference {
            RawDurableReferenceId::new(reference.as_str())?;
        }
        match (self.outcome, self.exit_code) {
            (RawExecutionOutcome::Succeeded, Some(0))
            | (RawExecutionOutcome::Failed, Some(_))
            | (RawExecutionOutcome::Failed, None)
            | (RawExecutionOutcome::Cancelled, Some(_))
            | (RawExecutionOutcome::Cancelled, None)
            | (RawExecutionOutcome::Lost, None) => Ok(()),
            _ => Err(RawExecutionError::InvalidHistoryRecord),
        }
    }
}

pub fn install_raw_history(
    history: &mut Vec<RawHistoryRecord>,
    records: impl IntoIterator<Item = RawHistoryRecord>,
) -> Result<(), RawExecutionError> {
    let mut replacement = Vec::new();
    for record in records {
        record.validate()?;
        if let Some(index) = replacement
            .iter()
            .position(|existing: &RawHistoryRecord| existing.request_id == record.request_id)
        {
            replacement.remove(index);
        }
        replacement.push(record);
    }
    replacement.sort_by_key(|record| std::cmp::Reverse(record.ended_unix_ms));
    replacement.truncate(MAX_RAW_HISTORY_RECORDS);
    *history = replacement;
    Ok(())
}

impl RawExecutionState {
    pub fn queued(
        request: RawConfirmedExecutionRequest,
        stdout: RawStreamId,
        stderr: RawStreamId,
        queued_unix_ms: u64,
        cursor: RawEventCursor,
    ) -> Result<Self, RawExecutionError> {
        request.validate()?;
        if stdout == stderr {
            return Err(RawExecutionError::DuplicateStreamIdentity);
        }
        let state = Self {
            request,
            phase: RawExecutionPhase::Queued,
            attachment: RawAttachmentState::Attached,
            owner: None,
            cancellation_requested: false,
            queued_unix_ms,
            started_unix_ms: None,
            elapsed_ms: 0,
            result: None,
            stdout: RawRetainedOutput::new(stdout, RawOutputStream::Stdout),
            stderr: RawRetainedOutput::new(stderr, RawOutputStream::Stderr),
            cursor,
        };
        state.validate()?;
        Ok(state)
    }

    pub fn validate(&self) -> Result<(), RawExecutionError> {
        self.request.validate()?;
        self.stdout.validate()?;
        self.stderr.validate()?;
        if self.stdout.stream_id == self.stderr.stream_id
            || self.stdout.stream != RawOutputStream::Stdout
            || self.stderr.stream != RawOutputStream::Stderr
        {
            return Err(RawExecutionError::DuplicateStreamIdentity);
        }
        if let Some(owner) = &self.owner {
            match owner {
                RawExecutionOwner::Job(id) => {
                    RawJobId::new(id.as_str())?;
                    if self.request.interaction != RawInteractionMode::NoninteractiveJob {
                        return Err(RawExecutionError::WrongOwnerKind);
                    }
                }
                RawExecutionOwner::Pty(id) => {
                    RawSessionId::new(id.as_str())?;
                    if self.request.interaction != RawInteractionMode::InteractivePty {
                        return Err(RawExecutionError::WrongOwnerKind);
                    }
                }
            }
        }
        match (self.phase, &self.result) {
            (RawExecutionPhase::Terminal(outcome), Some(result)) if outcome == result.outcome => {
                result.validate()?;
                if self.elapsed_ms != result.elapsed_ms {
                    return Err(RawExecutionError::InvalidResult);
                }
            }
            (RawExecutionPhase::Terminal(_), _) | (_, Some(_)) => {
                return Err(RawExecutionError::InvalidResult);
            }
            _ => {}
        }
        match self.phase {
            RawExecutionPhase::Queued if self.owner.is_some() || self.started_unix_ms.is_some() => {
                return Err(RawExecutionError::InvalidLifecycle);
            }
            RawExecutionPhase::Starting
                if self.owner.is_none() || self.started_unix_ms.is_some() =>
            {
                return Err(RawExecutionError::InvalidLifecycle);
            }
            RawExecutionPhase::Running
                if self.owner.is_none() || self.started_unix_ms.is_none() =>
            {
                return Err(RawExecutionError::InvalidLifecycle);
            }
            RawExecutionPhase::Cancelling
                if !self.cancellation_requested
                    || self.started_unix_ms.is_some() && self.owner.is_none() =>
            {
                return Err(RawExecutionError::InvalidLifecycle);
            }
            RawExecutionPhase::Terminal(RawExecutionOutcome::Cancelled)
                if !self.cancellation_requested =>
            {
                return Err(RawExecutionError::InvalidLifecycle);
            }
            _ => {}
        }
        Ok(())
    }
}
