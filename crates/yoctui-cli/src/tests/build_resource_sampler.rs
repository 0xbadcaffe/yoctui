use super::*;
#[test]
fn build_pressure_parsing_distinguishes_missing_from_zero_and_rejects_bad_values() {
    assert_eq!(
        parse_pressure("some avg10=0.00 avg60=0.00 total=10\n")
            .unwrap()
            .some_hundredths,
        0
    );
    assert_eq!(
        parse_pressure("some avg10=10.52 avg60=0.00 total=10\nfull avg10=1.25 total=2\n")
            .unwrap()
            .full_hundredths,
        Some(125)
    );
    for bad in [
        "",
        "full avg10=0.00",
        "some avg10=nan",
        "some avg10=100.01",
        "some avg10=-1.00",
    ] {
        assert!(parse_pressure(bad).is_none(), "{bad}");
    }
    assert_eq!(
        memory_value("Cached: 12 kB\nSwapTotal: 0 kB", "Cached:"),
        Some(12288)
    );
    assert_eq!(memory_value("Cached: 12 MB", "Cached:"), None);
}

#[test]
fn task_memory_samples_real_owned_processes_and_resets_between_builds() {
    let mut sampler = BuildResourceSampler::default();
    let roots = vec![TaskRoot {
        recipe: "owned-test".into(),
        task: "do_compile".into(),
        pid: std::process::id(),
    }];
    let sample = sampler.sample(Path::new("/tmp"), &roots, Some(1), None);
    assert!(sample.valid());
    assert_eq!(sample.recipe_peaks.len(), 1);
    assert!(sample.recipe_peaks[0].sampled_peak_rss_bytes > 0);
    assert!(sample.swap_in_bytes_per_second.is_none());
    let retained = sampler.sample(Path::new("/tmp"), &[], Some(1), None);
    assert_eq!(retained.recipe_peaks, sample.recipe_peaks);
    let reset = sampler.sample(Path::new("/tmp"), &[], Some(2), None);
    assert!(reset.recipe_peaks.is_empty());
}
#[test]
fn missing_task_pids_are_partial_and_never_invent_memory() {
    let mut budget = 1;
    assert_eq!(task_tree_rss(u32::MAX, &mut budget), (None, true));
    let mut budget = 0;
    assert_eq!(task_tree_rss(std::process::id(), &mut budget), (None, true));
}
