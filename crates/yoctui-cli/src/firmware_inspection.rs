//! Firmware inspection.
use super::*;

pub(crate) async fn firmware_variable_hint(
    backend: &mut dyn BitBakeBackend,
    name: &str,
    image: Option<&str>,
) -> Option<String> {
    if let Some(image) = image
        && let Ok(value) = backend
            .get_variable(name.into(), Some(image.to_owned()))
            .await
        && let Some(value) = value.value
        && !value.trim().is_empty()
    {
        return Some(value.trim().to_owned());
    }
    backend
        .get_variable(name.into(), None)
        .await
        .ok()
        .and_then(|value| value.value)
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

pub(crate) fn push_firmware_candidate(candidates: &mut Vec<String>, candidate: &str) {
    let candidate = candidate.trim();
    if candidate.is_empty()
        || !candidate
            .chars()
            .all(|value| value.is_ascii_alphanumeric() || "+._-/".contains(value))
        || candidates.iter().any(|existing| existing == candidate)
    {
        return;
    }
    candidates.push(candidate.to_owned());
}

pub(crate) fn classify_firmware_component(
    target: &str,
    provider: Option<&Path>,
    uboot_machine: Option<&str>,
    efi_provider: Option<&str>,
) -> PlatformComponent {
    let identity = format!(
        "{} {}",
        target.to_ascii_lowercase(),
        provider
            .map(|path| path.display().to_string().to_ascii_lowercase())
            .unwrap_or_default()
    );
    if identity.contains("u-boot") || uboot_machine.is_some() {
        PlatformComponent::UBoot
    } else if identity.contains("ovmf")
        || identity.contains("edk2")
        || identity.contains("uefi")
        || identity.contains("efi")
        || identity.contains("seabios")
        || identity.contains("coreboot")
        || efi_provider.is_some()
    {
        PlatformComponent::BiosUefi
    } else {
        PlatformComponent::BootFirmware
    }
}

pub(crate) async fn inspect_firmware_workbench(
    image: Option<String>,
    recipes: Vec<String>,
    deploy_dir: Option<PathBuf>,
    backend: &mut dyn BitBakeBackend,
) -> Action {
    let image = image.as_deref();
    let preferred =
        firmware_variable_hint(backend, "PREFERRED_PROVIDER_virtual/bootloader", image).await;
    let runtime = firmware_variable_hint(backend, "VIRTUAL-RUNTIME_bootloader", image).await;
    let uboot_machine = firmware_variable_hint(backend, "UBOOT_MACHINE", image).await;
    let efi_provider = firmware_variable_hint(backend, "EFI_PROVIDER", image).await;

    let mut candidates = Vec::new();
    if let Some(candidate) = preferred.as_deref() {
        push_firmware_candidate(&mut candidates, candidate);
    }
    if uboot_machine.is_some() {
        push_firmware_candidate(&mut candidates, "virtual/bootloader");
    }
    if let Some(candidate) = efi_provider.as_deref() {
        push_firmware_candidate(&mut candidates, candidate);
    }
    if let Some(candidate) = runtime.as_deref() {
        push_firmware_candidate(&mut candidates, candidate);
    }
    push_firmware_candidate(&mut candidates, "virtual/bootloader");
    for recipe in recipes {
        let name = recipe.to_ascii_lowercase();
        if [
            "u-boot",
            "uboot",
            "ovmf",
            "edk2",
            "uefi",
            "seabios",
            "coreboot",
            "grub-efi",
            "systemd-boot",
        ]
        .iter()
        .any(|needle| name.contains(needle))
        {
            push_firmware_candidate(&mut candidates, &recipe);
        }
    }

    let mut selected = None;
    let mut failures = Vec::new();
    for candidate in candidates {
        match backend.get_recipe_metadata(candidate.clone()).await {
            Ok(metadata) => {
                selected = Some((candidate, metadata));
                break;
            }
            Err(error) => failures.push(format!("{candidate}: {error}")),
        }
    }
    let Some((target, metadata)) = selected else {
        let detail = failures.first().map_or(
            "no boot firmware candidate was reported".to_owned(),
            Clone::clone,
        );
        return Action::FirmwareFailed(format!(
            "could not resolve U-Boot or BIOS/UEFI for the active image ({detail})"
        ));
    };

    let mut roots = Vec::new();
    let mut limitations = Vec::new();
    let mut provider = None;
    for variable in ["FILE", "S", "B", "WORKDIR"] {
        match backend
            .get_variable(variable.into(), Some(target.clone()))
            .await
        {
            Ok(value) => {
                let Some(value) = value.value.filter(|value| !value.trim().is_empty()) else {
                    limitations.push(format!("{variable} was not reported for {target}."));
                    continue;
                };
                let path = PathBuf::from(value);
                if variable == "FILE" {
                    if path.is_absolute() {
                        provider = Some(path);
                    } else {
                        limitations.push("Firmware provider FILE was not absolute.".into());
                    }
                } else if path.is_absolute() {
                    roots.push(path);
                } else {
                    limitations.push(format!("Firmware {variable} was not absolute."));
                }
            }
            Err(error) => limitations.push(format!("Could not query firmware {variable}: {error}")),
        }
    }
    if let Some(deploy) = deploy_dir {
        roots.push(deploy);
    }
    let component = classify_firmware_component(
        &target,
        provider.as_deref(),
        uboot_machine.as_deref(),
        efi_provider.as_deref(),
    );
    let scan = tokio::task::spawn_blocking(move || PlatformArtifactAdapter.scan(roots)).await;
    match scan {
        Ok(Ok(scan)) => {
            limitations.extend(scan.limitations);
            Action::FirmwareLoaded(PlatformInventory {
                component,
                target,
                provider,
                tasks: metadata.tasks.unwrap_or_default(),
                roots: scan.roots,
                files: scan.files,
                dtc: scan.dtc,
                limitations,
            })
        }
        Ok(Err(error)) => Action::FirmwareFailed(error.to_string()),
        Err(error) => Action::FirmwareFailed(format!("artifact scanner did not complete: {error}")),
    }
}
