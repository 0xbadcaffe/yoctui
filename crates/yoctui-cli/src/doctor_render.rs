//! Doctor render.
use super::*;

pub(crate) fn render_doctor_compatibility(report: &DoctorCompatibilityReport) -> String {
    let mut lines = vec![
        "compatibility report:".into(),
        format!("  authority: {:?}", report.authority),
    ];
    if let Some(reason) = &report.authority_reason {
        lines.push(format!("  authority reason: {reason}"));
    }
    lines.push(format!(
        "  release support: {:?} ({})",
        report.release_support, report.release_support_reason
    ));
    let Some(environment) = &report.environment else {
        return lines.join("\n");
    };
    lines.extend([
        format!("  snapshot generation: {}", report.generation.unwrap_or(0)),
        format!("  operating mode: {:?}", report.operating_mode),
        format!(
            "  build directory: {}",
            doctor_detected(&environment.build_directory, Clone::clone)
        ),
        format!(
            "  BitBake: {}",
            doctor_detected(&environment.bitbake_version, Clone::clone)
        ),
        format!(
            "  OE-Core: {}",
            doctor_detected(&environment.oe_core, |release| format!(
                "{} {}",
                release.name.as_deref().unwrap_or("unknown"),
                release.version.as_deref().unwrap_or("unknown")
            ))
        ),
        format!(
            "  Poky: {}",
            doctor_detected(&environment.poky, |release| format!(
                "{} {}",
                release.name.as_deref().unwrap_or("unknown"),
                release.version.as_deref().unwrap_or("unknown")
            ))
        ),
        format!(
            "  DISTRO: {}",
            doctor_detected(&environment.distro, |distro| format!(
                "{} {}",
                distro.name,
                distro.version.as_deref().unwrap_or("unknown")
            ))
        ),
        format!(
            "  MACHINE: {}",
            doctor_detected(&environment.machine, Clone::clone)
        ),
        format!(
            "  backend: {}",
            doctor_detected(&environment.backend, |backend| format!(
                "{} {}",
                backend.name,
                backend.version.as_deref().unwrap_or("unknown")
            ))
        ),
        format!(
            "  protocol: {}",
            doctor_detected(&environment.protocol, |protocol| format!(
                "{} {}",
                protocol.name, protocol.version
            ))
        ),
        format!(
            "  capabilities: available={} limited={} unavailable={} unknown={} unsupported={}",
            report.summary.available,
            report.summary.limited,
            report.summary.unavailable,
            report.summary.unknown,
            report.summary.unsupported
        ),
    ]);
    if let yoctui_protocol::daemon::CompatibilityDetected::Detected { value, authority } =
        &environment.source_roots
    {
        for root in value {
            lines.push(format!(
                "  source root: {}={} [{authority:?}]",
                root.kind, root.path
            ));
        }
    }
    if let yoctui_protocol::daemon::CompatibilityDetected::Detected { value, authority } =
        &environment.layer_series
    {
        for layer in value {
            lines.push(format!(
                "  layer series: {}={} ({}) [{authority:?}]",
                layer.layer,
                layer.root,
                layer.compatible_series.join(", ")
            ));
        }
    }
    if let yoctui_protocol::daemon::CompatibilityDetected::Detected { value, authority } =
        &environment.available_tools
    {
        for tool in value {
            lines.push(format!(
                "  available tool: {}={} {} [{authority:?}]",
                tool.id,
                tool.executable,
                tool.version.as_deref().unwrap_or("unknown")
            ));
        }
    }
    for missing in &report.missing_tools {
        lines.push(format!(
            "  missing tool: {} — {}",
            missing.tool, missing.reason
        ));
    }
    for (label, issues) in [
        ("limited", &report.limited_features),
        ("unavailable", &report.unavailable_features),
        ("unsupported", &report.unsupported_features),
        ("unknown", &report.unknown_features),
    ] {
        for issue in issues {
            let implementation = issue
                .implementation
                .as_deref()
                .map_or(String::new(), |value| format!("; implementation={value}"));
            let limitations = if issue.limitations.is_empty() {
                String::new()
            } else {
                format!("; limitations={}", issue.limitations.join(" | "))
            };
            lines.push(format!(
                "  {label}: {} — {} [{}]{}{}",
                issue.id, issue.reason, issue.reason_code, limitations, implementation
            ));
        }
    }
    lines.join("\n")
}
