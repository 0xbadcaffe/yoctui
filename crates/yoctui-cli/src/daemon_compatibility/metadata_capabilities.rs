//! Upgrade missing task capabilities only from daemon-owned recipe datastores.
use yoctui_bitbake::{CapabilityProbeObservation, CapabilityProbeStatus, CapabilityResolver};
use yoctui_model::{
    CapabilityCatalog, CapabilityEvidence, CapabilityEvidenceKind, CapabilityEvidenceOutcome,
    CapabilityProbeSpec, DaemonCompatibilitySnapshot,
};

pub(crate) fn recover(
    mut current: DaemonCompatibilitySnapshot,
    metadata: &crate::daemon_startup::StartupWorkspace,
) -> anyhow::Result<Option<DaemonCompatibilitySnapshot>> {
    anyhow::ensure!(
        current.snapshot.environment.build_directory.value()
            == metadata.workspace.build_dir.as_ref(),
        "task metadata belongs to another build"
    );
    let mut changed = false;
    for entry in CapabilityCatalog::builtin().entries {
        if entry.probes.is_empty() {
            continue;
        }
        let mut evidence = Vec::new();
        for probe in &entry.probes {
            let matched = match probe {
                CapabilityProbeSpec::MetadataAnyTask { names } => {
                    metadata.task_metadata.iter().find_map(|(recipe, tasks)| {
                        names
                            .iter()
                            .find(|name| {
                                tasks
                                    .iter()
                                    .any(|task| task == *name || task == &format!("do_{name}"))
                            })
                            .map(|name| {
                                format!(
                                    "{} ({}) reports task {name}",
                                    recipe.name,
                                    recipe.file.display()
                                )
                            })
                    })
                }
                CapabilityProbeSpec::Configuration { name } if name == "qa_tasks" => {
                    metadata.task_metadata.iter().find_map(|(recipe, tasks)| {
                        tasks
                            .iter()
                            .find(|task| {
                                [
                                    "do_package_qa",
                                    "do_patch_qa",
                                    "do_kernel_configcheck",
                                    "do_populate_lic",
                                    "do_checkuri",
                                ]
                                .contains(&task.as_str())
                            })
                            .map(|task| {
                                format!(
                                    "{} ({}) reports QA task {task}",
                                    recipe.name,
                                    recipe.file.display()
                                )
                            })
                    })
                }
                _ => None,
            };
            let Some(detail) = matched else {
                evidence.clear();
                break;
            };
            evidence.push(CapabilityProbeObservation {
                status: CapabilityProbeStatus::Positive,
                evidence: CapabilityEvidence {
                    kind: CapabilityEvidenceKind::Metadata,
                    outcome: CapabilityEvidenceOutcome::Positive,
                    subject: "Recipe datastore task inventory".into(),
                    detail,
                    argv: Vec::new(),
                },
            });
        }
        if evidence.len() != entry.probes.len() {
            continue;
        }
        let resolved = CapabilityResolver::default().resolve(
            &entry,
            current
                .snapshot
                .environment
                .bitbake_version
                .value()
                .map(String::as_str),
            &evidence,
        );
        if let Some(record) = current
            .snapshot
            .capabilities
            .iter_mut()
            .find(|record| record.id == entry.id)
            && *record != resolved.record
        {
            *record = resolved.record;
            if let Some(implementation) = resolved.implementation {
                current.implementations.insert(entry.id, implementation);
            }
            changed = true;
        }
    }
    if !changed {
        return Ok(None);
    }
    current.snapshot.generation = current
        .snapshot
        .generation
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("capability generation exhausted"))?;
    Ok(Some(current.normalize()?))
}
