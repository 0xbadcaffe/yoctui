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
mod tests {
    use std::{collections::BTreeMap, fs};

    use super::*;
    use crate::{PtyContextEntry, VerifiedPtyEnvironment};

    #[test]
    fn pty_sdk_shell_routes_installed_and_native_environments() {
        let root = std::env::temp_dir().join(format!(
            "yoctui-app-sdk-shell-{}-{}",
            std::process::id(),
            std::time::SystemTime::UNIX_EPOCH
                .elapsed()
                .unwrap()
                .as_nanos()
        ));
        for directory in ["source", "build", "sdk"] {
            fs::create_dir_all(root.join(directory)).unwrap();
        }
        let shell = fs::canonicalize("/bin/bash").unwrap();
        let authority = PtyContextAuthority::new(
            "workspace".into(),
            root.join("source"),
            root.join("build"),
            VerifiedPtyEnvironment {
                identity: "build-env".into(),
                shell: shell.clone(),
                environment: BTreeMap::from([(
                    "BUILDDIR".into(),
                    root.join("build").display().to_string(),
                )]),
            },
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            vec![(
                PtyContextEntry {
                    identity: "sdk-a".into(),
                    directory: root.join("sdk"),
                },
                VerifiedPtyEnvironment {
                    identity: "sdk-env-a".into(),
                    shell,
                    environment: BTreeMap::from([(
                        "SDKTARGETSYSROOT".into(),
                        "/sdk/sysroot".into(),
                    )]),
                },
            )],
        )
        .unwrap();
        let router = PtySdkShellRouter::new(authority);
        let sdk = router
            .preview(PtySdkShellAction::InstalledSdk {
                identity: "sdk-a".into(),
            })
            .unwrap();
        assert_eq!(sdk.kind, PtySessionKind::SdkShell);
        assert_eq!(sdk.environment_identity, "sdk-env-a");
        assert_eq!(sdk.command.arguments, vec!["-i"]);
        let native = router
            .preview(PtySdkShellAction::NativeBuildEnvironment)
            .unwrap();
        assert_eq!(native.kind, PtySessionKind::NativeShell);
        assert_eq!(native.environment_identity, "build-env");
        assert_eq!(native.cwd, fs::canonicalize(root.join("build")).unwrap());
        fs::remove_dir_all(root).unwrap();
    }
}
