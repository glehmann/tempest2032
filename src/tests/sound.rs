//! Decode actual effects and check event timing, bounded playback, and loop cleanup headlessly.

use super::*;
use crate::{
    game::Game,
    test_support::{firing_app, ticks},
};

fn app() -> App {
    let mut app = App::new();
    app.init_resource::<Sounds>()
        .init_resource::<EffectsPitch>()
        .init_resource::<Assets<Sample>>()
        .init_resource::<Game>()
        .init_resource::<Time>()
        .insert_resource(GlobalVolume::new(Volume::Linear(0.6)))
        .add_systems(Startup, setup)
        .add_systems(Update, (update_yes_pitch, update).chain());
    app
}

#[test]
fn gameplay_pitch_scales_effects_and_cleanup_without_changing_voices() {
    for multiplier in [1.0, 1.5, 2.0] {
        let mut app = app();
        app.insert_resource(EffectsPitch(multiplier));
        app.world_mut().resource_mut::<Sounds>().pending.extend([
            Request {
                effect: Effect::Shot,
                period: None,
                repeats: 0,
            },
            Request {
                effect: Effect::Recharge,
                period: None,
                repeats: 0,
            },
        ]);
        app.update();
        let mut clips = app
            .world_mut()
            .query::<(&AudioPlayer<Sample>, &PlaybackSettings, &Voice)>();
        for (player, settings, voice) in clips.iter(app.world()) {
            let catalog = app.world().resource::<Catalog>();
            let is_shot = player.0 == catalog.shots[Effect::Shot as usize];
            assert_eq!(settings.speed, if is_shot { multiplier } else { 1.0 });
            let sample = &catalog.samples[if is_shot {
                Effect::Shot
            } else {
                Effect::Recharge
            } as usize];
            let expected = sample.decoder().total_duration().unwrap().as_secs_f64()
                / f64::from(settings.speed)
                + 0.25;
            assert!((voice.deadline - expected).abs() < 0.001);
        }
        assert_eq!(clips.iter(app.world()).count(), 2);
    }
}

#[test]
fn effect_rates_and_durations_match_the_jaguar_dsp_not_amiga_periods() {
    assert_eq!(samples()[5].decoder().sample_rate(), 16_114);
    assert_eq!(samples()[22].decoder().sample_rate(), 19_302);
    assert_eq!(samples()[7].decoder().sample_rate(), 8_057);
    let recharge = samples()[22]
        .decoder()
        .total_duration()
        .unwrap()
        .as_secs_f64();
    assert!((recharge - 2.246).abs() < 0.001);
    assert!(playback_speed(512, 568) > 1.0);
    assert!(playback_speed(512, 428) < 1.0);
    assert_eq!(playback_speed(512, 512), 1.0);
    assert!(playback_speed(428, 20) > playback_speed(428, 10));
}

#[test]
fn the_laser_is_gained_down_because_it_overlaps_itself_four_times() {
    assert!(Effect::Laser.gain() < Effect::Shot.gain());
    assert_eq!(Effect::Shot.gain(), 1.0);
}

#[test]
fn all_effects_decode_as_finite_signed_pcm_at_their_original_periods() {
    for (index, sample) in samples().into_iter().enumerate() {
        if sample.data.is_empty() {
            assert!([3, 4].contains(&index));
            continue;
        }
        let mut decoder = sample.decoder();
        assert!(decoder.sample_rate() >= 4_000);
        assert!(decoder.total_duration().unwrap() > Duration::ZERO);
        for &byte in sample.data {
            assert_eq!(decoder.next(), Some(f32::from(byte as i8) / 128.));
        }
        assert_eq!(decoder.next(), None);
    }
    assert_eq!(samples()[5].data.len(), 1956);
    assert_eq!(samples()[26].data.len(), 11348);
}

#[test]
fn engine_loops_play_the_intro_once_then_repeat_only_the_original_loop_region() {
    for index in [0, 2] {
        let mut sample = samples()[index].clone();
        sample.looping = true;
        let mut decoder = sample.decoder();
        assert_eq!(decoder.total_duration(), None);
        let end = sample.repeat + sample.repeat_length;
        let initial: Vec<_> = decoder.by_ref().take(end).collect();
        for _ in 0..3 {
            let repeat: Vec<_> = decoder.by_ref().take(sample.repeat_length).collect();
            assert_eq!(repeat, initial[sample.repeat..end]);
        }
    }
}

#[test]
fn yes_sequence_repeats_the_original_loop_twice_then_finishes() {
    let mut sample = samples()[Effect::Yes as usize].clone();
    sample.repeats = 2;
    let mut decoder = sample.decoder();
    let duration = decoder.total_duration().unwrap().as_secs_f64();
    assert!(duration > 3.5 && duration < 3.7);
    let initial: Vec<_> = decoder.by_ref().take(sample.data.len()).collect();
    assert_eq!(initial.len(), sample.data.len());
    for _ in 0..2 {
        let repeat: Vec<_> = decoder.by_ref().take(sample.repeat_length).collect();
        assert_eq!(
            repeat,
            initial[sample.repeat..sample.repeat + sample.repeat_length]
        );
    }
    assert_eq!(decoder.next(), None);
}

#[test]
fn yes_yes_yes_playback_keeps_all_three_repetitions_and_rises_in_previews() {
    for (id, repeats) in [
        (crate::messages::Id::YesYesYes, 2),
        (crate::messages::Id::WarpEnabled, 0),
    ] {
        let mut app = app();
        app.insert_resource(EffectsPitch(3.));
        announcement(&mut app.world_mut().commands(), id);
        app.world_mut().flush();
        app.update();
        let (player, settings, voice) = app
            .world_mut()
            .query::<(&AudioPlayer<Sample>, &PlaybackSettings, &Voice)>()
            .single(app.world())
            .unwrap();
        let sample = app
            .world()
            .resource::<Assets<Sample>>()
            .get(&player.0)
            .unwrap();
        assert_eq!(sample.repeats, repeats);
        assert_eq!(settings.speed, 1.);
        let duration = sample.decoder().total_duration().unwrap().as_secs_f64();
        assert!((voice.deadline - duration - 0.25).abs() < 0.001);
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f64(1.5));
        app.update();
        assert_eq!(
            app.world_mut().query::<&Voice>().iter(app.world()).count(),
            usize::from(repeats > 0)
        );
        if repeats > 0 {
            let settings = app
                .world_mut()
                .query_filtered::<&PlaybackSettings, With<Voice>>()
                .single(app.world())
                .unwrap();
            assert_eq!(
                settings.speed,
                rate_for_increment(333 + 90) as f32 / playback_rate(512) as f32
            );
        }
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs(3));
        app.update();
        assert_eq!(
            app.world_mut().query::<&Voice>().iter(app.world()).count(),
            0
        );
    }
}

#[test]
fn exit_yes_pitch_adds_one_dsp_increment_per_simulation_tick() {
    let mut app = app();
    app.insert_resource(EffectsPitch(3.));
    let level = crate::level::Level::default();
    let mut exit = crate::animations::level_exit::Exit::new(
        level.web.glider_transform(level.web.start()).translation,
        &level.web,
    );
    exit.ticks = 73;
    app.world_mut().resource_mut::<Game>().exit = Some(exit);
    announcement(
        &mut app.world_mut().commands(),
        crate::messages::Id::YesYesYes,
    );
    app.world_mut().flush();
    app.update();
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(Duration::from_secs(1));
    app.update();
    let mut settings = app
        .world_mut()
        .query_filtered::<&PlaybackSettings, With<Voice>>();
    assert_eq!(settings.single(app.world()).unwrap().speed, 1.);
    app.world_mut()
        .resource_mut::<Game>()
        .exit
        .as_mut()
        .unwrap()
        .ticks += 60;
    app.update();
    let speed = rate_for_increment(333 + 60) as f32 / playback_rate(512) as f32;
    assert_eq!(settings.single(app.world()).unwrap().speed, speed);
    assert!(speed > 1.17 && speed < 1.19);
    for _ in 0..100 {
        app.update();
    }
    assert_eq!(settings.single(app.world()).unwrap().speed, speed);
    app.world_mut()
        .resource_mut::<Game>()
        .exit
        .as_mut()
        .unwrap()
        .ticks += 1;
    app.update();
    assert_eq!(
        settings.single(app.world()).unwrap().speed,
        rate_for_increment(394) as f32 / playback_rate(512) as f32
    );
}

#[test]
fn shots_emit_only_when_a_bullet_is_actually_fired() {
    let mut app = firing_app(0, &[KeyCode::Space]);
    app.init_resource::<Sounds>();
    ticks(&mut app, 1);
    assert_eq!(app.world().resource::<Sounds>().pending.len(), 1);
    assert_eq!(
        app.world().resource::<Sounds>().pending[0].effect,
        Effect::Shot
    );
    ticks(&mut app, 2);
    assert_eq!(app.world().resource::<Sounds>().pending.len(), 1);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::Space);
    ticks(&mut app, 30);
    assert_eq!(app.world().resource::<Sounds>().pending.len(), 1);
}

#[test]
fn sustained_fire_is_bounded_and_high_priority_voices_replace_shots() {
    let mut app = app();
    app.update();
    for tick in 0..100 {
        app.world_mut()
            .resource_mut::<Sounds>()
            .pending
            .extend((0..10).map(|_| Request {
                effect: Effect::Shot,
                period: None,
                repeats: 0,
            }));
        app.update();
        assert_eq!(
            app.world_mut().query::<&Voice>().iter(app.world()).count(),
            ((tick + 1) * 10).min(MAX_VOICES)
        );
    }
    app.world_mut()
        .resource_mut::<Sounds>()
        .pending
        .push(Request {
            effect: Effect::Recharge,
            period: None,
            repeats: 0,
        });
    app.update();
    assert!(
        app.world_mut()
            .query::<&Voice>()
            .iter(app.world())
            .any(|v| v.priority == 101)
    );
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(Duration::from_secs(20));
    app.update();
    assert_eq!(
        app.world_mut().query::<&Voice>().iter(app.world()).count(),
        0
    );
}

#[test]
fn exit_engines_are_reused_then_removed_when_exit_ends() {
    let mut app = app();
    let level = crate::level::Level::default();
    app.world_mut().resource_mut::<Game>().exit = Some(crate::animations::level_exit::Exit::new(
        level.web.glider_transform(level.web.start()).translation,
        &level.web,
    ));
    app.update();
    assert_eq!(
        app.world_mut().query::<&Engine>().iter(app.world()).count(),
        2
    );
    for _ in 0..100 {
        app.update();
    }
    assert_eq!(
        app.world_mut().query::<&Engine>().iter(app.world()).count(),
        2
    );
    app.world_mut().resource_mut::<Game>().exit = None;
    app.update();
    assert_eq!(
        app.world_mut().query::<&Engine>().iter(app.world()).count(),
        0
    );
}

#[test]
fn reset_stops_old_voices_and_muting_drops_new_requests() {
    let mut app = app();
    app.world_mut()
        .resource_mut::<Sounds>()
        .pending
        .push(Request {
            effect: Effect::Recharge,
            period: None,
            repeats: 0,
        });
    app.update();
    assert_eq!(
        app.world_mut().query::<&Voice>().iter(app.world()).count(),
        1
    );
    let mut commands = app.world_mut().commands();
    play(&mut commands, Effect::Shot);
    reset(&mut commands);
    app.world_mut().flush();
    assert!(app.world().resource::<Sounds>().pending.is_empty());
    app.update();
    assert_eq!(
        app.world_mut().query::<&Voice>().iter(app.world()).count(),
        0
    );
    app.insert_resource(GlobalVolume::new(Volume::Linear(0.)));
    app.world_mut()
        .resource_mut::<Sounds>()
        .pending
        .push(Request {
            effect: Effect::Shot,
            period: None,
            repeats: 0,
        });
    app.update();
    assert!(app.world().resource::<Sounds>().pending.is_empty());
    assert_eq!(
        app.world_mut().query::<&Voice>().iter(app.world()).count(),
        0
    );
}
