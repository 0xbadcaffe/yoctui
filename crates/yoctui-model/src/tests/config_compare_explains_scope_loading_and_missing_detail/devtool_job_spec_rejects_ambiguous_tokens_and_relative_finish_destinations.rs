use super::*;

#[test]
fn devtool_job_spec_rejects_ambiguous_tokens_and_relative_finish_destinations() {
    for recipe in ["", "busy box", "busy\nbox", "--help"] {
        assert_eq!(
            DevtoolOperation::Modify {
                recipe: recipe.into(),
            }
            .validate(),
            Err(DevtoolOperationError::InvalidRecipe)
        );
    }
    for target in ["", "root@host /opt", "root@host\n--help", "--help"] {
        assert_eq!(
            DevtoolOperation::DeployTarget {
                recipe: "busybox".into(),
                target: target.into(),
            }
            .validate(),
            Err(DevtoolOperationError::InvalidTarget)
        );
    }
    assert_eq!(
        DevtoolOperation::Finish {
            recipe: "busybox".into(),
            destination: "meta-custom".into(),
        }
        .validate(),
        Err(DevtoolOperationError::RelativeFinishDestination)
    );
}
