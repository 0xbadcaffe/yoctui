impl RawModeState {
    pub fn new(catalog: &RawCatalog) -> Self {
        let mut state = Self {
            catalog_version: catalog.version,
            category: catalog
                .browser_categories()
                .first()
                .map(|category| category.id.clone()),
            command: None,
            browser_column: RawBrowserColumn::Categories,
            view: RawModeView::Browser,
            focus: RawModeFocus::Categories,
            search: RawSearchState::default(),
            form: None,
            preview: None,
            execution: None,
            execution_states: BTreeMap::new(),
            output: RawOutputViewState::default(),
            history: Vec::new(),
            history_selection: 0,
            favorites: Vec::new(),
            favorite_selection: 0,
            favorite_confirmation: None,
            notification: None,
            return_stack: Vec::new(),
        };
        reconcile_raw_mode(&mut state, catalog);
        state
    }

    pub fn selected_command<'a>(&self, catalog: &'a RawCatalog) -> Option<&'a RawCommand> {
        self.command
            .as_ref()
            .and_then(|command| catalog.command(command))
    }

    pub fn visible_commands<'a>(&self, catalog: &'a RawCatalog) -> Vec<&'a RawCommand> {
        raw_visible_commands(self, catalog)
    }

    pub fn is_favorite(&self, command: &RawCommandId) -> bool {
        self.favorites
            .iter()
            .any(|favorite| &favorite.command == command)
    }

    pub fn selected_execution(&self) -> Option<&RawExecutionState> {
        if let Some(request) = &self.output.request
            && let Some(execution) = self.execution_states.get(request)
        {
            return Some(execution);
        }
        let command = self.execution.as_ref()?;
        self.execution_states
            .values()
            .rev()
            .find(|execution| &execution.request.command == command)
    }

    fn enter_view(&mut self, view: RawModeView, focus: RawModeFocus) {
        if self.return_stack.len() == MAX_RAW_VIEW_DEPTH {
            self.return_stack.remove(0);
        }
        self.return_stack.push((self.view, self.focus));
        self.view = view;
        self.focus = focus;
    }

    fn leave_view(&mut self) {
        match self.view {
            RawModeView::Preview => self.preview = None,
            RawModeView::Form => self.form = None,
            RawModeView::Execution => {
                self.execution = None;
                self.output = RawOutputViewState::default();
            }
            RawModeView::Browser | RawModeView::History | RawModeView::Favorites => {}
        }
        if let Some((view, focus)) = self.return_stack.pop() {
            self.view = view;
            self.focus = focus;
        } else {
            self.view = RawModeView::Browser;
            self.focus = match self.browser_column {
                RawBrowserColumn::Categories => RawModeFocus::Categories,
                RawBrowserColumn::Commands => RawModeFocus::Commands,
            };
        }
    }

    fn close_unsafe_work(&mut self, reason: String) {
        self.form = None;
        self.preview = None;
        self.execution = None;
        self.return_stack.clear();
        self.view = RawModeView::Browser;
        self.browser_column = RawBrowserColumn::Commands;
        self.focus = RawModeFocus::Commands;
        self.notification = Some(reason);
    }
}
