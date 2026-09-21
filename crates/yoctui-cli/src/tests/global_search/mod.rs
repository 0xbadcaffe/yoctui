use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

fn fixture_root() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "yoctui-global-search-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

mod global_search_discloses_limits_and_honors_cancellation;

mod global_search_only_matches_build_text_contents_including_rootfs_and_artifacts;
