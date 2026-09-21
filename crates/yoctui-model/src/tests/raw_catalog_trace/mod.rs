use super::*;
use crate::{
    RAW_BUILTIN_CATEGORY_COUNT, RAW_BUILTIN_COMMAND_COUNT, RAW_BUILTIN_EXECUTABLE_COUNT,
    RAW_REFERENCE_SHA256,
};
use std::collections::BTreeSet;

const REFERENCE: &str =
    include_str!("../../../../../docs/reference/bitbake-cheatsheet-wrynose-6.0-bitbake-2.18.md");

#[derive(Debug)]
struct ReferenceEntry<'a> {
    line: usize,
    category_heading: &'a str,
    heading: &'a str,
    description: &'a str,
    command: &'a str,
}

fn reference_entries() -> (Vec<&'static str>, Vec<ReferenceEntry<'static>>) {
    let mut categories = Vec::new();
    let mut entries = Vec::new();
    let mut category_heading = "";
    let mut heading = "";
    let mut description = "";
    let mut in_bash = false;

    for (index, line) in REFERENCE.lines().enumerate() {
        let line_number = index + 1;
        if line == "```bash" {
            in_bash = true;
            description = "";
            continue;
        }
        if in_bash && line == "```" {
            in_bash = false;
            description = "";
            continue;
        }
        if !in_bash && line.starts_with("# ") {
            if line_number != 1 {
                category_heading = &line[2..];
                heading = category_heading;
                categories.push(category_heading);
            }
            continue;
        }
        if !in_bash && line.starts_with("##") {
            heading = line.trim_start_matches('#').trim();
            continue;
        }
        if !in_bash {
            continue;
        }
        if let Some(comment) = line.strip_prefix("# ") {
            description = comment;
            continue;
        }
        if line.is_empty() {
            continue;
        }
        assert!(!category_heading.is_empty(), "line {line_number}");
        assert!(!description.is_empty(), "line {line_number}");
        entries.push(ReferenceEntry {
            line: line_number,
            category_heading,
            heading,
            description,
            command: line,
        });
        description = "";
    }
    (categories, entries)
}

fn direct_bitbake(command: &str) -> bool {
    command.starts_with("bitbake ")
        && ![" | ", " > ", " && ", " || ", "; "]
            .iter()
            .any(|operator| command.contains(operator))
}

mod raw_catalog_trace_covers_every_reference_command_exactly_once;

mod raw_catalog_trace_counts_classifications_and_unique_references;
