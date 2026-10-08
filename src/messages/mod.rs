//! Event-driven native-resolution announcements and independent text effects.

mod catalog;
mod font;
mod render;

use bevy::prelude::*;
pub(crate) use catalog::Id;

#[derive(Clone, Copy, Debug)]
pub(crate) struct Event {
    pub(crate) id: Id,
}

impl From<Id> for Event {
    fn from(id: Id) -> Self {
        Self { id }
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Active {
    pub(super) event: Event,
    pub(super) age: u16,
}

impl Active {
    pub(super) fn scale(self) -> Vec2 {
        Vec2::new(
            1.,
            1. + f32::from(self.age.saturating_sub(self.event.id.hold() + 1)) * 0.5,
        )
    }

    pub(super) fn alive(self) -> bool {
        self.age < self.event.id.hold() + 52
    }
}

#[derive(Resource, Default)]
pub(crate) struct Messages {
    pub(super) banner: Option<Active>,
    pub(super) clock: u32,
    pub(crate) events: Vec<Event>,
    epoch: u32,
}

#[derive(Resource)]
pub(crate) struct Preview {
    pub(crate) id: Id,
    pub(crate) capture_tick: Option<u16>,
    pub(crate) elapsed: u16,
}

impl Messages {
    pub(crate) fn send(&mut self, id: Id) {
        self.events.push(id.into());
    }
    pub(super) fn step(&mut self) {
        self.clock = self.clock.wrapping_add(1);
        if let Some(active) = &mut self.banner {
            active.age += 1;
        }
        self.banner = self.banner.filter(|active| active.alive());
        for event in self.events.drain(..) {
            self.banner = Some(Active { event, age: 0 });
        }
    }
}

pub(crate) fn add_systems(app: &mut App) {
    app.init_resource::<Messages>()
        .init_resource::<font::Font>()
        .add_systems(Startup, render::setup)
        .add_systems(FixedUpdate, advance.after(crate::game::load_level))
        .add_systems(Update, render::draw.after(crate::warp::render::draw));
}

fn advance(
    mut messages: ResMut<Messages>,
    mut preview: Option<ResMut<Preview>>,
    mut game: ResMut<crate::game::Game>,
    mut upgrades: ResMut<crate::powerups::Upgrades>,
) {
    if messages.epoch != game.message_epoch {
        if let Some(preview) = &mut preview {
            preview.elapsed = 0;
        }
        *messages = Messages {
            epoch: game.message_epoch,
            ..default()
        };
    }
    if preview.is_none() {
        for id in upgrades.announcements.drain(..) {
            messages.send(id);
        }
        messages.events.append(&mut game.announcements);
    } else {
        upgrades.announcements.clear();
        game.announcements.clear();
    }
    if let Some(preview) = &mut preview {
        if preview
            .capture_tick
            .is_some_and(|tick| preview.elapsed > tick)
        {
            return;
        }
        if preview.elapsed == 0 {
            messages.send(preview.id);
        }
        preview.elapsed += 1;
        if preview.capture_tick.is_none() && preview.elapsed > preview.id.hold() + 90 {
            preview.elapsed = 0;
        }
    }
    messages.step();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn game_events_are_consumed_once_and_restart_clears_old_messages() {
        let mut app = App::new();
        app.init_resource::<Messages>()
            .init_resource::<crate::game::Game>()
            .init_resource::<crate::powerups::Upgrades>()
            .add_systems(Update, advance);
        app.world_mut()
            .resource_mut::<crate::game::Game>()
            .announce(Id::CaughtYou);
        app.update();
        assert_eq!(
            app.world().resource::<Messages>().banner.unwrap().event.id,
            Id::CaughtYou
        );
        assert!(
            app.world()
                .resource::<crate::game::Game>()
                .announcements
                .is_empty()
        );
        app.update();
        assert_eq!(app.world().resource::<Messages>().banner.unwrap().age, 1);
        app.world_mut()
            .resource_mut::<crate::game::Game>()
            .message_epoch += 1;
        app.update();
        assert!(app.world().resource::<Messages>().banner.is_none());
    }
    #[test]
    fn announcements_hold_then_stretch_with_original_inclusive_expiry() {
        let mut messages = Messages::default();
        messages.send(Id::ParticleLaser);
        messages.step();
        for _ in 0..101 {
            messages.step();
        }
        assert_eq!(messages.banner.unwrap().scale(), Vec2::ONE);
        messages.step();
        assert_eq!(messages.banner.unwrap().scale(), Vec2::new(1., 1.5));
        for _ in 0..50 {
            messages.step();
        }
        assert!(messages.banner.is_none());
        messages.send(Id::AvoidSpikes);
        messages.step();
        for _ in 0..251 {
            messages.step();
        }
        assert_eq!(messages.banner.unwrap().scale(), Vec2::ONE);
    }
    #[test]
    fn replacement_and_identical_announcements_restart_the_timer() {
        let mut messages = Messages::default();
        messages.send(Id::ParticleLaser);
        messages.step();
        messages.step();
        messages.send(Id::ParticleLaser);
        messages.step();
        assert_eq!(messages.banner.unwrap().age, 0);
        messages.send(Id::AiDroid);
        messages.send(Id::CaughtYou);
        messages.step();
        assert_eq!(messages.banner.unwrap().event.id, Id::CaughtYou);
    }
}
