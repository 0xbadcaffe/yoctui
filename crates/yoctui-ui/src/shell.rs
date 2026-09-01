//! Workbench shell progress, utilization, and pacing projection.

use super::*;

pub(super) fn activity_symbol(
    projection: yoctui_model::ActivityProjection,
    unicode: bool,
) -> &'static str {
    let Some(phase) = projection.phase else {
        return match projection.lifecycle {
            yoctui_model::ActivityLifecycle::Succeeded => "+",
            yoctui_model::ActivityLifecycle::Failed => "x",
            yoctui_model::ActivityLifecycle::Cancelled => "#",
            _ => "",
        };
    };
    let symbols = if unicode {
        throbber_widgets_tui::WHITE_SQUARE.symbols
    } else {
        throbber_widgets_tui::ASCII.symbols
    };
    symbols[phase % symbols.len()]
}

pub(super) fn task_activity(app: &App, task_progress: Option<u8>) -> String {
    if task_progress.is_some() {
        return String::new();
    }
    let unicode = app.preferences.symbols == SymbolPreference::Unicode;
    let projection = app.activity_projection(yoctui_model::ActivityLifecycle::Running);
    if projection.phase.is_none() {
        return if unicode { "⊞".into() } else { "#".into() };
    }
    activity_symbol(projection, unicode).into()
}

pub(super) fn task_progress_bar(progress: u8) -> String {
    const WIDTH: usize = 10;
    let progress = progress.min(100);
    let filled = (usize::from(progress) * WIDTH).div_ceil(100);
    format!(
        "{}{} {progress}%",
        "▪".repeat(filled),
        "▫".repeat(WIDTH.saturating_sub(filled))
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
