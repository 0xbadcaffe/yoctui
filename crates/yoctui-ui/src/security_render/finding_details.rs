pub(crate) fn security_cve_inspector(app: &App) -> String {
    let Some((report, finding)) = selected_security_finding(app) else {
        return "Select a typed CVE finding to inspect it.".into();
    };
    let mapping = display_security_metadata(&finding.mapping);
    let metadata = display_security_metadata(&report.metadata);
    let limitations = if report.limitations.is_empty() {
        "none".into()
    } else {
        report.limitations.join("\n")
    };
    format!(
        "Finding: {}\nStatus: {}\nRecipe: {}\nPackage: {}\nProduct: {}\nVersion: {}\nSeverity: {}\nScore: {}\nVector: {}\nAdvisory: {}\nSummary: {}\n\nExact report: {}\nFingerprint: {}\nBytes: {}\nModified: {}\nReport scope: {}\n\nPackage mapping:\n{}\n\nReport metadata:\n{}\n\nLimitations:\n{}",
        finding.identity.cve,
        security_cve_status_label(finding.status),
        finding.identity.recipe,
        finding.identity.package.as_deref().unwrap_or("unavailable"),
        finding.product.as_deref().unwrap_or("unavailable"),
        finding.version.as_deref().unwrap_or("unavailable"),
        finding.severity.as_deref().unwrap_or("unavailable"),
        finding.score.as_deref().unwrap_or("unavailable"),
        finding.vector.as_deref().unwrap_or("unavailable"),
        finding.advisory_url.as_deref().unwrap_or("unavailable"),
        finding.summary.as_deref().unwrap_or("unavailable"),
        report.identity.path.display(),
        report.identity.fingerprint,
        report.identity.byte_size,
        timestamp_text(report.identity.modified_at),
        security_scope_text(report.scope.as_ref()),
        mapping,
        metadata,
        limitations,
    )
}

pub(crate) fn security_sbom_inspector(app: &App) -> String {
    let Some(report) = app.security.selected_report() else {
        return "Select an exact SPDX, CycloneDX, or package-manifest document to inspect it."
            .into();
    };
    if let SecurityReport::CycloneDx(document) = report {
        return format!(
            "Exact artifact: {}\nFingerprint: {}\nBytes: {}\nModified: {}\nFormat: CycloneDX\nSpec version: {}\nSerial number: {}\nDocument version: {}\nComponents: {}\nDependencies: {}\n\nLimitations:\n{}",
            document.identity.path.display(),
            document.identity.fingerprint,
            document.identity.byte_size,
            timestamp_text(document.identity.modified_at),
            document.spec_version.as_deref().unwrap_or("unavailable"),
            document.serial_number.as_deref().unwrap_or("unavailable"),
            document
                .version
                .map_or_else(|| "unavailable".into(), |value| value.to_string()),
            document.components.len(),
            document
                .dependency_count
                .map_or_else(|| "unavailable".into(), |value| value.to_string()),
            if document.limitations.is_empty() {
                "none".into()
            } else {
                document.limitations.join("\n")
            },
        );
    }
    if let SecurityReport::PackageManifest(document) = report {
        return format!(
            "Exact artifact: {}\nFingerprint: {}\nBytes: {}\nModified: {}\nFormat: Yocto package manifest fallback\nComponents: {}\n\nThis fallback supplies package names and versions only. License, supplier, file, and relationship claims are unavailable.\n\nLimitations:\n{}",
            document.identity.path.display(),
            document.identity.fingerprint,
            document.identity.byte_size,
            timestamp_text(document.identity.modified_at),
            document.components.len(),
            if document.limitations.is_empty() {
                "none".into()
            } else {
                document.limitations.join("\n")
            },
        );
    }
    let SecurityReport::Spdx(document) = report else {
        return "Select an SBOM document.".into();
    };
    let creators = if document.creators.is_empty() {
        "unavailable".into()
    } else {
        document.creators.join("\n")
    };
    let checksums = display_security_metadata(&document.checksums);
    let limitations = if document.limitations.is_empty() {
        "none".into()
    } else {
        document.limitations.join("\n")
    };
    let component = app
        .security
        .visible_components()
        .into_iter()
        .find(|component| {
            app.security.component_selection.as_deref() == Some(component.identity.as_str())
        })
        .map_or_else(
            || "No component selected.".into(),
            |component| {
                format!(
                    "Component: {}\nName: {}\nVersion: {}\nSupplier: {}\nLicense: {}",
                    component.identity,
                    component.name,
                    component.version.as_deref().unwrap_or("unavailable"),
                    component.supplier.as_deref().unwrap_or("unavailable"),
                    component.license.as_deref().unwrap_or("unavailable"),
                )
            },
        );
    format!(
        "Exact artifact: {}\nFingerprint: {}\nBytes: {}\nModified: {}\nKind: {}\nScope: {}\nSPDX version: {}\nDocument: {}\nNamespace: {}\nData license: {}\nCreators:\n{}\nComponents: {}\nFiles: {}\nRelationships: {}\nChecksums:\n{}\n\n{}\n\nLimitations:\n{}",
        document.identity.path.display(),
        document.identity.fingerprint,
        document.identity.byte_size,
        timestamp_text(document.identity.modified_at),
        security_spdx_kind_label(document.kind),
        security_scope_text(document.scope.as_ref()),
        document.spdx_version.as_deref().unwrap_or("unavailable"),
        document.name.as_deref().unwrap_or("unavailable"),
        document.namespace.as_deref().unwrap_or("unavailable"),
        document.data_license.as_deref().unwrap_or("unavailable"),
        creators,
        document.components.len(),
        document
            .file_count
            .map_or_else(|| "unavailable".into(), |value| value.to_string()),
        document
            .relationship_count
            .map_or_else(|| "unavailable".into(), |value| value.to_string()),
        checksums,
        component,
        limitations,
    )
}

pub(crate) fn selected_security_finding(
    app: &App,
) -> Option<(&yoctui_model::CveReport, &yoctui_model::CveFinding)> {
    let identity = app.security.finding_selection.as_ref()?;
    app.security
        .inventory
        .reports()?
        .iter()
        .find_map(|report| match report {
            SecurityReport::Cve(report) => report
                .findings
                .iter()
                .find(|finding| &finding.identity == identity)
                .map(|finding| (report, finding)),
            SecurityReport::Spdx(_)
            | SecurityReport::CycloneDx(_)
            | SecurityReport::PackageManifest(_) => None,
        })
}

pub(crate) fn cve_source_for_finding<'a>(
    app: &'a App,
    identity: &yoctui_model::CveFindingIdentity,
) -> Option<&'a yoctui_model::CveReport> {
    app.security
        .inventory
        .reports()?
        .iter()
        .find_map(|report| match report {
            SecurityReport::Cve(report)
                if report
                    .findings
                    .iter()
                    .any(|finding| &finding.identity == identity) =>
            {
                Some(report)
            }
            _ => None,
        })
}

pub(crate) fn display_security_metadata(values: &[yoctui_model::SecurityMetadata]) -> String {
    if values.is_empty() {
        "unavailable".into()
    } else {
        values
            .iter()
            .map(|value| format!("{}={}", value.key, value.value))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

pub(crate) fn display_security_paths(paths: &[std::path::PathBuf]) -> String {
    if paths.is_empty() {
        "unavailable".into()
    } else {
        paths
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    }
}
