use super::*;

#[test]
fn cache_summary_rejects_inconsistent_and_overflowing_counts() {
    let summary = SstateSummary {
        wanted: 10,
        local: 3,
        mirrors: 2,
        missed: 5,
        current: 8,
    };
    assert_eq!(summary.match_percent(), Some(50));
    assert!(
        !SstateSummary {
            missed: 6,
            ..summary
        }
        .valid()
    );
    assert!(
        !SstateSummary {
            local: u64::MAX,
            ..summary
        }
        .valid()
    );
    assert_eq!(
        SstateSummary {
            wanted: 0,
            local: 0,
            mirrors: 0,
            missed: 0,
            current: 8
        }
        .match_percent(),
        None
    );
}
