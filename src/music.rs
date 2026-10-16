//! Embedded MOD music and optional per-level files, independent of effect pitch.

use crate::{
    game::{Game, Status},
    level::Level,
    warp::ZoneKind,
};
use bevy::{
    audio::{AddAudioSource, Source, Volume},
    prelude::*,
};
use std::{collections::HashMap, path::PathBuf, sync::LazyLock, time::Duration};
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
struct Catalog {
    original: [Handle<Track>; 7],
    custom: HashMap<PathBuf, Handle<AudioSource>>,
}

#[derive(Clone, Debug, PartialEq)]
enum Selection {
    Original(usize),
    Custom(Handle<AudioSource>),
}

#[derive(Component)]
struct Playing {
    track: Selection,
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
        .init_resource::<Assets<AudioSource>>()
        .init_resource::<MusicVolume>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            update
                .after(crate::game::load_level)
                .run_if(crate::pause::running),
        );
}

fn setup(
    mut commands: Commands,
    mut assets: ResMut<Assets<Track>>,
    levels: Res<crate::level::Levels>,
    mut audio: ResMut<Assets<AudioSource>>,
) {
    LazyLock::force(&MODULES);
    commands.insert_resource(Catalog {
        original: std::array::from_fn(|index| assets.add(Track(index))),
        custom: custom_tracks(&levels, &mut audio),
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn load_soundtrack(path: &std::path::Path) -> Result<AudioSource, String> {
    let source = AudioSource {
        bytes: std::fs::read(path)
            .map_err(|error| error.to_string())?
            .into(),
    };
    let mut decoder = rodio::Decoder::new(std::io::Cursor::new(source.clone()))
        .map_err(|error| error.to_string())?;
    if decoder.next().is_none() {
        return Err("soundtrack contains no audio samples".into());
    }
    Ok(source)
}

fn custom_tracks(
    levels: &crate::level::Levels,
    audio: &mut Assets<AudioSource>,
) -> HashMap<PathBuf, Handle<AudioSource>> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut tracks = HashMap::new();
        let paths: std::collections::HashSet<_> = levels
            .0
            .iter()
            .filter_map(|level| level.soundtrack.as_ref())
            .collect();
        for path in paths {
            match load_soundtrack(path) {
                Ok(source) => {
                    tracks.insert(path.clone(), audio.add(source));
                }
                Err(error) => warn!(
                    "Could not load soundtrack {}: {error}; using original music",
                    path.display()
                ),
            }
        }
        tracks
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (levels, audio);
        HashMap::new()
    }
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
    front: Option<Res<crate::menu::FrontEnd>>,
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
    // rreset requests modnum=1 at yak.s:1018; modtable[0] is tune13.mod.
    let track = if front.is_some_and(|front| front.screen != crate::menu::Screen::Gameplay) {
        Selection::Original(0)
    } else if game.status != Status::GameOver
        && game.bonus.is_none()
        && let Some(handle) = level
            .soundtrack
            .as_ref()
            .and_then(|path| catalog.custom.get(path))
    {
        Selection::Custom(handle.clone())
    } else {
        Selection::Original(selected(&game, &level))
    };
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
    let mut entity = commands.spawn(Playing {
        track: track.clone(),
        fade: 0.,
    });
    match track {
        Selection::Original(index) => {
            entity.insert((
                AudioPlayer(catalog.original[index].clone()),
                PlaybackSettings::ONCE.with_volume(Volume::Linear(volume.0)),
            ));
            info!("Playing original music track {}", index + 1);
        }
        Selection::Custom(handle) => {
            entity.insert((
                AudioPlayer(handle),
                PlaybackSettings::LOOP.with_volume(Volume::Linear(volume.0)),
            ));
            info!("Playing custom level soundtrack");
        }
    }
}

#[cfg(test)]
#[path = "tests/music.rs"]
mod tests;
