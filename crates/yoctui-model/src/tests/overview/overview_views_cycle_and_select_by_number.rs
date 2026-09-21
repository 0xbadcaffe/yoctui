use super::*;

#[test]
fn overview_views_cycle_and_select_by_number() {
    assert_eq!(OverviewView::Timeline.shifted(-1), OverviewView::DiskUsage);
    assert_eq!(
        OverviewView::from_number(7),
        Some(OverviewView::SupplyChain)
    );
    assert_eq!(OverviewView::from_number(9), None);
}
