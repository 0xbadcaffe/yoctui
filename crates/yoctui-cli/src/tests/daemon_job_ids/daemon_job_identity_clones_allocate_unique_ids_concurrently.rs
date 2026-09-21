use super::*;

#[test]
fn daemon_job_identity_clones_allocate_unique_ids_concurrently() {
    let ids = DaemonJobIds::default();
    let threads = (0..8)
        .map(|_| {
            let ids = ids.clone();
            std::thread::spawn(move || {
                (0..32)
                    .map(|_| ids.allocate().unwrap().0)
                    .collect::<Vec<_>>()
            })
        })
        .collect::<Vec<_>>();
    let mut allocated = threads
        .into_iter()
        .flat_map(|thread| thread.join().unwrap())
        .collect::<Vec<_>>();
    allocated.sort_unstable();
    assert_eq!(allocated, (1..=256).collect::<Vec<_>>());
}
