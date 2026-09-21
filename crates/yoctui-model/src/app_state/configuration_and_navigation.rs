impl App {
    pub fn new_unconfigured(max_entries: usize, max_bytes: usize) -> Self {
        let mut app = Self::new(max_entries, max_bytes);
        app.build_environment = BuildEnvironmentState::Unconfigured;
        app.screen = Screen::BuildEnvironment;
        app.focus = FocusTarget::Navigator;
        app
    }
    pub fn install_keymap(&mut self, preferences: KeymapPreferences) -> Result<(), KeymapError> {
        let preferences = preferences.migrate()?;
        let effective = EffectiveKeymap::from_preferences(&preferences)?;
        self.keymap_preferences = preferences;
        self.effective_keymap = effective;
        self.keymap_chord.clear();
        Ok(())
    }
    pub fn reset_keymap(&mut self) {
        self.keymap_preferences = KeymapPreferences::default();
        self.effective_keymap = EffectiveKeymap::default();
        self.keymap_chord.clear();
    }
    pub fn effective_keymap_report(&self) -> String {
        self.effective_keymap.report()
    }
    pub fn elapsed(&self) -> Option<Duration> {
        self.build
            .started
            .and_then(|s| SystemTime::now().duration_since(s).ok())
    }
    pub fn navigator_screen(&self) -> Screen {
        NAVIGATOR_SCREENS
            .get(self.navigator_selection)
            .copied()
            .unwrap_or(Screen::Dashboard)
    }
    pub fn navigator_compatibility_destination(&self) -> WorkspaceDestination {
        NAVIGATOR_COMPATIBILITY_DESTINATIONS
            .get(self.navigator_selection)
            .copied()
            .unwrap_or(WorkspaceDestination::Dashboard)
    }
    pub fn navigator_group_index(&self) -> usize {
        navigator_group_for_selection(self.navigator_selection)
    }
    pub fn navigator_visible_row_count(&self) -> usize {
        NAVIGATOR_GROUPS.len()
            + NAVIGATOR_GROUPS
                .iter()
                .enumerate()
                .filter(|(index, _)| self.navigator_groups_expanded[*index])
                .map(|(_, group)| group.end - group.start)
                .sum::<usize>()
    }
    pub fn navigator_visual_row(&self) -> usize {
        let selected_group = self.navigator_group_index();
        let rows_before = NAVIGATOR_GROUPS
            .iter()
            .enumerate()
            .take(selected_group)
            .map(|(index, group)| {
                1 + usize::from(self.navigator_groups_expanded[index]) * (group.end - group.start)
            })
            .sum::<usize>();
        if self.navigator_groups_expanded[selected_group] {
            rows_before
                + 1
                + self
                    .navigator_selection
                    .saturating_sub(NAVIGATOR_GROUPS[selected_group].start)
        } else {
            rows_before
        }
    }
    pub fn navigator_selection_at_visual_row(&self, visual_row: usize) -> Option<usize> {
        let mut cursor = 0;
        for (group_index, group) in NAVIGATOR_GROUPS.iter().enumerate() {
            if visual_row == cursor {
                return None;
            }
            cursor += 1;
            if !self.navigator_groups_expanded[group_index] {
                continue;
            }
            let end = cursor + group.end - group.start;
            if visual_row < end {
                return Some(group.start + visual_row - cursor);
            }
            cursor = end;
        }
        None
    }
    pub fn navigator_group_at_visual_row(&self, visual_row: usize) -> Option<usize> {
        let mut cursor = 0;
        for (group_index, group) in NAVIGATOR_GROUPS.iter().enumerate() {
            if visual_row == cursor {
                return Some(group_index);
            }
            cursor += 1;
            if self.navigator_groups_expanded[group_index] {
                cursor += group.end - group.start;
            }
        }
        None
    }
    pub fn navigator_viewport_start(&self, visible_rows: usize) -> usize {
        self.navigator_visual_row()
            .saturating_sub(visible_rows.saturating_sub(1))
            .min(
                self.navigator_visible_row_count()
                    .saturating_sub(visible_rows),
            )
    }
    pub(crate) fn navigator_selection_is_visible(&self, selection: usize) -> bool {
        let group = navigator_group_for_selection(selection);
        self.navigator_groups_expanded[group] || selection == NAVIGATOR_GROUPS[group].start
    }
}
