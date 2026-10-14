//! Original signed PCM effects, Jaguar DSP pitch, and bounded non-spatial playback.

use bevy::{
    audio::{AddAudioSource, Source, Volume},
    prelude::*,
};
use std::time::Duration;

const DSP_RATE: f64 = 26_590_906. / (64. * 28.);
const MAX_VOICES: usize = 16;

fn playback_rate(pitch: u16) -> u32 {
    // moomoo.dat PLAYFX2 at $4440 scales pitch by 28/43. The DSP adds
    // that increment to an 8:8 sample position per SSI tick, with SCLK = 27.
    rate_for_increment(u32::from(pitch) * 28 / 43)
}

fn rate_for_increment(increment: u32) -> u32 {
    (DSP_RATE * f64::from(increment.max(1)) / 256.).round() as u32
}

fn playback_speed(base: u16, pitch: u16) -> f32 {
    playback_rate(pitch) as f32 / playback_rate(base) as f32
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub(crate) enum Effect {
    Shot = 5,
    Jump = 6,
    Crackle = 7,
    Reward = 8,
    Warp = 9,
    LargeExplosion = 10,
    Laser = 11,
    Spike = 13,
    Explosion = 16,
    Pulse = 19,
    Shield = 20,
    Excellent = 21,
    Recharge = 22,
    Yes = 23,
    OneUp = 24,
    Scream = 25,
    YesOne = 26,
    YesTwo = 27,
    Tink = 28,
}

impl Effect {
    fn voice(self) -> bool {
        matches!(
            self,
            Self::Excellent
                | Self::Recharge
                | Self::Yes
                | Self::OneUp
                | Self::Scream
                | Self::YesOne
                | Self::YesTwo
        )
    }

    fn priority(self) -> u16 {
        match self {
            Self::Shot | Self::Laser => 1,
            Self::Explosion | Self::Spike | Self::Tink => 2,
            Self::LargeExplosion | Self::Crackle => 50,
            Self::Excellent
            | Self::Recharge
            | Self::Scream
            | Self::Yes
            | Self::YesOne
            | Self::YesTwo
            | Self::OneUp => 101,
            _ => 3,
        }
    }

    /// The laser sample is both far louder than the plain shot and long enough
    /// to overlap the next two shots, so holding the trigger stacks four copies
    /// of it. It needs a much lower gain than the 121 ms click it replaces.
    fn gain(self) -> f32 {
        match self {
            Self::Laser => 0.2,
            _ => 1.0,
        }
    }
}

#[derive(Clone, Copy)]
struct Request {
    effect: Effect,
    period: Option<u16>,
    exit_yes: bool,
}

#[derive(Resource, Default)]
pub(crate) struct Sounds {
    pending: Vec<Request>,
    generation: u32,
    random: u32,
    exit_yes: bool,
}

pub(crate) fn play(commands: &mut Commands, effect: Effect) {
    request(commands, effect, None, false);
}

pub(crate) fn pitched(commands: &mut Commands, effect: Effect, period: u16) {
    request(commands, effect, Some(period), false);
}

fn request(commands: &mut Commands, effect: Effect, period: Option<u16>, exit_yes: bool) {
    commands.queue(move |world: &mut World| {
        if let Some(mut sounds) = world.get_resource_mut::<Sounds>()
            && sounds.pending.len() < 128
        {
            sounds.pending.push(Request {
                effect,
                period,
                exit_yes,
            });
        }
    });
}

pub(crate) fn reset(commands: &mut Commands) {
    commands.queue(|world: &mut World| {
        if let Some(mut sounds) = world.get_resource_mut::<Sounds>() {
            sounds.pending.clear();
            sounds.generation = sounds.generation.wrapping_add(1);
            sounds.exit_yes = false;
        }
    });
}

#[derive(Asset, TypePath, Clone)]
pub(crate) struct Sample {
    data: &'static [u8],
    period: u16,
    repeat: usize,
    repeat_length: usize,
    looping: bool,
}

pub(crate) struct Decoder {
    sample: Sample,
    position: usize,
}

impl Iterator for Decoder {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.sample.looping && self.position == self.sample.repeat + self.sample.repeat_length {
            self.position = self.sample.repeat;
        }
        let value = *self.sample.data.get(self.position)? as i8;
        self.position += 1;
        Some(f32::from(value) / 128.)
    }
}

impl Source for Decoder {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> u16 {
        1
    }
    fn sample_rate(&self) -> u32 {
        playback_rate(self.sample.period)
    }
    fn total_duration(&self) -> Option<Duration> {
        (!self.sample.looping).then(|| {
            Duration::from_secs_f64(self.sample.data.len() as f64 / f64::from(self.sample_rate()))
        })
    }
}

impl Decodable for Sample {
    type DecoderItem = f32;
    type Decoder = Decoder;
    fn decoder(&self) -> Decoder {
        Decoder {
            sample: self.clone(),
            position: 0,
        }
    }
}

fn samples() -> [Sample; 30] {
    let bytes = include_bytes!("../assets/sounds.bin");
    assert_eq!(&bytes[..8], b"T32S\x1e\0\0\0");
    let mut offset = 8;
    std::array::from_fn(|_| {
        let period = u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap());
        let fields: [usize; 3] = std::array::from_fn(|i| {
            let start = offset + 2 + i * 4;
            u32::from_le_bytes(bytes[start..start + 4].try_into().unwrap()) as usize
        });
        offset += 14;
        let data = &bytes[offset..offset + fields[0]];
        offset += fields[0];
        assert!(period > 0 && fields[1] + fields[2] <= data.len());
        Sample {
            data,
            period,
            repeat: fields[1],
            repeat_length: fields[2],
            looping: false,
        }
    })
}

#[derive(Resource)]
pub(crate) struct EffectsPitch(pub(crate) f32);

impl Default for EffectsPitch {
    fn default() -> Self {
        Self(2.0)
    }
}

#[derive(Resource)]
struct Catalog {
    shots: [Handle<Sample>; 30],
    engines: [Handle<Sample>; 2],
    yes: Handle<Sample>,
    samples: [Sample; 30],
}

#[derive(Component)]
struct Voice {
    priority: u16,
    generation: u32,
    deadline: f64,
}

#[derive(Component)]
struct Engine(usize);

#[derive(Component)]
struct PitchRise {
    generation: u32,
    increment: u32,
    rate: u32,
    started: f64,
    exit_tick: Option<u16>,
}

pub(crate) fn add_systems(app: &mut App) {
    app.add_audio_source::<Sample>()
        .init_resource::<Sounds>()
        .init_resource::<EffectsPitch>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                update_yes_pitch
                    .after(crate::game::load_level)
                    .before(update),
                update.after(crate::game::load_level),
            )
                .run_if(crate::pause::running),
        );
}

fn setup(mut commands: Commands, mut assets: ResMut<Assets<Sample>>) {
    let samples = samples();
    let shots = samples.each_ref().map(|sample| assets.add(sample.clone()));
    let engines = [0, 2].map(|index| {
        let mut sample = samples[index].clone();
        sample.looping = true;
        assets.add(sample)
    });
    let mut yes = samples[Effect::Yes as usize].clone();
    yes.looping = true;
    commands.insert_resource(Catalog {
        shots,
        engines,
        yes: assets.add(yes),
        samples,
    });
}

fn engine_period(game: &crate::game::Game) -> Option<u16> {
    game.exit
        .filter(|exit| exit.flash == 0)
        .map(|exit| 10 + exit.ticks)
}

fn update_yes_pitch(
    mut commands: Commands,
    sounds: Res<Sounds>,
    time: Res<Time>,
    game: Res<crate::game::Game>,
    preview: Option<Res<crate::messages::Preview>>,
    volume: Res<GlobalVolume>,
    mut voices: Query<(
        Entity,
        &PitchRise,
        &mut PlaybackSettings,
        Option<&AudioSink>,
    )>,
) {
    for (entity, rise, mut settings, sink) in &mut voices {
        if rise.generation != sounds.generation || volume.volume == Volume::Linear(0.) {
            commands.entity(entity).despawn();
            continue;
        }
        let ticks = match (rise.exit_tick, game.exit) {
            (Some(start), Some(exit)) if exit.flash == 0 => {
                u32::from(exit.ticks.saturating_sub(start))
            }
            (None, _)
                if preview
                    .as_ref()
                    .is_some_and(|preview| preview.id == crate::messages::Id::YesYesYes) =>
            {
                ((time.elapsed_secs_f64() - rise.started) * 60.).round() as u32
            }
            _ => {
                commands.entity(entity).despawn();
                continue;
            }
        };
        // CHANGEFX adds directly to the DSP increment after PLAYFX2's 28/43 conversion.
        let speed = rate_for_increment(rise.increment + ticks) as f32 / rise.rate as f32;
        settings.speed = speed;
        if let Some(sink) = sink {
            sink.set_speed(speed);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn update(
    mut commands: Commands,
    mut sounds: ResMut<Sounds>,
    catalog: Res<Catalog>,
    time: Res<Time>,
    game: Res<crate::game::Game>,
    volume: Res<GlobalVolume>,
    effects_pitch: Res<EffectsPitch>,
    voices: Query<(Entity, &Voice)>,
    yes: Query<Entity, With<PitchRise>>,
    mut engines: Query<(Entity, &Engine, &mut PlaybackSettings, Option<&AudioSink>)>,
) {
    let now = time.elapsed_secs_f64();
    let mut active = Vec::new();
    for (entity, voice) in &voices {
        if voice.generation != sounds.generation || voice.deadline <= now {
            commands.entity(entity).despawn();
        } else {
            active.push((entity, voice.priority));
        }
    }
    if engine_period(&game).is_none() {
        sounds.exit_yes = false;
    } else if sounds.pending.iter().any(|request| request.exit_yes) {
        sounds.exit_yes = true;
    }
    let period =
        engine_period(&game).filter(|_| !sounds.exit_yes && volume.volume != Volume::Linear(0.));
    let mut running = [false; 2];
    for (entity, engine, mut settings, sink) in &mut engines {
        if let Some(period) = period {
            running[engine.0] = true;
            let speed = playback_speed(428, period * (engine.0 + 1) as u16) * effects_pitch.0;
            settings.speed = speed;
            if let Some(sink) = sink {
                sink.set_speed(speed);
            }
        } else {
            commands.entity(entity).despawn();
        }
    }
    if let Some(period) = period {
        for (index, running) in running.into_iter().enumerate() {
            if !running {
                commands.spawn((
                    Engine(index),
                    AudioPlayer(catalog.engines[index].clone()),
                    PlaybackSettings::ONCE
                        .with_volume(Volume::Linear(0.18))
                        .with_speed(
                            playback_speed(428, period * (index + 1) as u16) * effects_pitch.0,
                        ),
                ));
            }
        }
    }
    let pending = std::mem::take(&mut sounds.pending);
    let mut yes_voice = yes.iter().next();
    for request in pending {
        if volume.volume == Volume::Linear(0.) {
            continue;
        }
        let priority = request.effect.priority();
        if !request.exit_yes && active.len() == MAX_VOICES {
            let (index, &(_, lowest)) = active.iter().enumerate().min_by_key(|(_, v)| v.1).unwrap();
            if priority < lowest {
                continue;
            }
            commands.entity(active.remove(index).0).despawn();
        }
        let index = request.effect as usize;
        let sample = &catalog.samples[index];
        let period = request
            .period
            .unwrap_or_else(|| {
                if request.effect == Effect::Explosion {
                    sounds.random = sounds.random.wrapping_mul(1664525).wrapping_add(1013904223);
                    128 + ((sounds.random >> 16) & 255) as u16
                } else {
                    sample.period
                }
            })
            .max(1);
        let scale = if request.effect.voice() {
            1.0
        } else {
            effects_pitch.0
        };
        let speed = playback_speed(sample.period, period) * scale;
        let (audio, settings) = if request.exit_yes {
            if let Some(entity) = yes_voice.take() {
                commands.entity(entity).despawn();
            }
            (catalog.yes.clone(), PlaybackSettings::ONCE)
        } else {
            (catalog.shots[index].clone(), PlaybackSettings::DESPAWN)
        };
        let entity = commands
            .spawn((
                AudioPlayer(audio),
                settings.with_speed(speed).with_volume(Volume::Linear(
                    if priority >= 100 { 0.7 } else { 0.4 } * request.effect.gain(),
                )),
            ))
            .id();
        if request.exit_yes {
            commands.entity(entity).insert(PitchRise {
                generation: sounds.generation,
                increment: u32::from(period) * 28 / 43,
                rate: playback_rate(catalog.samples[index].period),
                started: now,
                exit_tick: game.exit.map(|exit| exit.ticks),
            });
            yes_voice = Some(entity);
        } else {
            commands.entity(entity).insert(Voice {
                priority,
                generation: sounds.generation,
                deadline: now
                    + sample.data.len() as f64
                        / (f64::from(playback_rate(period)) * f64::from(scale))
                    + 0.25,
            });
            active.push((entity, priority));
        }
    }
}

pub(crate) fn announcement(commands: &mut Commands, id: crate::messages::Id) {
    use crate::messages::Id;
    let effect = match id {
        Id::SuperzapperRecharge => Effect::Recharge,
        Id::Excellent => Effect::Excellent,
        Id::OneUp => Effect::OneUp,
        Id::CaughtYou => Effect::Scream,
        Id::ShotYou | Id::Spiked => {
            pitched(commands, Effect::LargeExplosion, 200);
            pitched(commands, Effect::LargeExplosion, 196);
            return;
        }
        Id::FriedYou => {
            pitched(commands, Effect::Crackle, 500);
            return;
        }
        Id::YesYesYes => {
            request(commands, Effect::Yes, None, true);
            return;
        }
        Id::WarpEnabled => Effect::Yes,
        Id::Points2000 | Id::Points250 | Id::Points500 | Id::Points750 => Effect::Reward,
        _ => return,
    };
    play(commands, effect);
}

#[cfg(test)]
#[path = "tests/sound.rs"]
mod tests;
