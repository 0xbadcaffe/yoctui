use super::*;

#[test]
fn environment_identity_deduplicates_exact_authoritative_records() {
    let mut identity = full_identity();
    let AuthoritativeValue::Detected { value, .. } = &mut identity.source_roots else {
        unreachable!();
    };
    value.push(value[0].clone());
    let normalized = identity.normalize().unwrap();
    assert_eq!(normalized.source_roots.value().unwrap().len(), 2);
}
