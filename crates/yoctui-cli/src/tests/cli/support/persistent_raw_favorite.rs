use super::*;

pub(crate) fn persistent_raw_favorite() -> yoctui_model::RawFavorite {
    let command = yoctui_model::builtin_raw_catalog()
        .commands
        .iter()
        .find(|command| {
            matches!(
                command.execution,
                yoctui_model::RawExecutionPolicy::Executable { .. }
            )
        })
        .unwrap();
    yoctui_model::RawFavorite::new(
        command,
        "My Raw favorite",
        BTreeMap::new(),
        yoctui_model::RawAdditionalArguments::from_vec(vec!["--dry-run".into()]).unwrap(),
        0,
    )
    .unwrap()
}
