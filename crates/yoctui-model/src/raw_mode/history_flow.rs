impl RawModeState {
    pub fn workspace_status_rows(&self) -> u16 {
        u16::from(self.notification.is_some()) + u16::from(self.view == RawModeView::Browser
            && (self.search.editing || !self.search.query.is_empty()))
    }
    /// Active requests and retained records share one newest-first projection.
    /// Never mistake retained history metadata for a live execution authority.
    pub fn history_request_ids(&self) -> Vec<RawRequestId> {
        let mut entries = BTreeMap::new();
        for record in &self.history {
            entries.insert(record.request_id.clone(), record.started_unix_ms);
        }
        for (id, execution) in &self.execution_states {
            entries.insert(id.clone(), execution.queued_unix_ms);
        }
        let mut entries: Vec<_> = entries.into_iter().collect();
        entries.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        entries.into_iter().map(|(id, _)| id).collect()
    }

    pub fn selected_history_request(&self) -> Option<RawRequestId> {
        self.history_request_ids().get(self.history_selection).cloned()
    }

    /// Keep the selected request stable while new daemon snapshots arrive.
    pub fn restore_history_selection(&mut self, selected: Option<&RawRequestId>) {
        let ids = self.history_request_ids();
        self.history_selection = selected.and_then(|id| ids.iter().position(|entry| entry == id))
            .unwrap_or_else(|| self.history_selection.min(ids.len().saturating_sub(1)));
    }

    pub fn modal_active(&self) -> bool {
        self.recipe_picker.is_some() || self.favorite_confirmation.is_some()
            || self.favorite_rename.is_some()
    }

    pub fn owns_escape(&self) -> bool {
        self.modal_active() || self.view != RawModeView::Browser
            || self.search.editing || !self.search.query.is_empty()
            || self.browser_column == RawBrowserColumn::Commands
    }
}

pub fn raw_category_column_width(width: u16) -> u16 {
    ((u32::from(width) * 52) / 100) as u16
}

fn open_raw_history_output(state: &mut RawModeState, catalog: &RawCatalog) {
    let Some(id) = state.selected_history_request() else {
        state.notification = Some("No Raw execution is selected.".into());
        return;
    };
    if let Some(execution) = state.execution_states.get(&id) {
        state.execution = Some(execution.request.command.clone());
        state.output = RawOutputViewState { request: Some(id), ..Default::default() };
        state.enter_view(RawModeView::Execution, RawModeFocus::Execution);
    } else {
        let stale = state.history.iter().find(|record| record.request_id == id)
            .is_none_or(|record| catalog.command(&record.command).is_none());
        state.notification = Some(if stale {
            "The retained Raw command is stale or unavailable.".into()
        } else {
            "Only history metadata is retained; output is unavailable. Press r to review a new command (nothing runs automatically).".into()
        });
    }
}

fn review_raw_history(state: &mut RawModeState, catalog: &RawCatalog,
    authority: Option<&DaemonCompatibilitySnapshot>) {
    let Some(id) = state.selected_history_request() else { return; };
    let intent = state.execution_states.get(&id).map(|execution| (
        execution.request.command.clone(), execution.request.parameters.clone(),
        execution.request.additional_arguments.clone(),
    )).or_else(|| state.history.iter().find(|record| record.request_id == id)
        .map(|record| (record.command.clone(), record.parameters.clone(), Vec::new())));
    let Some((id, parameters, arguments)) = intent else { return; };
    let Some(command) = catalog.command(&id) else {
        state.notification = Some("The retained Raw command is stale or unavailable.".into());
        return;
    };
    let Ok(arguments) = RawAdditionalArguments::from_vec(arguments) else { return; };
    state.command = Some(command.id.clone());
    state.category = Some(command.category.clone());
    open_raw_form(state, command, authority, &parameters, &arguments);
}
