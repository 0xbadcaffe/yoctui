use super::*;

fn id<T>(value: &str) -> T
where
    T: TryFrom<&'static str>,
    <T as TryFrom<&'static str>>::Error: std::fmt::Debug,
{
    T::try_from(Box::leak(value.to_owned().into_boxed_str())).unwrap()
}

fn valid_catalog() -> RawCatalog {
    RawCatalog {
        version: RAW_CATALOG_VERSION,
        categories: vec![RawCategory {
            id: id("task-control"),
            label: "Task control".into(),
            reference_heading: "Recipe Task Execution".into(),
            kind: RawCategoryKind::Executable,
        }],
        commands: vec![RawCommand {
            id: id("task-control.run"),
            category: id("task-control"),
            label: "Run a recipe task".into(),
            description: "Execute one named task for a recipe.".into(),
            reference: RawReference {
                id: id("recipe-task-execution.run-task"),
                heading: "Recipe Task Execution".into(),
                command: "bitbake -c <task> <recipe>".into(),
                description: "Execute one named task for a recipe.".into(),
            },
            parameters: vec![
                RawParameter {
                    id: id("task"),
                    label: "Task".into(),
                    placeholder: "<task>".into(),
                    kind: RawParameterKind::Task,
                    presence: RawParameterPresence::Required,
                },
                RawParameter {
                    id: id("recipe"),
                    label: "Recipe".into(),
                    placeholder: "<recipe>".into(),
                    kind: RawParameterKind::Recipe,
                    presence: RawParameterPresence::Required,
                },
            ],
            execution: RawExecutionPolicy::Executable {
                template: RawExecutableTemplate {
                    executable: RawExecutable::BitBake,
                    arguments: vec![
                        RawArgument::Literal { value: "-c".into() },
                        RawArgument::Parameter {
                            parameter: id("task"),
                        },
                        RawArgument::Parameter {
                            parameter: id("recipe"),
                        },
                    ],
                    capabilities: RawCapabilityRequirement::All {
                        capabilities: vec![CapabilityId::BitBakeBuild],
                    },
                    interaction: RawInteractionMode::NoninteractiveJob,
                    safety: RawSafetyClass::Build,
                },
            },
        }],
    }
}

mod raw_catalog_model_accepts_valid_bounded_typed_catalog;

mod raw_catalog_model_rejects_partial_records;

mod raw_catalog_model_rejects_duplicate_identities;

mod raw_catalog_model_rejects_oversized_records;

mod raw_catalog_model_rejects_unsafe_or_disagreeing_templates;

mod raw_catalog_model_rejects_missing_or_duplicate_capability_policy;
