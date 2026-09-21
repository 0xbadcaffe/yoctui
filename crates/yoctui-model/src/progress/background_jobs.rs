impl BackgroundJob {
    pub fn progress_projection(&self) -> GaugeProjection {
        let terminal = match self.status {
            BackgroundJobStatus::Succeeded => Some(WidgetTerminalState::Success),
            BackgroundJobStatus::Failed | BackgroundJobStatus::Lost => {
                Some(WidgetTerminalState::Failure)
            }
            BackgroundJobStatus::Cancelled => Some(WidgetTerminalState::Cancelled),
            _ => None,
        };
        match (&self.progress, terminal) {
            (BackgroundJobProgress::Percent(value), Some(terminal)) => {
                GaugeProjection::terminal(&self.title, u64::from(*value), 100, terminal, "final")
            }
            (BackgroundJobProgress::Units { completed, total }, Some(terminal)) => {
                GaugeProjection::terminal(&self.title, *completed, *total, terminal, "final")
            }
            (BackgroundJobProgress::Percent(value), None) => GaugeProjection::determinate(
                &self.title,
                u64::from(*value),
                100,
                WidgetRole::Progress,
            ),
            (BackgroundJobProgress::Units { completed, total }, None) => {
                GaugeProjection::determinate(&self.title, *completed, *total, WidgetRole::Progress)
            }
            (BackgroundJobProgress::Indeterminate, Some(terminal)) => {
                terminal_progress(&self.title, 0, None, terminal, "progress not reported")
            }
            (BackgroundJobProgress::Indeterminate, None) => {
                GaugeProjection::indeterminate(&self.title, "progress unknown")
            }
        }
    }
}
