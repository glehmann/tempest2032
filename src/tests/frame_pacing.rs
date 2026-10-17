//! Late-frame detection, monitor changes, and warning aggregation.

use super::FramePacing;
use std::time::Duration;

fn sample(
    pacing: &mut FramePacing,
    now_ms: u64,
    delta_ms: u64,
    rate: u32,
) -> Option<(u32, Duration)> {
    pacing.sample(
        Duration::from_millis(now_ms),
        Duration::from_millis(delta_ms),
        rate,
    )
}

#[test]
fn scheduling_jitter_is_tolerated_but_a_double_refresh_interval_warns() {
    let mut pacing = FramePacing::default();
    assert_eq!(sample(&mut pacing, 100, 100, 60_000), None);
    for ms in [16, 17, 19, 25] {
        assert_eq!(sample(&mut pacing, 200, ms, 60_000), None);
    }
    assert_eq!(
        sample(&mut pacing, 233, 33, 60_000),
        Some((1, Duration::from_millis(33)))
    );
}

#[test]
fn warnings_are_limited_to_once_a_second_without_losing_late_intervals() {
    let mut pacing = FramePacing::default();
    sample(&mut pacing, 0, 0, 60_000);
    assert_eq!(
        sample(&mut pacing, 33, 33, 60_000),
        Some((1, Duration::from_millis(33)))
    );
    assert_eq!(sample(&mut pacing, 66, 33, 60_000), None);
    assert_eq!(sample(&mut pacing, 166, 100, 60_000), None);
    assert_eq!(sample(&mut pacing, 1000, 16, 60_000), None);
    assert_eq!(
        sample(&mut pacing, 1033, 16, 60_000),
        Some((2, Duration::from_millis(100)))
    );
    assert_eq!(sample(&mut pacing, 2033, 16, 60_000), None);
}

#[test]
fn the_threshold_tracks_monitor_refresh_rate_and_resets_on_changes() {
    let mut pacing = FramePacing::default();
    sample(&mut pacing, 0, 0, 60_000);
    assert_eq!(sample(&mut pacing, 20, 20, 60_000), None);
    assert_eq!(sample(&mut pacing, 40, 20, 120_000), None);
    assert_eq!(
        sample(&mut pacing, 60, 20, 120_000),
        Some((1, Duration::from_millis(20)))
    );
    assert_eq!(sample(&mut pacing, 100, 40, 30_000), None);
    assert_eq!(sample(&mut pacing, 140, 40, 30_000), None);
}

#[test]
fn background_intervals_are_discarded_and_resuming_reestablishes_the_baseline() {
    let mut pacing = FramePacing::default();
    sample(&mut pacing, 0, 0, 60_000);
    sample(&mut pacing, 33, 33, 60_000);
    sample(&mut pacing, 66, 33, 60_000);
    // warn_late_frames resets the state while the window is in the background.
    pacing = FramePacing::default();
    assert_eq!(sample(&mut pacing, 10_100, 100, 60_000), None);
    assert_eq!(sample(&mut pacing, 10_116, 16, 60_000), None);
    assert_eq!(
        sample(&mut pacing, 10_150, 34, 60_000),
        Some((1, Duration::from_millis(34)))
    );
}
