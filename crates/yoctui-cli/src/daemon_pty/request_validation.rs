use std::{collections::BTreeMap, path::PathBuf};
use yoctui_model::{
    PtyCommandIdentity, PtyDimensions, PtySessionId, PtySessionKind, PtySessionSpec,
    PtyWorkspaceContext,
};
use yoctui_protocol::daemon::{PtyCommand, PtyKind, TerminalDimensions};

pub(super) fn wire_spec(
    id: PtySessionId,
    name: String,
    kind: PtyKind,
    cwd: String,
    command: PtyCommand,
    dimensions: TerminalDimensions,
) -> Result<PtySessionSpec, String> {
    let cwd = PathBuf::from(cwd);
    if !cwd.is_absolute() {
        return Err("PTY cwd must be absolute".into());
    }
    validate_dimensions(dimensions)?;
    Ok(PtySessionSpec {
        id,
        name,
        kind: match kind {
            PtyKind::BuildShell => PtySessionKind::BuildShell,
            PtyKind::SourceShell => PtySessionKind::SourceShell,
            PtyKind::LayerShell => PtySessionKind::LayerShell,
            PtyKind::RecipeShell => PtySessionKind::RecipeShell,
            PtyKind::DevtoolShell => PtySessionKind::DevtoolShell,
            PtyKind::Devshell => PtySessionKind::Devshell,
            PtyKind::Menuconfig => PtySessionKind::Menuconfig,
            PtyKind::SdkShell => PtySessionKind::SdkShell,
            PtyKind::NativeShell => PtySessionKind::NativeShell,
            PtyKind::QemuConsole => PtySessionKind::QemuConsole,
            PtyKind::SshConsole => PtySessionKind::SshConsole,
            PtyKind::Utility => PtySessionKind::InteractiveTool,
        },
        cwd: cwd.clone(),
        command: PtyCommandIdentity {
            executable: command.program.into(),
            arguments: command.arguments,
        },
        dimensions: PtyDimensions {
            columns: dimensions.columns,
            rows: dimensions.rows,
        },
        restartable: true,
        workspace: PtyWorkspaceContext {
            source_dir: cwd.clone(),
            build_dir: cwd.clone(),
            authorized_context_roots: vec![cwd.clone()],
            owner_identity: "daemon".into(),
        },
    })
}

pub(super) fn validate_dimensions(dimensions: TerminalDimensions) -> Result<(), String> {
    if dimensions.columns == 0
        || dimensions.rows == 0
        || dimensions.columns > 512
        || dimensions.rows > 512
    {
        return Err("PTY dimensions must be within 1..=512".into());
    }
    Ok(())
}

pub(super) fn inherited_environment() -> BTreeMap<String, String> {
    let mut environment = std::env::vars().collect();
    ensure_interactive_terminal_environment(&mut environment);
    environment
}

pub(super) fn ensure_interactive_terminal_environment(environment: &mut BTreeMap<String, String>) {
    if environment
        .get("TERM")
        .is_none_or(|value| value.is_empty() || value == "dumb")
    {
        environment.insert("TERM".into(), "xterm-256color".into());
    }
}
