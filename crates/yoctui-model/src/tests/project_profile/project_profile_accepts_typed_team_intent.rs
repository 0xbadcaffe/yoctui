use super::*;

#[test]
fn project_profile_accepts_typed_team_intent() {
    assert_eq!(profile().validate(), Ok(()));
}
