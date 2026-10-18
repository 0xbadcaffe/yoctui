use crate::*;
use std::path::Path;

// The only shell text is fixed source/exec plumbing. Every path and fragment is
// a separate argument; generated wrappers reset positional parameters themselves.
pub const SETUP_FRAGMENT_SHELL: &str = r#"set -e
unset BBSERVER BBPATH BUILDDIR OEROOT BITBAKEDIR TEMPLATECONF
yoctui_setup_script=$1
yoctui_setup_build=$2
yoctui_setup_generated=$3
shift 3
yoctui_setup_argv=("$@")
if [ "$yoctui_setup_generated" = generated ]; then
    set --
    source "$yoctui_setup_script" >&2
else
    source "$yoctui_setup_script" "$yoctui_setup_build" >&2
fi
cd "$yoctui_setup_build"
exec "${yoctui_setup_argv[@]}"
"#;

impl SetupOperationDialog {
    pub fn launch(&self) -> Result<TerminalLaunchRequest, String> {
        let profile = &self.profile;
        for path in [
            &profile.source_dir,
            &profile.build_dir,
            &profile.init_script,
        ] {
            let text = path.to_str().ok_or("Paths must be UTF-8")?;
            if !path.is_absolute()
                || text.len() > 4096
                || text.chars().any(char::is_control)
                || path
                    .components()
                    .any(|c| matches!(c, std::path::Component::ParentDir))
            {
                return Err(
                    "Choose absolute build/source/script paths without '..' or control characters."
                        .into(),
                );
            }
        }
        let mut request = TerminalLaunchRequest {
            name: format!(
                "{}: {}",
                self.operation.label(),
                profile.build_dir.display()
            ),
            kind: TerminalCreationKind::Utility,
            cwd: profile.source_dir.clone(),
            program: "/bin/bash".into(),
            arguments: Vec::new(),
            completion: None,
        };
        // PTY names have a fixed wire bound; keep the full path in the review/cwd.
        while request.name.len() > 128 {
            request.name.pop();
        }
        if matches!(
            self.operation,
            SetupOperation::Status | SetupOperation::Update
        ) {
            let root = profile.build_dir.parent().ok_or("Missing setup parent")?;
            if profile.build_dir.file_name().is_none_or(|n| n != "build") {
                return Err(
                    "Status/update requires a managed bitbake-setup setup, not a legacy build."
                        .into(),
                );
            }
            let top = root.parent().ok_or("Missing top directory")?;
            let plan = BitBakeSetupPlan::from_values(&[
                self.tool.clone(),
                top.display().to_string(),
                "check".into(),
                "".into(),
                "".into(),
            ])?;
            let mut args = plan.launch()?.arguments;
            args.truncate(8); // Only the two per-invocation top-directory settings.
            args.extend(["--color".into(), "never".into()]);
            if self.offline {
                args.push("--no-network".into());
            }
            args.extend([
                if self.operation == SetupOperation::Status {
                    "status"
                } else {
                    "update"
                }
                .into(),
                "--setup-dir".into(),
                root.display().to_string(),
            ]);
            if self.operation == SetupOperation::Update {
                args.extend([
                    "--update-bb-conf".into(),
                    self.config_policy().into(),
                    "--rebase-conflicts-strategy".into(),
                    if self.backup_conflicts {
                        "backup"
                    } else {
                        "abort"
                    }
                    .into(),
                ]);
                request.completion = Some(TerminalCompletion::SetupChanged {
                    profile: profile.clone(),
                });
            }
            request.cwd = top.into();
            request.program = Path::new(&self.tool).into();
            request.arguments = args;
        } else {
            if self.operation != SetupOperation::Fragments && !valid_setup_fragment(&self.fragment)
            {
                return Err(
                    "Choose a complete valid fragment ID; disabling prefixes is not supported."
                        .into(),
                );
            }
            request.arguments = vec![
                "-c".into(),
                SETUP_FRAGMENT_SHELL.into(),
                "yoctui-fragment".into(),
                profile.init_script.display().to_string(),
                profile.build_dir.display().to_string(),
                if profile.init_script == profile.build_dir.join("init-build-env") {
                    "generated"
                } else {
                    "legacy"
                }
                .into(),
                "bitbake-config-build".into(),
                "--color".into(),
                "never".into(),
                match self.operation {
                    SetupOperation::Fragments => "list-fragments",
                    SetupOperation::Show => "show-fragment",
                    SetupOperation::Enable => "enable-fragment",
                    SetupOperation::Disable => "disable-fragment",
                    _ => unreachable!(),
                }
                .into(),
            ];
            if self.operation != SetupOperation::Fragments {
                request.arguments.push(self.fragment.clone());
            }
            if matches!(
                self.operation,
                SetupOperation::Enable | SetupOperation::Disable
            ) {
                request.arguments.extend([
                    "--confpath".into(),
                    profile
                        .build_dir
                        .join("conf/toolcfg.conf")
                        .display()
                        .to_string(),
                ]);
                request.completion = Some(TerminalCompletion::SetupChanged {
                    profile: profile.clone(),
                });
            }
        }
        Ok(request)
    }
}
