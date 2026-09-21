//! Named client operations whose work happens outside the input/render loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BackgroundActivity {
    Cloning,
    Initializing,
    Cancelling,
    Loading,
}
impl BackgroundActivity {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Cloning => "Cloning… · Esc cancels in Build Environment",
            Self::Initializing => "Initializing…",
            Self::Cancelling => "Cancelling…",
            Self::Loading => "Loading…",
        }
    }
}

#[cfg(test)]
#[path = "tests/background_activity/mod.rs"]
mod tests;
