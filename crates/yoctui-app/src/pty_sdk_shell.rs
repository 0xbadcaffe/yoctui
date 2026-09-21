use crate::{PtyContextAction, PtyContextAuthority, PtyContextError, PtyContextLaunch};
use yoctui_model::PtySessionKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PtySdkShellAction {
    InstalledSdk { identity: String },
    NativeBuildEnvironment,
}

pub type PtySdkShellPreview = PtyContextLaunch;

pub struct PtySdkShellRouter {
    contexts: PtyContextAuthority,
}

impl PtySdkShellRouter {
    pub fn new(contexts: PtyContextAuthority) -> Self {
        Self { contexts }
    }

    pub fn preview(
        &self,
        action: PtySdkShellAction,
    ) -> Result<PtySdkShellPreview, PtyContextError> {
        match action {
            PtySdkShellAction::InstalledSdk { identity } => self
                .contexts
                .resolve(PtyContextAction::SdkEnvironment { identity }),
            PtySdkShellAction::NativeBuildEnvironment => {
                let mut launch = self.contexts.resolve(PtyContextAction::BuildDirectory)?;
                launch.name = "Native build environment".into();
                launch.kind = PtySessionKind::NativeShell;
                Ok(launch)
            }
        }
    }
}

#[cfg(test)]
#[path = "tests/pty_sdk_shell/mod.rs"]
mod tests;
