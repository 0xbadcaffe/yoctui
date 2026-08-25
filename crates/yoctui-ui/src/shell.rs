//! Workbench shell progress, utilization, and pacing projection.

use super::*;

pub(super) fn task_activity(app: &App, task_progress: Option<u8>) -> &'static str {
    if task_progress.is_some() {
        return "";
    }
    if app.reduced_motion {
        return " active";
    }
    const FAST: [&str; 8] = [
        "▸▸▸▸▸▸▸▸",
        "▹▸▸▸▸▸▸▸",
        "▹▹▸▸▸▸▸▸",
        "▹▹▹▸▸▸▸▸",
        "▹▹▹▹▸▸▸▸",
        "▹▹▹▹▹▸▸▸",
        "▹▹▹▹▹▹▸▸",
        "▹▹▹▹▹▹▹▸",
    ];
    FAST[(app.animation_frame as usize
        / if app.animation_speed == yoctui_model::AnimationSpeed::Slow {
            3
        } else {
            1
        })
        % FAST.len()]
}

pub(super) fn task_progress_bar(progress: u8) -> String {
    const WIDTH: usize = 10;
    const PARTIAL: [&str; 8] = ["", "▏", "▎", "▍", "▌", "▋", "▊", "▉"];
    let progress = progress.min(100);
    let eighths = usize::from(progress) * WIDTH * 8 / 100;
    let filled = eighths / 8;
    let partial = eighths % 8;
    let occupied = filled + usize::from(partial > 0);
    format!(
        "{}{}{} {progress}%",
        "█".repeat(filled),
        PARTIAL[partial],
        "░".repeat(WIDTH.saturating_sub(occupied))
    )
}

pub(super) fn utilization_percent(total: Option<u64>, available: Option<u64>) -> Option<u8> {
    let (total, available) = (total?, available?);
    if total == 0 || available > total {
        return None;
    }
    let used = total - available;
    u8::try_from((u128::from(used) * 100 / u128::from(total)).min(100)).ok()
}

pub(super) fn build_pace(app: &App) -> String {
    build_pace_at(app, SystemTime::now())
}

pub(super) fn build_pace_at(app: &App, now: SystemTime) -> String {
    let Some(elapsed) = app
        .build
        .started
        .and_then(|started| now.duration_since(started).ok())
        .filter(|elapsed| elapsed.as_secs() > 0)
    else {
        return "avg --/m · ETA --".into();
    };
    let Ok(completed) = u64::try_from(app.build.completed) else {
        return "avg --/m · ETA --".into();
    };
    if completed == 0 {
        return "avg --/m · ETA --".into();
    }
    let seconds = elapsed.as_secs();
    let rate_tenths = completed.saturating_mul(600) / seconds;
    let eta = app.build.total.and_then(|total| {
        let remaining = total.saturating_sub(app.build.completed);
        u64::try_from(remaining)
            .ok()
            .map(|remaining| Duration::from_secs(remaining.saturating_mul(seconds) / completed))
    });
    format!(
        "avg {}.{}/m · ETA {}",
        rate_tenths / 10,
        rate_tenths % 10,
        eta.map(format_duration).unwrap_or_else(|| "--".into())
    )
}
