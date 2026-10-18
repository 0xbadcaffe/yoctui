use yoctui_model::{SetupFragment, valid_setup_fragment};

/// Upstream has no JSON list-fragments API. Only accept its explicit headings
/// and tab-separated records; unknown output fails closed, never invents state.
pub(super) fn parse(text: &str) -> Result<Vec<SetupFragment>, String> {
    let mut section = None;
    let mut recognized = false;
    let mut rows = Vec::new();
    for line in text.lines() {
        match line.trim() {
            "Available built-in fragments:" => {
                section = Some((false, true));
                recognized = true;
                continue;
            }
            "Enabled built-in fragments:" | "Enabled fragments:" => {
                section = Some((true, false));
                recognized = true;
                continue;
            }
            "Unused fragments:" => {
                section = Some((false, false));
                recognized = true;
                continue;
            }
            heading if heading.starts_with("Available fragments in ") => {
                section = None;
                recognized = true;
                continue;
            }
            _ => {}
        }
        let Some((enabled, pattern)) = section else {
            continue;
        };
        let Some((name, summary)) = line.split_once('\t') else {
            continue;
        };
        let valid = if pattern {
            name.strip_suffix("/...")
                .is_some_and(|prefix| valid_setup_fragment(&format!("{prefix}/value")))
        } else {
            valid_setup_fragment(name)
        };
        if !valid {
            return Err(
                "Unrecognized fragment identifier in upstream output; no changes were made.".into(),
            );
        }
        if rows.len() >= 2048 {
            return Err(
                "More than 2048 fragments; narrow the build's active layers before listing.".into(),
            );
        }
        rows.push(SetupFragment {
            name: name.into(),
            summary: summary.chars().take(1024).collect(),
            enabled,
            pattern,
        });
    }
    if !recognized {
        return Err("Unrecognized list-fragments output (or tool not supported by this build). No changes were made.".into());
    }
    Ok(rows)
}
