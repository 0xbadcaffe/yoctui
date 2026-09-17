//! Normalize the native sstate availability summary at the adapter boundary.
use yoctui_model::SstateSummary;

pub(crate) fn parse_sstate_summary(message: &str) -> Option<SstateSummary> {
    let message = message
        .trim()
        .strip_prefix("NOTE: ")
        .unwrap_or(message.trim());
    let mut words = message.strip_prefix("Sstate summary: ")?.split_whitespace();
    let mut count = |label| -> Option<u64> {
        if words.next()? != label {
            return None;
        }
        let value = words.next()?;
        if !value.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        value.parse().ok()
    };
    let summary = SstateSummary {
        wanted: count("Wanted")?,
        local: count("Local")?,
        mirrors: count("Mirrors")?,
        missed: count("Missed")?,
        current: count("Current")?,
    };
    summary.valid().then_some(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cache_bridge_only_promotes_native_unscoped_info() {
        for (level, recipe, expected) in [
            ("info", None, true),
            ("warning", None, false),
            ("info", Some("recipe"), false),
        ] {
            let event = crate::BridgeBackend::event(yoctui_protocol::Event::Log {
                level: level.into(),
                message: "Sstate summary: Wanted 10 Local 3 Mirrors 2 Missed 5 Current 8 (50% match, 72% complete)".into(),
                recipe: recipe.map(str::to_owned), task: None, path: None,
            }).unwrap();
            assert_eq!(
                matches!(event, crate::BackendEvent::SstateSummary(_)),
                expected
            );
        }
    }

    #[test]
    fn cache_native_summary_parsing_is_checked() {
        let line = "NOTE: Sstate summary: Wanted 10 Local 3 Mirrors 2 Missed 5 Current 8 (50% match, 72% complete)";
        assert_eq!(
            parse_sstate_summary(line).unwrap().match_percent(),
            Some(50)
        );
        for line in [
            line.replace("Missed 5", "Missed 6"),
            line.replace("Local 3", "Local -3"),
            line.replace("Local 3", "Local 18446744073709551616"),
            format!("recipe: {line}"),
        ] {
            assert_eq!(parse_sstate_summary(&line), None);
        }
    }
}
