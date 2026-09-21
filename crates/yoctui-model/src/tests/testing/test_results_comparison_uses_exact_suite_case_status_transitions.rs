use super::*;

#[test]
fn test_results_comparison_uses_exact_suite_case_status_transitions() {
    let baseline = result(
        "baseline",
        "base",
        vec![
            case("suite", "regression", TestCaseOutcome::Passed),
            case("suite", "fixed", TestCaseOutcome::Failed),
            case("suite", "removed", TestCaseOutcome::Skipped),
            case("suite", "same", TestCaseOutcome::Passed),
        ],
    );
    let candidate = result(
        "candidate",
        "candidate",
        vec![
            case("suite", "regression", TestCaseOutcome::Error),
            case("suite", "fixed", TestCaseOutcome::Passed),
            case("suite", "new-failure", TestCaseOutcome::Failed),
            case("suite", "same", TestCaseOutcome::Passed),
        ],
    );
    let comparison = TestComparison::between(&baseline, &candidate).unwrap();
    let categories = comparison
        .transitions
        .iter()
        .map(|transition| (transition.identity.case.as_str(), transition.category))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(categories["regression"], TestComparisonCategory::Regression);
    assert_eq!(categories["fixed"], TestComparisonCategory::NewPass);
    assert_eq!(
        categories["new-failure"],
        TestComparisonCategory::NewFailure
    );
    assert_eq!(categories["removed"], TestComparisonCategory::Removed);
    assert_eq!(categories["same"], TestComparisonCategory::UnchangedOther);
    assert!(TestComparison::between(&baseline, &baseline).is_err());
}
