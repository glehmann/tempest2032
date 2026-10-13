//! The portal logo uses the same CRY scale/rotate feedback as the game-over view.

use super::{FrontEnd, Screen};
use bevy::{
    asset::RenderAssetUsages,
    image::{CompressedImageFormats, ImageSampler, ImageType},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

const WIDTH: usize = 384;
const HEIGHT: usize = 240;
const ART_SIZE: usize = 96;

#[derive(Component)]
pub(super) struct Canvas;

#[derive(Resource)]
struct Feedback {
    pixels: Vec<u16>,
    scratch: Vec<u16>,
    art: Vec<u16>,
    image: Handle<Image>,
    clock: Timer,
    tick: u32,
}

pub(super) fn add_systems(app: &mut App) {
    app.add_systems(Startup, setup.after(super::setup))
        .add_systems(Update, advance);
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut canvas: Single<&mut ImageNode, With<Canvas>>,
) {
    let logo = Image::from_buffer(
        include_bytes!("../../assets/steam/logo.png"),
        ImageType::Extension("png"),
        CompressedImageFormats::NONE,
        true,
        ImageSampler::default(),
        RenderAssetUsages::default(),
    )
    .unwrap();
    let data = logo.data.as_ref().unwrap();
    let mut art = vec![0; ART_SIZE * ART_SIZE];
    for y in 0..ART_SIZE {
        for x in 0..ART_SIZE {
            let offset = (y * logo.height() as usize / ART_SIZE * logo.width() as usize
                + x * logo.width() as usize / ART_SIZE)
                * 4;
            if data[offset + 3] < 128 {
                continue;
            }
            let rgb = Vec3::new(
                data[offset] as f32,
                data[offset + 1] as f32,
                data[offset + 2] as f32,
            );
            art[y * ART_SIZE + x] = (0..256)
                .map(|hue| {
                    let palette = Vec3::new(
                        crate::cry::RED[hue >> 4][hue & 15] as f32,
                        crate::cry::GREEN[hue >> 4][hue & 15] as f32,
                        crate::cry::BLUE[hue >> 4][hue & 15] as f32,
                    );
                    let intensity = (rgb.dot(palette) / palette.length_squared().max(1.) * 255.)
                        .clamp(0., 255.) as u16;
                    (
                        (palette * (intensity as f32 / 255.) - rgb).length_squared(),
                        (hue as u16) << 8 | intensity,
                    )
                })
                .min_by(|a, b| a.0.total_cmp(&b.0))
                .unwrap()
                .1;
        }
    }
    let mut image = Image::new_fill(
        Extent3d {
            width: WIDTH as u32,
            height: HEIGHT as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::linear();
    let image = images.add(image);
    canvas.image = image.clone();
    commands.insert_resource(Feedback {
        pixels: vec![0; WIDTH * HEIGHT],
        scratch: vec![0; WIDTH * HEIGHT],
        art,
        image,
        clock: Timer::from_seconds(1. / 15., TimerMode::Repeating),
        tick: 0,
    });
}

fn advance(
    front: Res<FrontEnd>,
    time: Res<Time<Real>>,
    mut feedback: ResMut<Feedback>,
    mut images: ResMut<Assets<Image>>,
) {
    if front.screen == Screen::Gameplay {
        return;
    }
    if !feedback.clock.tick(time.delta()).just_finished() {
        return;
    }
    let feedback = &mut *feedback;
    // Bound catch-up after window suspension; menu animation has no gameplay role.
    for _ in 0..feedback.clock.times_finished_this_tick().min(4) {
        crate::cry::feedback(
            &mut feedback.pixels,
            &mut feedback.scratch,
            UVec2::new(WIDTH as u32, HEIGHT as u32),
            ((feedback.tick / 8) & 15) as i32 - 7,
            Vec2::splat(500. / 256.),
            -8 * 65536,
        );
        let scale =
            1. + crate::sines::sine((feedback.tick.wrapping_mul(3) & 255) as i32) as f32 / 2048.;
        for y in 0..ART_SIZE {
            for x in 0..ART_SIZE {
                let pixel = feedback.art[y * ART_SIZE + x];
                if pixel == 0 {
                    continue;
                }
                let dx = (WIDTH as f32 / 2. + (x as f32 - ART_SIZE as f32 / 2.) * scale) as usize;
                let dy = (HEIGHT as f32 / 2. + (y as f32 - ART_SIZE as f32 / 2.) * scale) as usize;
                feedback.pixels[dy * WIDTH + dx] = pixel;
            }
        }
        feedback.tick = feedback.tick.wrapping_add(1);
    }
    if let Some(image) = images.get_mut(&feedback.image) {
        for (pixel, rgba) in feedback
            .pixels
            .iter()
            .zip(image.data.as_mut().unwrap().as_chunks_mut::<4>().0)
        {
            let hue = (pixel >> 8) as usize;
            for (channel, table) in [&crate::cry::RED, &crate::cry::GREEN, &crate::cry::BLUE]
                .iter()
                .enumerate()
            {
                rgba[channel] = ((u16::from(table[hue >> 4][hue & 15]) * (pixel & 255)) >> 8) as u8;
            }
            rgba[3] = 255;
        }
    }
}
