use super::validation as v;
use super::{DevtoolUtilityCommand as C, DevtoolUtilityDraft};

pub(super) fn build_arguments(draft: &DevtoolUtilityDraft) -> Result<Vec<String>, String> {
    let mut args = vec![draft.command.subcommand().to_owned()];
    match draft.command {
        C::Add => add(draft, &mut args)?,
        C::Modify => modify(draft, &mut args)?,
        C::Upgrade => upgrade(draft, &mut args)?,
        C::Status => {}
        C::LatestVersion => latest_version(draft, &mut args)?,
        C::CheckUpgradeStatus => check_upgrade(draft, &mut args)?,
        C::Search => args.push(v::required(draft.value(0), "search expression")?.into()),
        C::Build => build(draft, &mut args)?,
        C::IdeSdk => ide_sdk(draft, &mut args)?,
        C::Rename => rename(draft, &mut args)?,
        C::EditRecipe | C::FindRecipe => edit_or_find(draft, &mut args)?,
        C::ConfigureHelp => configure_help(draft, &mut args)?,
        C::UpdateRecipe => update_recipe(draft, &mut args)?,
        C::Reset => reset(draft, &mut args)?,
        C::Finish => finish(draft, &mut args)?,
        C::DeployTarget => deploy(draft, &mut args)?,
        C::UndeployTarget => undeploy(draft, &mut args)?,
        C::BuildImage => build_image(draft, &mut args)?,
        C::CreateWorkspace => create_workspace(draft, &mut args)?,
        C::Export => export(draft, &mut args)?,
        C::Extract => extract(draft, &mut args)?,
        C::Sync => sync(draft, &mut args)?,
        C::Import => import(draft, &mut args)?,
        C::Menuconfig => args.push(v::token(draft.value(0), "recipe/component")?.into()),
    }
    Ok(args)
}

fn add(d: &DevtoolUtilityDraft, a: &mut Vec<String>) -> Result<(), String> {
    choice_flag(
        d,
        a,
        4,
        "same source",
        "--same-dir",
        "separate",
        "--no-same-dir",
    );
    flags(d, a, &[(5, "--npm-dev"), (6, "--no-pypi")]);
    option_token(d, a, 7, "--version", "recipe version")?;
    flag(d, a, 8, "--no-git");
    match d.value(9) {
        "fixed SRCREV" => {
            a.push("--srcrev".into());
            a.push(v::token(d.value(10), "SRCREV")?.into());
        }
        "AUTOREV" => a.push("--autorev".into()),
        _ => {
            if !d.value(10).is_empty() {
                return Err("select fixed SRCREV before entering an SRCREV value".into());
            }
        }
    }
    option_token(d, a, 11, "--srcbranch", "source branch")?;
    flags(
        d,
        a,
        &[(12, "--binary"), (13, "--also-native"), (15, "--mirrors")],
    );
    option_token(d, a, 14, "--src-subdir", "source subdirectory")?;
    option_token(d, a, 16, "--provides", "provides alias")?;
    let recipe = v::optional_token(d.value(0), "recipe name")?;
    let source = v::optional_absolute_path(d.value(1), "source tree")?;
    let fetch = v::optional_token(d.value(2), "fetch URI")?;
    if source.is_some() && recipe.is_none() {
        return Err("recipe name is required before a source tree".into());
    }
    if fetch.is_some() && recipe.is_none() {
        return Err("recipe name is required before a fetch URI".into());
    }
    if d.value(3) == "--fetch (deprecated)" {
        if let Some(fetch) = fetch {
            a.extend(["--fetch".into(), fetch.into()]);
        }
        push_optional(a, recipe);
        push_optional(a, source);
    } else {
        push_optional(a, recipe);
        push_optional(a, source);
        push_optional(a, fetch);
    }
    Ok(())
}

fn modify(d: &DevtoolUtilityDraft, a: &mut Vec<String>) -> Result<(), String> {
    flag(d, a, 2, "--wildcard");
    choice_flag(
        d,
        a,
        3,
        "extract",
        "--extract",
        "no extract",
        "--no-extract",
    );
    choice_flag(
        d,
        a,
        4,
        "same source",
        "--same-dir",
        "separate",
        "--no-same-dir",
    );
    option_token(d, a, 5, "--branch", "development branch")?;
    flags(
        d,
        a,
        &[
            (6, "--no-overrides"),
            (7, "--keep-temp"),
            (8, "--debug-build"),
        ],
    );
    a.push(v::token(d.value(0), "recipe name")?.into());
    push_optional(a, v::optional_absolute_path(d.value(1), "source tree")?);
    Ok(())
}

fn upgrade(d: &DevtoolUtilityDraft, a: &mut Vec<String>) -> Result<(), String> {
    flag(d, a, 2, "--stable");
    option_token(d, a, 3, "--version", "version")?;
    option_token(d, a, 4, "--srcrev", "SRCREV")?;
    option_token(d, a, 5, "--srcbranch", "source branch")?;
    option_token(d, a, 6, "--branch", "development branch")?;
    flags(d, a, &[(7, "--no-patch"), (8, "--no-overrides")]);
    choice_flag(
        d,
        a,
        9,
        "same source",
        "--same-dir",
        "separate",
        "--no-same-dir",
    );
    flags(d, a, &[(10, "--keep-temp"), (11, "--keep-failure")]);
    a.push(v::token(d.value(0), "recipe name")?.into());
    push_optional(a, v::optional_absolute_path(d.value(1), "source tree")?);
    Ok(())
}

fn latest_version(d: &DevtoolUtilityDraft, a: &mut Vec<String>) -> Result<(), String> {
    flag(d, a, 1, "--stable");
    a.push(v::token(d.value(0), "recipe name")?.into());
    Ok(())
}

fn check_upgrade(d: &DevtoolUtilityDraft, a: &mut Vec<String>) -> Result<(), String> {
    flags(d, a, &[(1, "--stable"), (2, "--all")]);
    a.extend(v::tokens(d.value(0), "recipe name", false)?);
    Ok(())
}

fn build(d: &DevtoolUtilityDraft, a: &mut Vec<String>) -> Result<(), String> {
    flags(d, a, &[(1, "--disable-parallel-make"), (2, "--clean")]);
    a.push(v::token(d.value(0), "recipe name")?.into());
    Ok(())
}

fn ide_sdk(d: &DevtoolUtilityDraft, a: &mut Vec<String>) -> Result<(), String> {
    option_choice(d, a, 1, "--mode", "modified");
    option_choice(d, a, 2, "--ide", "code");
    option_token(d, a, 3, "--target", "target")?;
    option_port(d, a, 4, "--gdbserver-port-start", "GDB server port")?;
    flag(d, a, 5, "--no-host-check");
    option_token(d, a, 6, "--ssh-exec", "SSH executable")?;
    option_port(d, a, 7, "--port", "SSH port")?;
    option_path(d, a, 8, "--key", "SSH private key")?;
    flags(
        d,
        a,
        &[
            (9, "--skip-bitbake"),
            (10, "--bitbake-k"),
            (11, "--no-strip"),
            (12, "--dry-run"),
            (13, "--show-status"),
            (14, "--no-preserve"),
            (15, "--no-check-space"),
        ],
    );
    a.extend(v::tokens(d.value(0), "recipe/image name", true)?);
    Ok(())
}

fn rename(d: &DevtoolUtilityDraft, a: &mut Vec<String>) -> Result<(), String> {
    option_token(d, a, 2, "--version", "version")?;
    flag(d, a, 3, "--no-srctree");
    a.push(v::token(d.value(0), "recipe name")?.into());
    push_optional(a, v::optional_token(d.value(1), "new recipe name")?);
    Ok(())
}

fn edit_or_find(d: &DevtoolUtilityDraft, a: &mut Vec<String>) -> Result<(), String> {
    flag(d, a, 1, "--any-recipe");
    a.push(v::token(d.value(0), "recipe name")?.into());
    Ok(())
}

fn configure_help(d: &DevtoolUtilityDraft, a: &mut Vec<String>) -> Result<(), String> {
    flags(d, a, &[(1, "--no-pager"), (2, "--no-header")]);
    a.push(v::token(d.value(0), "recipe name")?.into());
    let extra = v::trailing_arguments(d.value(3))?;
    if !extra.is_empty() {
        a.push("--arg".into());
        a.extend(extra);
    }
    Ok(())
}

fn update_recipe(d: &DevtoolUtilityDraft, a: &mut Vec<String>) -> Result<(), String> {
    option_choice(d, a, 1, "--mode", "auto");
    option_token(d, a, 2, "--initial-rev", "initial revision")?;
    option_path(d, a, 3, "--append", "append layer directory")?;
    flags(
        d,
        a,
        &[
            (4, "--wildcard-version"),
            (5, "--no-remove"),
            (6, "--no-overrides"),
            (7, "--dry-run"),
            (8, "--force-patch-refresh"),
        ],
    );
    a.push(v::token(d.value(0), "recipe name")?.into());
    Ok(())
}

fn reset(d: &DevtoolUtilityDraft, a: &mut Vec<String>) -> Result<(), String> {
    let recipes = v::tokens(d.value(0), "recipe name", false)?;
    if d.value(1) == "yes" {
        if !recipes.is_empty() {
            return Err("reset all cannot be combined with recipe names".into());
        }
        a.push("--all".into());
    } else if recipes.is_empty() {
        return Err("enter at least one recipe or select reset all".into());
    }
    flags(d, a, &[(2, "--no-clean"), (3, "--remove-work")]);
    a.extend(recipes);
    Ok(())
}

fn finish(d: &DevtoolUtilityDraft, a: &mut Vec<String>) -> Result<(), String> {
    option_choice(d, a, 2, "--mode", "auto");
    option_token(d, a, 3, "--initial-rev", "initial revision")?;
    flags(
        d,
        a,
        &[
            (4, "--force"),
            (5, "--remove-work"),
            (6, "--no-clean"),
            (7, "--no-overrides"),
            (8, "--dry-run"),
            (9, "--force-patch-refresh"),
        ],
    );
    a.push(v::token(d.value(0), "recipe name")?.into());
    let destination = v::required(d.value(1), "destination layer/path")?;
    if destination.starts_with('-') {
        return Err("destination layer/path cannot begin with '-'".into());
    }
    a.push(destination.into());
    Ok(())
}

fn deploy(d: &DevtoolUtilityDraft, a: &mut Vec<String>) -> Result<(), String> {
    flags(
        d,
        a,
        &[
            (2, "--no-host-check"),
            (3, "--show-status"),
            (4, "--dry-run"),
            (5, "--no-preserve"),
            (6, "--no-check-space"),
        ],
    );
    ssh_options(d, a, 7, 8, 9)?;
    choice_flag(d, a, 10, "strip", "--strip", "no strip", "--no-strip");
    a.push(v::token(d.value(0), "recipe name")?.into());
    a.push(v::token(d.value(1), "target")?.into());
    Ok(())
}

fn undeploy(d: &DevtoolUtilityDraft, a: &mut Vec<String>) -> Result<(), String> {
    flags(d, a, &[(2, "--no-host-check"), (3, "--show-status")]);
    let recipe = v::optional_token(d.value(0), "recipe name")?;
    if d.value(4) == "yes" {
        if recipe.is_some() {
            return Err("undeploy all cannot be combined with a recipe name".into());
        }
        a.push("--all".into());
    } else if recipe.is_none() {
        return Err("enter a recipe or select undeploy all".into());
    }
    flag(d, a, 5, "--dry-run");
    ssh_options(d, a, 6, 7, 8)?;
    push_optional(a, recipe);
    a.push(v::token(d.value(1), "target")?.into());
    Ok(())
}

fn build_image(d: &DevtoolUtilityDraft, a: &mut Vec<String>) -> Result<(), String> {
    if let Some(packages) = v::comma_list(d.value(1), "additional packages")? {
        a.extend(["--add-packages".into(), packages.into()]);
    }
    push_optional(a, v::optional_token(d.value(0), "image recipe")?);
    Ok(())
}

fn create_workspace(d: &DevtoolUtilityDraft, a: &mut Vec<String>) -> Result<(), String> {
    option_token(d, a, 1, "--layerseries", "layer series")?;
    flag(d, a, 2, "--create-only");
    push_optional(
        a,
        v::optional_absolute_path(d.value(0), "workspace layer path")?,
    );
    Ok(())
}

fn export(d: &DevtoolUtilityDraft, a: &mut Vec<String>) -> Result<(), String> {
    option_path(d, a, 0, "--file", "archive file")?;
    flag(d, a, 1, "--overwrite");
    let recipes = v::tokens(d.value(3), "recipe name", false)?;
    match d.value(2) {
        "include" | "exclude" if recipes.is_empty() => {
            return Err(format!(
                "{} filter requires at least one recipe",
                d.value(2)
            ));
        }
        "include" => {
            a.push("--include".into());
            a.extend(recipes);
        }
        "exclude" => {
            a.push("--exclude".into());
            a.extend(recipes);
        }
        _ if !recipes.is_empty() => {
            return Err("select include or exclude before entering recipe filters".into());
        }
        _ => {}
    }
    Ok(())
}

fn extract(d: &DevtoolUtilityDraft, a: &mut Vec<String>) -> Result<(), String> {
    option_token(d, a, 2, "--branch", "development branch")?;
    flags(d, a, &[(3, "--no-overrides"), (4, "--keep-temp")]);
    a.push(v::token(d.value(0), "recipe name")?.into());
    a.push(v::absolute_path(d.value(1), "source tree")?.into());
    Ok(())
}

fn sync(d: &DevtoolUtilityDraft, a: &mut Vec<String>) -> Result<(), String> {
    option_token(d, a, 2, "--branch", "development branch")?;
    flag(d, a, 3, "--keep-temp");
    a.push(v::token(d.value(0), "recipe name")?.into());
    a.push(v::absolute_path(d.value(1), "source tree")?.into());
    Ok(())
}

fn import(d: &DevtoolUtilityDraft, a: &mut Vec<String>) -> Result<(), String> {
    flag(d, a, 1, "--overwrite");
    a.push(v::absolute_path(d.value(0), "archive file")?.into());
    Ok(())
}

fn flag(d: &DevtoolUtilityDraft, a: &mut Vec<String>, index: usize, name: &str) {
    if d.value(index) == "yes" {
        a.push(name.into());
    }
}

fn flags(d: &DevtoolUtilityDraft, a: &mut Vec<String>, values: &[(usize, &str)]) {
    for (index, name) in values {
        flag(d, a, *index, name);
    }
}

fn choice_flag(
    d: &DevtoolUtilityDraft,
    a: &mut Vec<String>,
    index: usize,
    first_value: &str,
    first_flag: &str,
    second_value: &str,
    second_flag: &str,
) {
    match d.value(index) {
        value if value == first_value => a.push(first_flag.into()),
        value if value == second_value => a.push(second_flag.into()),
        _ => {}
    }
}

fn option_choice(
    d: &DevtoolUtilityDraft,
    a: &mut Vec<String>,
    index: usize,
    name: &str,
    default: &str,
) {
    if d.value(index) != default {
        a.extend([name.into(), d.value(index).into()]);
    }
}

fn option_token(
    d: &DevtoolUtilityDraft,
    a: &mut Vec<String>,
    index: usize,
    name: &str,
    label: &str,
) -> Result<(), String> {
    if let Some(value) = v::optional_token(d.value(index), label)? {
        a.extend([name.into(), value.into()]);
    }
    Ok(())
}

fn option_path(
    d: &DevtoolUtilityDraft,
    a: &mut Vec<String>,
    index: usize,
    name: &str,
    label: &str,
) -> Result<(), String> {
    if let Some(value) = v::optional_absolute_path(d.value(index), label)? {
        a.extend([name.into(), value.into()]);
    }
    Ok(())
}

fn option_port(
    d: &DevtoolUtilityDraft,
    a: &mut Vec<String>,
    index: usize,
    name: &str,
    label: &str,
) -> Result<(), String> {
    if let Some(value) = v::port(d.value(index), label)? {
        a.extend([name.into(), value.into()]);
    }
    Ok(())
}

fn ssh_options(
    d: &DevtoolUtilityDraft,
    a: &mut Vec<String>,
    executable: usize,
    port: usize,
    key: usize,
) -> Result<(), String> {
    option_token(d, a, executable, "--ssh-exec", "SSH executable")?;
    option_port(d, a, port, "--port", "SSH port")?;
    option_path(d, a, key, "--key", "SSH private key")
}

fn push_optional(a: &mut Vec<String>, value: Option<&str>) {
    if let Some(value) = value {
        a.push(value.into());
    }
}
