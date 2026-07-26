use std::{env, path::PathBuf};

use yoctui_bitbake::SignatureAdapter;
use yoctui_model::{SignatureComparisonRequest, SignatureTarget};

fn required_path(name: &str) -> PathBuf {
    env::var_os(name)
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("{name} must name an absolute live-test path"))
}

#[tokio::test]
#[ignore = "requires an initialized writable Yocto build and real BitBake signature tools"]
async fn signature_adapter_live_smoke() {
    let build_dir = required_path("YOCTUI_LIVE_BUILD_DIR");
    let dumpsig = required_path("YOCTUI_LIVE_DUMPSIG");
    let diffsigs = required_path("YOCTUI_LIVE_DIFFSIGS");
    let left_target = SignatureTarget {
        recipe: env::var("YOCTUI_LIVE_SIGNATURE_RECIPE")
            .unwrap_or_else(|_| "autoconf-native".into()),
        task: env::var("YOCTUI_LIVE_SIGNATURE_TASK").unwrap_or_else(|_| "do_fetch".into()),
    };
    let right_target = SignatureTarget {
        recipe: env::var("YOCTUI_LIVE_SIGNATURE_COMPARE_RECIPE")
            .unwrap_or_else(|_| left_target.recipe.clone()),
        task: env::var("YOCTUI_LIVE_SIGNATURE_COMPARE_TASK")
            .unwrap_or_else(|_| "do_recipe_qa".into()),
    };
    let adapter = SignatureAdapter::with_programs(build_dir, dumpsig, diffsigs);
    let left = adapter.dump(left_target).await.unwrap();
    let right = adapter.dump(right_target).await.unwrap();
    assert!(
        !left.records.is_empty(),
        "the requested live signature dump returned no records"
    );
    assert!(
        !right.records.is_empty(),
        "the requested live comparison signature dump returned no records"
    );
    let request = SignatureComparisonRequest {
        left: left.records.last().unwrap().identity.clone(),
        right: right.records.last().unwrap().identity.clone(),
    };
    let comparison = adapter.compare(request).await.unwrap();
    eprintln!(
        "live signatures: left_records={} right_records={} differences={} dump_limitations={} comparison_limitations={}",
        left.records.len(),
        right.records.len(),
        comparison.differences.len(),
        left.limitations.len() + right.limitations.len(),
        comparison.limitations.len()
    );
}
