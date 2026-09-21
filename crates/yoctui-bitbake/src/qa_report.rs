include!("qa_report/types_and_adapter.rs");

include!("qa_report/report_acquisition.rs");

include!("qa_report/json_parsing.rs");

include!("qa_report/text_and_xml_parsing.rs");

include!("qa_report/metadata_and_limits.rs");

#[cfg(test)]
#[path = "tests/qa_report/mod.rs"]
mod tests;
