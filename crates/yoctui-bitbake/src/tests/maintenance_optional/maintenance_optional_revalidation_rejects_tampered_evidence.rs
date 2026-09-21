use super::*;

#[test]
fn maintenance_optional_revalidation_rejects_tampered_evidence() {
    let fixture = TestDirectory::new("tampered");
    let inspection =
        MaintenanceOptionalCapabilityInspector::inspect(complete_fixture(&fixture)).unwrap();
    fs::write(fixture.join("report.json"), "{\"changed\":true}\n").unwrap();
    assert!(matches!(
        inspection.revalidate(),
        Err(MaintenanceOptionalAdapterError::StaleEvidence(path))
            if path == fixture.join("report.json")
    ));
}
