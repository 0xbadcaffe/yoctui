use super::*;

#[test]
fn compatibility_version_parser_compares_numeric_components_and_suffixes() {
    assert!(CorrelatedVersion::parse("1.52").unwrap() < CorrelatedVersion::parse("2.0").unwrap());
    assert_eq!(
        CorrelatedVersion::parse("2.8").unwrap(),
        CorrelatedVersion::parse("2.8.0").unwrap()
    );
    assert_eq!(
        CorrelatedVersion::parse("2.18.0+git").unwrap(),
        CorrelatedVersion::parse("2.18").unwrap()
    );
    for malformed in ["", ".2", "2.", "two", "2..1", "1.2.3.4.5"] {
        assert!(CorrelatedVersion::parse(malformed).is_err(), "{malformed}");
    }
}
