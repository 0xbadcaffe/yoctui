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
mod tests {
    use super::*;
    use crate::{Action, App, update};
    #[test]
    fn clone_activity_reducer_is_idempotent() {
        let mut app = App::new(10, 1024);
        for _ in 0..2 {
            update(
                &mut app,
                Action::SetBackgroundActivity {
                    activity: BackgroundActivity::Cloning,
                    active: true,
                },
            );
        }
        assert_eq!(app.background_activities.len(), 1);
        assert!(update(&mut app, Action::ConfirmBuildEnvironmentClone).is_none());
        update(
            &mut app,
            Action::SetBackgroundActivity {
                activity: BackgroundActivity::Cloning,
                active: false,
            },
        );
        assert!(app.background_activities.is_empty());
    }
}
