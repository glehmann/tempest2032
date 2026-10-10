//! Embedded original MOD tracks, streamed in stereo independently of effect pitch.

use crate::{
    game::{Game, Status},
    level::Level,
    warp::ZoneKind,
};
use bevy::{
    audio::{AddAudioSource, Source, Volume},
    prelude::*,
};
use std::{sync::LazyLock, time::Duration};
use xmrs::prelude::Module;
use xmrsplayer::prelude::XmrsPlayer;

const RATE: u32 = 44_100;
const DATA: [&[u8]; 7] = [
    include_bytes!("../assets/music/tune13.mod"),
    include_bytes!("../assets/music/tune7.mod"),
    include_bytes!("../assets/music/tune1.mod"),
    include_bytes!("../assets/music/tune3.mod"),
    include_bytes!("../assets/music/rave4.mod"),
    include_bytes!("../assets/music/tune5.mod"),
    include_bytes!("../assets/music/tune12.mod"),
];
static MODULES: LazyLock<[Module; 7]> = LazyLock::new(|| {
    DATA.map(|data| Module::load_mod(data).expect("invalid embedded music module"))
});

#[derive(Resource)]
pub(crate) struct MusicVolume(pub(crate) f32);

impl Default for MusicVolume {
    fn default() -> Self {
        Self(0.35)
    }
}

#[derive(Asset, TypePath)]
pub(crate) struct Track(usize);

pub(crate) struct Decoder(XmrsPlayer<'static>);

impl Iterator for Decoder {
    type Item = i16;

    fn next(&mut self) -> Option<i16> {
        self.0.next().or_else(|| {
            self.0.goto(0, 0, 0);
            self.0.next()
        })
    }
}

impl Source for Decoder {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> u16 {
        2
    }
    fn sample_rate(&self) -> u32 {
        RATE
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

impl Decodable for Track {
    type DecoderItem = i16;
    type Decoder = Decoder;
    fn decoder(&self) -> Decoder {
        Decoder(XmrsPlayer::new(&MODULES[self.0], RATE, 0))
    }
}

#[derive(Resource)]
struct Catalog([Handle<Track>; 7]);

#[derive(Component)]
struct Playing {
    track: usize,
    fade: f32,
}

fn selected(game: &Game, level: &Level) -> usize {
    if game.status == Status::GameOver {
        return 0;
    }
    if let Some(bonus) = &game.bonus {
        return if bonus.kind == ZoneKind::Tunnel { 3 } else { 2 };
    }
    // yak.s webtunes uses zero-based cwave >> 4 and one-based module numbers.
    [4, 1, 5, 6, 4, 1, 5, 6][(level.number.saturating_sub(1) / 16) & 7]
}

pub(crate) fn add_systems(app: &mut App) {
    app.add_audio_source::<Track>()
        .init_resource::<MusicVolume>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            update
                .after(crate::game::load_level)
                .run_if(crate::pause::running),
        );
}

fn setup(mut commands: Commands, mut assets: ResMut<Assets<Track>>) {
    LazyLock::force(&MODULES);
    commands.insert_resource(Catalog(std::array::from_fn(|index| {
        assets.add(Track(index))
    })));
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn update(
    mut commands: Commands,
    game: Res<Game>,
    level: Res<Level>,
    catalog: Res<Catalog>,
    time: Res<Time>,
    volume: Res<MusicVolume>,
    master: Res<GlobalVolume>,
    mut playing: Query<(
        Entity,
        &mut Playing,
        &mut PlaybackSettings,
        Option<&mut AudioSink>,
    )>,
) {
    if volume.0 == 0. || master.volume.to_linear() == 0. {
        for (entity, _, _, _) in &mut playing {
            commands.entity(entity).despawn();
        }
        return;
    }
    let track = selected(&game, &level);
    for (entity, mut current, mut settings, sink) in &mut playing {
        current.fade = if current.track == track {
            0.
        } else {
            current.fade + time.delta_secs()
        };
        if current.fade < 1. {
            let gain = volume.0 * (1. - current.fade);
            settings.volume = Volume::Linear(gain);
            if let Some(mut sink) = sink {
                sink.set_volume(Volume::Linear(gain * master.volume.to_linear()));
            }
            return;
        }
        commands.entity(entity).despawn();
    }
    commands.spawn((
        Playing { track, fade: 0. },
        AudioPlayer(catalog.0[track].clone()),
        PlaybackSettings::ONCE.with_volume(Volume::Linear(volume.0)),
    ));
    info!("Playing original music track {}", track + 1);
}

#[cfg(test)]
#[path = "tests/music.rs"]
mod tests;
