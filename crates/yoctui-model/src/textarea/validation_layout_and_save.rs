impl TextAreaState {
    pub fn set_validation<I>(&mut self, spans: I)
    where
        I: IntoIterator<Item = TextAreaValidationSpan>,
    {
        self.advanced.validation = spans
            .into_iter()
            .take(TEXTAREA_MAX_VALIDATION_SPANS)
            .map(|mut span| {
                span.start = clamp_boundary(&self.text, span.start);
                span.end = clamp_boundary(&self.text, span.end);
                if span.start > span.end {
                    std::mem::swap(&mut span.start, &mut span.end);
                }
                span.message = bounded_utf8(span.message, 4_096);
                span
            })
            .collect();
    }

    pub fn visual_lines(&self, first: usize, limit: usize) -> Vec<TextAreaVisualLine> {
        let cap = limit.min(4_096);
        if cap == 0 {
            return Vec::new();
        }
        let mut projected = Vec::with_capacity(cap);
        let mut visual_index = 0usize;
        for (source_line, (start, end)) in line_ranges(&self.text).into_iter().enumerate() {
            let width = self.advanced.layout.wrap_width.filter(|width| *width > 0);
            let chunks = wrap_ranges(&self.text, start, end, width);
            for (index, (chunk_start, chunk_end)) in chunks.into_iter().enumerate() {
                if visual_index >= first {
                    projected.push(TextAreaVisualLine {
                        source_line,
                        start: chunk_start,
                        end: chunk_end,
                        continuation: index > 0,
                    });
                    if projected.len() == cap {
                        return projected;
                    }
                }
                visual_index = visual_index.saturating_add(1);
            }
        }
        projected
    }

    pub fn preview_diff(&mut self) -> &TextAreaDiffPreview {
        let preview = build_diff(&self.advanced.diff_base_text, &self.text);
        self.advanced.save = TextAreaSaveState::Preview { preview };
        let TextAreaSaveState::Preview { preview } = &self.advanced.save else {
            unreachable!()
        };
        preview
    }

    pub fn begin_atomic_save(
        &mut self,
        target: impl Into<PathBuf>,
        observed: TextAreaRevision,
    ) -> Option<TextAreaAtomicSave> {
        let target = target.into();
        let expected = TextAreaRevision::of(&self.advanced.base_text);
        if observed != expected {
            self.advanced.save = TextAreaSaveState::Conflict {
                target,
                expected,
                observed,
            };
            return None;
        }
        let revision = TextAreaRevision::of(&self.text);
        let temporary = atomic_temporary_path(&target, &revision);
        self.advanced.save = TextAreaSaveState::Saving {
            target: target.clone(),
            temporary: temporary.clone(),
            revision,
        };
        Some(TextAreaAtomicSave {
            target,
            temporary,
            content: self.text.clone(),
            revision,
        })
    }

    pub fn mark_saved(&mut self, request: &TextAreaAtomicSave) -> bool {
        let matches = matches!(
            &self.advanced.save,
            TextAreaSaveState::Saving { target, temporary, revision }
                if target == &request.target && temporary == &request.temporary && revision == &request.revision
        ) && TextAreaRevision::of(&self.text) == request.revision;
        if matches {
            self.advanced.base_text.clone_from(&self.text);
            self.advanced.diff_base_text.clone_from(&self.text);
            self.advanced.save = TextAreaSaveState::Saved {
                target: request.target.clone(),
                revision: request.revision,
            };
        }
        matches
    }

    pub fn mark_save_failed(&mut self, message: impl Into<String>, recoverable: bool) -> bool {
        let TextAreaSaveState::Saving {
            target,
            temporary,
            revision,
        } = &self.advanced.save
        else {
            return false;
        };
        self.advanced.save = TextAreaSaveState::Failed {
            target: target.clone(),
            temporary: temporary.clone(),
            revision: *revision,
            message: bounded_utf8(message.into(), 8_192),
            recoverable,
        };
        true
    }

    pub fn retry_save(&mut self) -> Result<TextAreaAtomicSave, TextAreaError> {
        let TextAreaSaveState::Failed {
            target,
            temporary,
            revision,
            recoverable,
            ..
        } = &self.advanced.save
        else {
            return Err(TextAreaError::NoSaveFailure);
        };
        if !recoverable {
            return Err(TextAreaError::NoSaveFailure);
        }
        let request = TextAreaAtomicSave {
            target: target.clone(),
            temporary: temporary.clone(),
            content: self.text.clone(),
            revision: *revision,
        };
        self.advanced.save = TextAreaSaveState::Saving {
            target: request.target.clone(),
            temporary: request.temporary.clone(),
            revision: request.revision,
        };
        Ok(request)
    }

}
