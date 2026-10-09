//! Original track decoding, uninterrupted same-track playback, and transitions.

use super::*;

fn app() -> App {
    let mut app = App::new();
    app.init_resource::<Game>()
        .init_resource::<Level>()
        .init_resource::<Time>()
        .init_resource::<MusicVolume>()
        .init_resource::<Assets<Track>>()
        .insert_resource(GlobalVolume::new(Volume::Linear(0.6)))
        .insert_resource(crate::sound::EffectsPitch(4.0))
        .add_systems(Startup, setup)
        .add_systems(Update, update);
    app
}

fn current(app: &mut App) -> (Entity, usize) {
    let mut query = app.world_mut().query::<(Entity, &Playing)>();
    let (entity, playing) = query.single(app.world()).unwrap();
    (entity, playing.track)
}

#[test]
fn all_seven_original_modules_generate_stereo_music_at_normal_speed() {
    for index in 0..7 {
        assert_eq!(MODULES[index].get_num_channels(), 4);
        let mut decoder = Track(index).decoder();
        assert_eq!(decoder.channels(), 2);
        assert_eq!(decoder.sample_rate(), 44_100);
        assert_eq!(decoder.total_duration(), None);
        let audio: Vec<_> = decoder.by_ref().take(RATE as usize * 2).collect();
        assert_eq!(audio.len(), RATE as usize * 2);
        assert!(audio.iter().any(|sample| sample.unsigned_abs() > 1000));
        assert!(audio.chunks_exact(2).any(|pair| pair[0] != pair[1]));
    }
}

#[test]
fn decoder_continues_after_song_end() {
    let mut decoder = Track(4).decoder();
    decoder.0.set_max_loop_count(1);
    assert!(decoder.0.seek_seconds(decoder.0.duration_seconds() - 0.02));
    assert_eq!(
        decoder.by_ref().take(RATE as usize * 4).count(),
        RATE as usize * 4
    );
    // rave4's own pattern jump repeats from inside the song, not its intro.
    assert!(decoder.0.position_seconds() < decoder.0.duration_seconds() / 2.0);
}

#[test]
fn selection_matches_the_original_web_groups_and_bonus_tunes() {
    let mut game = Game::default();
    let mut level = Level::default();
    for (number, track) in [
        (1, 4),
        (16, 4),
        (17, 1),
        (32, 1),
        (33, 5),
        (48, 5),
        (49, 6),
        (64, 6),
        (65, 4),
        (80, 4),
        (81, 1),
        (96, 1),
        (97, 5),
        (99, 5),
    ] {
        level.number = number;
        assert_eq!(selected(&game, &level), track);
    }
    game.status = Status::Bonus;
    for (kind, track) in [
        (ZoneKind::GateFlight, 2),
        (ZoneKind::Tunnel, 3),
        (ZoneKind::Rings, 2),
    ] {
        game.bonus = Some(crate::warp::Session::new(kind, 0, None, 0));
        assert_eq!(selected(&game, &level), track);
    }
    game.status = Status::GameOver;
    assert_eq!(selected(&game, &level), 0);
}

#[test]
fn same_track_is_preserved_and_new_tracks_replace_it_after_a_fade() {
    let mut app = app();
    app.update();
    let original = current(&mut app);
    assert_eq!(original.1, 4);
    app.world_mut().resource_mut::<Level>().number = 2;
    for _ in 0..100 {
        app.update();
    }
    assert_eq!(current(&mut app), original);
    app.world_mut().resource_mut::<Level>().number = 17;
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(Duration::from_millis(500));
    app.update();
    assert_eq!(current(&mut app), original);
    let settings = app.world().get::<PlaybackSettings>(original.0).unwrap();
    assert_eq!(settings.speed, 1.0);
    assert_eq!(settings.volume, Volume::Linear(0.175));
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(Duration::from_millis(500));
    app.update();
    let next = current(&mut app);
    assert_eq!(next.1, 1);
    assert_ne!(next.0, original.0);
    assert!(app.world().get_entity(original.0).is_err());
    app.world_mut().resource_mut::<MusicVolume>().0 = 0.0;
    app.update();
    assert_eq!(
        app.world_mut()
            .query::<&Playing>()
            .iter(app.world())
            .count(),
        0
    );
}
