use super::*;

#[test]
fn raw_job_planner_reconstructs_exact_native_argv_and_rejects_tampering_before_spawn() {
    let fixture = Fixture::new("planner");
    let executable = fixture.executable("#!/bin/sh\nexit 0\n");
    let authority = fixture_authority(&fixture.0, &executable);
    let request = request(&authority);
    let command = RawJobPlanner::new(&authority)
        .plan(
            &request,
            RawJobId::new("raw-job:planner").unwrap(),
            RawStreamId::new("raw-stream:planner-out").unwrap(),
            RawStreamId::new("raw-stream:planner-err").unwrap(),
        )
        .unwrap();
    assert_eq!(command.executable(), executable);
    assert_eq!(command.current_directory(), fixture.0);
    assert_eq!(
        command.arguments().last().map(OsString::as_os_str),
        Some(std::ffi::OsStr::new("extra-target"))
    );
    assert!(!command.arguments().iter().any(|argument| argument == "sh"));

    let mut tampered = request;
    tampered.preview_digest.0[0] ^= 0xff;
    assert_eq!(
        RawJobPlanner::new(&authority).plan(
            &tampered,
            RawJobId::new("raw-job:tampered").unwrap(),
            RawStreamId::new("raw-stream:tampered-out").unwrap(),
            RawStreamId::new("raw-stream:tampered-err").unwrap(),
        ),
        Err(RawJobPlannerError::PreviewMismatch)
    );
}
