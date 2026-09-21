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
#[path = "tests/build_cache/mod.rs"]
mod tests;
