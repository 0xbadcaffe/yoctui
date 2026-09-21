#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawOutputStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawOutputChunk {
    pub stream_id: RawStreamId,
    pub stream: RawOutputStream,
    pub sequence: u64,
    pub text: String,
    pub truncated_bytes: u64,
    pub dropped_lines: u64,
}

impl RawOutputChunk {
    pub fn validate(&self) -> Result<(), RawExecutionError> {
        RawStreamId::new(self.stream_id.as_str())?;
        if self.sequence == 0 || self.text.len() > MAX_RAW_OUTPUT_CHUNK_BYTES {
            return Err(RawExecutionError::InvalidOutputChunk);
        }
        Ok(())
    }

    fn line_count(&self) -> usize {
        raw_output_line_count(&self.text)
    }
}

fn raw_output_line_count(text: &str) -> usize {
    if text.is_empty() {
        1
    } else {
        text.bytes().filter(|byte| *byte == b'\n').count() + usize::from(!text.ends_with('\n'))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawRetainedOutput {
    pub stream_id: RawStreamId,
    pub stream: RawOutputStream,
    pub chunks: VecDeque<RawOutputChunk>,
    pub next_sequence: u64,
    pub retained_bytes: usize,
    pub retained_lines: usize,
    pub dropped_bytes: u64,
    pub dropped_lines: u64,
    pub truncated_chunks: u64,
}

impl RawRetainedOutput {
    pub fn new(stream_id: RawStreamId, stream: RawOutputStream) -> Self {
        Self {
            stream_id,
            stream,
            chunks: VecDeque::new(),
            next_sequence: 1,
            retained_bytes: 0,
            retained_lines: 0,
            dropped_bytes: 0,
            dropped_lines: 0,
            truncated_chunks: 0,
        }
    }

    fn append(&mut self, chunk: RawOutputChunk) -> Result<bool, RawExecutionError> {
        chunk.validate()?;
        if chunk.stream_id != self.stream_id || chunk.stream != self.stream {
            return Err(RawExecutionError::WrongOutputStream);
        }
        if chunk.sequence < self.next_sequence {
            return Ok(false);
        }
        if chunk.sequence != self.next_sequence {
            return Err(RawExecutionError::OutputGap {
                expected: self.next_sequence,
                actual: chunk.sequence,
            });
        }
        self.next_sequence = self
            .next_sequence
            .checked_add(1)
            .ok_or(RawExecutionError::SequenceExhausted)?;
        self.retained_bytes = self.retained_bytes.saturating_add(chunk.text.len());
        self.retained_lines = self.retained_lines.saturating_add(chunk.line_count());
        self.dropped_bytes = self.dropped_bytes.saturating_add(chunk.truncated_bytes);
        self.dropped_lines = self.dropped_lines.saturating_add(chunk.dropped_lines);
        self.truncated_chunks = self
            .truncated_chunks
            .saturating_add(u64::from(chunk.truncated_bytes > 0));
        self.chunks.push_back(chunk);
        while self.retained_bytes > MAX_RAW_OUTPUT_RETAINED_BYTES
            || self.retained_lines > MAX_RAW_OUTPUT_RETAINED_LINES
            || self.chunks.len() > MAX_RAW_OUTPUT_RETAINED_LINES
        {
            let Some(removed) = self.chunks.pop_front() else {
                break;
            };
            let removed_bytes = removed.text.len();
            let removed_lines = removed.line_count();
            self.retained_bytes = self.retained_bytes.saturating_sub(removed_bytes);
            self.retained_lines = self.retained_lines.saturating_sub(removed_lines);
            self.dropped_bytes = self.dropped_bytes.saturating_add(removed_bytes as u64);
            self.dropped_lines = self.dropped_lines.saturating_add(removed_lines as u64);
        }
        Ok(true)
    }

    pub fn validate(&self) -> Result<(), RawExecutionError> {
        RawStreamId::new(self.stream_id.as_str())?;
        if self.next_sequence == 0
            || self.retained_bytes > MAX_RAW_OUTPUT_RETAINED_BYTES
            || self.retained_lines > MAX_RAW_OUTPUT_RETAINED_LINES
            || self.chunks.len() > MAX_RAW_OUTPUT_RETAINED_LINES
            || self.retained_bytes != self.chunks.iter().map(|chunk| chunk.text.len()).sum()
            || self.retained_lines != self.chunks.iter().map(RawOutputChunk::line_count).sum()
        {
            return Err(RawExecutionError::InvalidOutputSnapshot);
        }
        let mut expected = self
            .chunks
            .front()
            .map(|chunk| chunk.sequence)
            .unwrap_or(self.next_sequence);
        for chunk in &self.chunks {
            chunk.validate()?;
            if chunk.stream_id != self.stream_id
                || chunk.stream != self.stream
                || chunk.sequence != expected
            {
                return Err(RawExecutionError::InvalidOutputSnapshot);
            }
            expected = expected
                .checked_add(1)
                .ok_or(RawExecutionError::SequenceExhausted)?;
        }
        if expected != self.next_sequence {
            return Err(RawExecutionError::InvalidOutputSnapshot);
        }
        Ok(())
    }
}
