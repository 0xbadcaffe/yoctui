use super::*;

#[tokio::test]
async fn clone_completion_and_failure_clear_pending_state() {
    for success in [true, false] {
        let mut app = App::new(100, 4096);
        let task = tokio::spawn(async move {
            if success {
                Ok(())
            } else {
                Err("fixture failed".into())
            }
        });
        let mut slot = Some(fixture(&app, task));
        activity(&mut app, true);
        while !slot.as_ref().unwrap().task.is_finished() {
            tokio::task::yield_now().await;
        }
        poll(&mut app, &mut slot).await;
        assert!(app.background_activities.is_empty());
        assert!(app.notification.as_ref().unwrap().contains(if success {
            "Poky cloned"
        } else {
            "fixture failed"
        }));
    }
}
