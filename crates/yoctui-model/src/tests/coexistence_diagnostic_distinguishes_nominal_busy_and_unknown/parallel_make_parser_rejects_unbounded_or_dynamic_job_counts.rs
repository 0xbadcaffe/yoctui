use super::*;

#[test]
fn parallel_make_parser_rejects_unbounded_or_dynamic_job_counts() {
    assert_eq!(parse_parallel_make_jobs("-j12"), Some(12));
    assert_eq!(parse_parallel_make_jobs("--jobs 6"), Some(6));
    assert_eq!(parse_parallel_make_jobs("-j"), None);
    assert_eq!(
        parse_parallel_make_jobs("-j ${@oe.utils.cpu_count()}"),
        None
    );
}
