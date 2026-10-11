//! A small typography lab. Candidate SVG paths are loaded from TOML at runtime.

#[path = "lettering_preview/messages.rs"]
mod messages;

#[allow(dead_code)]
#[path = "../src/cry.rs"]
mod cry;
#[allow(dead_code)]
#[path = "../src/sines.rs"]
mod sines;

use bevy::{
    camera::{RenderTarget, ScalingMode},
    core_pipeline::tonemapping::Tonemapping,
    prelude::*,
    render::{
        render_resource::TextureFormat,
        view::screenshot::{Screenshot, ScreenshotCaptured, save_to_disk},
    },
    window::{Monitor, MonitorSelection, PrimaryMonitor, PrimaryWindow, WindowPosition},
};
use clap::{Parser, ValueEnum};
use lyon_tessellation::{
    math::point,
    path::{Path as LyonPath, iterator::PathIterator},
};
use messages::geometry::{self, Glyphs, Style, Triangle};
use serde::Deserialize;
use std::{collections::BTreeMap, fs, path::PathBuf, time::Duration};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Clone, Copy, Debug, ValueEnum)]
enum Pose {
    Both,
    Straight,
    Slanted,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum Background {
    Black,
    Web,
}

#[derive(Parser, Resource)]
#[command(about = "Compare editable vector lettering using T2K's actual 3D materials")]
struct Options {
    #[arg(long, default_value = concat!(env!("CARGO_MANIFEST_DIR"), "/tools/lettering-preview.toml"))]
    config: PathBuf,
    /// One candidate ID, or 'current'; omit for the whole comparison sheet
    #[arg(long)]
    variant: Option<String>,
    #[arg(long, value_enum, default_value = "both")]
    pose: Pose,
    #[arg(long, value_enum, default_value = "black")]
    background: Background,
    #[arg(long)]
    output: Option<PathBuf>,
    #[arg(long)]
    width: Option<u32>,
    #[arg(long)]
    height: Option<u32>,
    /// Keep the window open and reload edits; R reloads, S saves, Esc exits
    #[arg(long)]
    watch: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Spec {
    message: String,
    advance: f32,
    cap_height: f32,
    slant: f32,
    variants: Vec<Variant>,
    fonts: BTreeMap<String, BTreeMap<char, String>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Variant {
    id: String,
    label: String,
    family: String,
    inset: f32,
}

struct Row {
    id: String,
    label: String,
    glyphs: BTreeMap<char, Vec<Triangle>>,
    top: f32,
    height: f32,
}

struct Layout {
    spec: Spec,
    rows: Vec<Row>,
    poses: Vec<f32>,
    width: u32,
    height: u32,
}

#[derive(Resource)]
struct Preview {
    layout: Layout,
    source: String,
    mesh: Handle<Mesh>,
    labels: Vec<Entity>,
    image: Handle<Image>,
    camera: Option<Entity>,
    next_poll: Duration,
    frames: u32,
    save: bool,
    capturing: bool,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("Lettering preview failed: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let mut options = Options::parse();
    let source = fs::read_to_string(&options.config)?;
    let layout = load(&source, &options)?;
    if options.output.is_none() {
        let name = options.variant.as_deref().unwrap_or("comparison");
        let background = match options.background {
            Background::Black => "black",
            Background::Web => "web",
        };
        options.output = Some(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join(format!("target/lettering-preview/{name}-{background}.png")),
        );
    }
    let output = options.output.as_ref().unwrap();
    if let Some(parent) = output.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    println!(
        "Previewing {}: {}",
        options.config.display(),
        layout.spec.message
    );
    println!("PNG output: {}", output.display());
    if options.watch {
        println!("Edit the TOML to reload. R: reload, S: save PNG, Esc: exit.");
    }
    App::new()
        .insert_resource(Preview {
            layout,
            source,
            mesh: default(),
            labels: Vec::new(),
            image: default(),
            camera: None,
            next_poll: Duration::ZERO,
            frames: 0,
            save: false,
            capturing: false,
        })
        .insert_resource(options)
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "T2K lettering comparison".into(),
                        resolution: bevy::window::WindowResolution::new(960, 540),
                        ..default()
                    }),
                    ..default()
                })
                .disable::<bevy::audio::AudioPlugin>()
                .disable::<bevy::gilrs::GilrsPlugin>(),
        )
        .add_systems(Startup, setup)
        .add_systems(Update, (fit_viewer, reload, capture).chain())
        .run();
    Ok(())
}

fn load(source: &str, options: &Options) -> Result<Layout> {
    let spec: Spec = toml::from_str(source)?;
    if spec.message.is_empty()
        || !spec.advance.is_finite()
        || spec.advance <= 0.
        || !spec.cap_height.is_finite()
        || spec.cap_height <= 0.
        || !spec.slant.is_finite()
    {
        return Err("Message, advance, cap_height, and slant must be valid".into());
    }
    let mut rows = Vec::new();
    let mut ids = std::collections::BTreeSet::new();
    for variant in &spec.variants {
        if !ids.insert(variant.id.to_ascii_lowercase())
            || variant.id.eq_ignore_ascii_case("current")
        {
            return Err(format!("Duplicate or reserved variant ID {}", variant.id).into());
        }
        if !variant.inset.is_finite() || !(-0.25..=0.75).contains(&variant.inset) {
            return Err(format!("{}: inset must be between -0.25 and 0.75", variant.id).into());
        }
        let font = spec
            .fonts
            .get(&variant.family)
            .ok_or_else(|| format!("Missing family {}", variant.family))?;
        let mut glyphs = BTreeMap::new();
        for c in spec.message.chars().filter(|c| !c.is_whitespace()) {
            if glyphs.contains_key(&c) {
                continue;
            }
            let path = font
                .get(&c)
                .ok_or_else(|| format!("{} has no glyph {c}", variant.family))?;
            let mut contours =
                svg_contours(path).map_err(|e| format!("{} / {c}: {e}", variant.id))?;
            thin(&mut contours, variant.inset);
            glyphs.insert(c, messages::candidate(&contours));
        }
        rows.push(row(variant.id.clone(), variant.label.clone(), glyphs)?);
    }
    let current = Glyphs::default();
    let glyphs = spec
        .message
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| (c, current.get(c, Style::Chrome).triangles.clone()))
        .collect();
    rows.insert(
        0,
        row("current".into(), "Current game / baseline".into(), glyphs)?,
    );
    if let Some(id) = &options.variant {
        rows.retain(|r| r.id.eq_ignore_ascii_case(id));
        if rows.is_empty() {
            return Err(format!("Unknown variant {id}").into());
        }
    }
    let poses = match options.pose {
        Pose::Both => vec![0., spec.slant],
        Pose::Straight => vec![0.],
        Pose::Slanted => vec![spec.slant],
    };
    let width = options
        .width
        .unwrap_or(if poses.len() == 2 { 3840 } else { 1920 });
    let height = options.height.unwrap_or(432 * rows.len() as u32);
    if !(640..=8192).contains(&width) || !(240..=8192).contains(&height) {
        return Err("Preview dimensions must be 640..8192 by 240..8192".into());
    }
    Ok(Layout {
        spec,
        rows,
        poses,
        width,
        height,
    })
}

fn row(id: String, label: String, glyphs: BTreeMap<char, Vec<Triangle>>) -> Result<Row> {
    let mut top = f32::INFINITY;
    let mut bottom = f32::NEG_INFINITY;
    for (p, _) in glyphs
        .values()
        .flatten()
        .flatten()
        .filter(|(p, _)| p.z >= 0.)
    {
        if !p.is_finite() {
            return Err(format!("Invalid geometry in {id}").into());
        }
        top = top.min(p.y);
        bottom = bottom.max(p.y);
    }
    if !top.is_finite() || bottom - top < 1. {
        return Err(format!("No usable geometry in {id}").into());
    }
    Ok(Row {
        id,
        label,
        glyphs,
        top,
        height: bottom - top,
    })
}

fn svg_contours(text: &str) -> Result<Vec<Vec<Vec2>>> {
    use lyon_tessellation::path::Event;
    use svgtypes::PathSegment;
    let mut builder = LyonPath::builder();
    let mut position = Vec2::ZERO;
    let mut start = Vec2::ZERO;
    let mut open = false;
    let absolute = |abs: bool, x: f64, y: f64, p: Vec2| {
        if abs {
            Vec2::new(x as f32, y as f32)
        } else {
            p + Vec2::new(x as f32, y as f32)
        }
    };
    for segment in svgtypes::PathParser::from(text) {
        match segment? {
            PathSegment::MoveTo { abs, x, y } => {
                if open {
                    return Err("Every candidate contour must be closed with Z".into());
                }
                position = absolute(abs, x, y, position);
                start = position;
                builder.begin(point(position.x, position.y));
                open = true;
            }
            PathSegment::LineTo { abs, x, y } => {
                if !open {
                    return Err("L requires an open contour".into());
                }
                position = absolute(abs, x, y, position);
                builder.line_to(point(position.x, position.y));
            }
            PathSegment::HorizontalLineTo { abs, x } => {
                if !open {
                    return Err("H requires an open contour".into());
                }
                position.x = if abs { x as f32 } else { position.x + x as f32 };
                builder.line_to(point(position.x, position.y));
            }
            PathSegment::VerticalLineTo { abs, y } => {
                if !open {
                    return Err("V requires an open contour".into());
                }
                position.y = if abs { y as f32 } else { position.y + y as f32 };
                builder.line_to(point(position.x, position.y));
            }
            PathSegment::CurveTo {
                abs,
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => {
                if !open {
                    return Err("C requires an open contour".into());
                }
                let a = absolute(abs, x1, y1, position);
                let b = absolute(abs, x2, y2, position);
                if !a.is_finite() || !b.is_finite() {
                    return Err("Non-finite SVG control point".into());
                }
                position = absolute(abs, x, y, position);
                builder.cubic_bezier_to(
                    point(a.x, a.y),
                    point(b.x, b.y),
                    point(position.x, position.y),
                );
            }
            PathSegment::Quadratic { abs, x1, y1, x, y } => {
                if !open {
                    return Err("Q requires an open contour".into());
                }
                let a = absolute(abs, x1, y1, position);
                if !a.is_finite() {
                    return Err("Non-finite SVG control point".into());
                }
                position = absolute(abs, x, y, position);
                builder.quadratic_bezier_to(point(a.x, a.y), point(position.x, position.y));
            }
            PathSegment::ClosePath { .. } => {
                if !open {
                    return Err("Z requires an open contour".into());
                }
                builder.end(true);
                position = start;
                open = false;
            }
            _ => return Err("Use SVG M, L, H, V, C, Q, and Z commands".into()),
        }
        if !position.is_finite() {
            return Err("Non-finite SVG coordinate".into());
        }
    }
    if open {
        return Err("Every contour must end with Z".into());
    }
    let mut contours = Vec::new();
    let mut contour = Vec::new();
    for event in builder.build().iter().flattened(0.012) {
        match event {
            Event::Begin { at } => {
                contour.clear();
                contour.push(Vec2::new(at.x, at.y));
            }
            Event::Line { to, .. } => {
                let p = Vec2::new(to.x, to.y);
                if contour.last().is_none_or(|q| q.distance_squared(p) > 1e-10) {
                    contour.push(p);
                }
            }
            Event::End { .. } => {
                if contour.len() > 1
                    && contour[0].distance_squared(*contour.last().unwrap()) < 1e-10
                {
                    contour.pop();
                }
                if contour.len() < 3 {
                    return Err("A contour needs at least three vertices".into());
                }
                contours.push(contour.clone());
            }
            _ => unreachable!(),
        }
    }
    if contours.is_empty() {
        return Err("Empty SVG path".into());
    }
    Ok(contours)
}

fn contains(contour: &[Vec2], p: Vec2) -> bool {
    let mut inside = false;
    for i in 0..contour.len() {
        let a = contour[i];
        let b = contour[(i + 1) % contour.len()];
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
    }
    inside
}

fn thin(contours: &mut [Vec<Vec2>], amount: f32) {
    for i in 0..contours.len() {
        let hole = (0..contours.len())
            .filter(|&j| i != j && contains(&contours[j], contours[i][0]))
            .count()
            % 2
            == 1;
        let area: f32 = (0..contours[i].len())
            .map(|j| contours[i][j].perp_dot(contours[i][(j + 1) % contours[i].len()]))
            .sum();
        if (area < 0.) != hole {
            contours[i].reverse();
        }
    }
    if amount == 0. {
        return;
    }
    for contour in contours {
        *contour = (0..contour.len())
            .map(|i| {
                let p = contour[i];
                let a = contour[(i + contour.len() - 1) % contour.len()];
                let b = contour[(i + 1) % contour.len()];
                let n0 = (p - a).normalize_or_zero().perp();
                let n1 = (b - p).normalize_or_zero().perp();
                let direction = (n0 + n1).normalize_or_zero();
                p + direction * (amount / direction.dot(n1).max(0.35))
            })
            .collect();
    }
}

fn render(layout: &Layout, background: Background) -> Vec<Triangle> {
    let width = layout.width as f32;
    let height = layout.height as f32;
    let row_height = height / layout.rows.len() as f32;
    let column_width = width / layout.poses.len() as f32;
    let word_width = layout.spec.message.chars().count() as f32 * layout.spec.advance;
    let scale =
        ((column_width - 160.) / word_width).min((row_height - 170.) / layout.spec.cap_height);
    let mut triangles = Vec::new();
    let world = |p: Vec3| Vec3::new(p.x - width / 2., height / 2. - p.y, p.z);
    for (row_index, row) in layout.rows.iter().enumerate() {
        for (column, &shear) in layout.poses.iter().enumerate() {
            let cx = (column as f32 + 0.5) * column_width;
            let cy = (row_index as f32 + 0.6) * row_height;
            if matches!(background, Background::Web) {
                let center = Vec2::new(cx, cy);
                for lane in 0..16 {
                    let a = std::f32::consts::TAU * lane as f32 / 16.;
                    let b = std::f32::consts::TAU * (lane + 1) as f32 / 16.;
                    let radius = Vec2::new(column_width * 0.56, row_height * 0.65);
                    let outer = [
                        center + Vec2::from_angle(a) * radius,
                        center + Vec2::from_angle(b) * radius,
                    ];
                    let inner = outer.map(|p| center + (p - center) * 0.15);
                    for face in [
                        [outer[0], outer[1], inner[1]],
                        [outer[0], inner[1], inner[0]],
                    ] {
                        triangles.push(face.map(|p| {
                            (
                                world(p.extend(-20.)),
                                Color::srgb(0.005, 0.007, if lane % 2 == 0 { 0.19 } else { 0.24 }),
                            )
                        }));
                    }
                    let side = (outer[0] - inner[0]).normalize_or_zero().perp() * 1.1;
                    let corners = [
                        outer[0] - side,
                        outer[0] + side,
                        inner[0] + side,
                        inner[0] - side,
                    ];
                    for face in [[0, 1, 2], [0, 2, 3]] {
                        triangles.push(face.map(|i| {
                            (
                                world(corners[i].extend(-19.)),
                                Color::srgb(0.03, 0.05, 0.75),
                            )
                        }));
                    }
                }
            }
            for (index, c) in layout.spec.message.chars().enumerate() {
                if c.is_whitespace() {
                    continue;
                }
                for triangle in &row.glyphs[&c] {
                    triangles.push(triangle.map(|(p, color)| {
                        let x = p.x - 7.5;
                        let y = (p.y - row.top) / row.height * layout.spec.cap_height
                            - layout.spec.cap_height / 2.;
                        let xy = Vec2::new(x + y * shear, y - x * shear * 0.5);
                        (
                            world(Vec3::new(
                                cx + (index as f32 * layout.spec.advance - word_width / 2.
                                    + 7.5
                                    + xy.x)
                                    * scale,
                                cy + xy.y * scale,
                                p.z * scale,
                            )),
                            color,
                        )
                    }));
                }
            }
        }
    }
    triangles
}

fn labels(commands: &mut Commands, layout: &Layout, camera: Entity) -> Vec<Entity> {
    let row_height = layout.height as f32 / layout.rows.len() as f32;
    let column_width = layout.width as f32 / layout.poses.len() as f32;
    let mut entities = Vec::new();
    for (i, row) in layout.rows.iter().enumerate() {
        for (column, shear) in layout.poses.iter().enumerate() {
            entities.push(
                commands
                    .spawn((
                        UiTargetCamera(camera),
                        Text::new(format!(
                            "{}   {}  |  {}",
                            row.id.to_uppercase(),
                            row.label,
                            if *shear == 0. { "straight" } else { "slanted" }
                        )),
                        TextFont {
                            font_size: 26.,
                            ..default()
                        },
                        TextColor(Color::srgb(0.8, 0.82, 0.88)),
                        Node {
                            position_type: PositionType::Absolute,
                            left: percent(
                                (column as f32 * column_width + 40.) / layout.width as f32 * 100.,
                            ),
                            top: percent(
                                (i as f32 * row_height + 35.) / layout.height as f32 * 100.,
                            ),
                            ..default()
                        },
                    ))
                    .id(),
            );
        }
    }
    entities
}

fn setup(
    mut commands: Commands,
    options: Res<Options>,
    mut preview: ResMut<Preview>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    preview.image = images.add(Image::new_target_texture(
        preview.layout.width,
        preview.layout.height,
        TextureFormat::Rgba8UnormSrgb,
        None,
    ));
    let camera = commands
        .spawn((
            Camera3d::default(),
            Camera {
                order: -1,
                ..default()
            },
            RenderTarget::Image(preview.image.clone().into()),
            bevy::render::view::NoIndirectDrawing,
            Projection::Orthographic(OrthographicProjection {
                scaling_mode: ScalingMode::FixedVertical {
                    viewport_height: preview.layout.height as f32,
                },
                ..OrthographicProjection::default_3d()
            }),
            Transform::from_xyz(0., 0., 500.),
            Tonemapping::None,
            Msaa::Sample4,
        ))
        .id();
    preview.camera = Some(camera);
    commands.spawn(Camera2d);
    commands.spawn(Sprite::from_image(preview.image.clone()));
    let mesh = meshes.add(geometry::mesh(render(&preview.layout, options.background)));
    commands.spawn((
        Mesh3d(mesh.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            unlit: true,
            cull_mode: None,
            ..default()
        })),
        bevy::camera::visibility::NoFrustumCulling,
    ));
    preview.mesh = mesh;
    preview.labels = labels(&mut commands, &preview.layout, camera);
}

fn fitted_size(canvas: Vec2, bounds: Vec2) -> Vec2 {
    canvas * (bounds.x / canvas.x).min(bounds.y / canvas.y).min(1.)
}

fn fit_viewer(
    preview: Res<Preview>,
    monitors: Query<&Monitor, With<PrimaryMonitor>>,
    mut windows: Query<(Entity, &mut Window), With<PrimaryWindow>>,
    mut sprites: Query<&mut Transform, With<Sprite>>,
    mut fitted: Local<bool>,
    _main_thread: bevy::ecs::system::NonSendMarker,
) {
    let Ok((entity, mut window)) = windows.single_mut() else {
        return;
    };
    let canvas = Vec2::new(preview.layout.width as f32, preview.layout.height as f32);
    let monitor = if *fitted {
        None
    } else {
        monitors
            .single()
            .ok()
            .map(|monitor| {
                (
                    monitor.physical_size().as_vec2() / monitor.scale_factor as f32,
                    MonitorSelection::Primary,
                )
            })
            .or_else(|| {
                bevy::winit::WINIT_WINDOWS.with(|windows| {
                    windows
                        .borrow()
                        .get_window(entity)?
                        .current_monitor()
                        .map(|monitor| {
                            let size = monitor.size();
                            (
                                Vec2::new(size.width as f32, size.height as f32)
                                    / monitor.scale_factor() as f32,
                                MonitorSelection::Current,
                            )
                        })
                })
            })
    };
    if let Some((bounds, selection)) = monitor {
        let bounds = bounds * 0.8;
        let size = fitted_size(canvas, bounds.min(Vec2::new(1600., 900.)));
        window.resolution.set(size.x, size.y);
        window.position = WindowPosition::Centered(selection);
        println!(
            "Viewer: {:.0} x {:.0} logical pixels; PNG: {} x {} pixels",
            size.x, size.y, preview.layout.width, preview.layout.height
        );
        *fitted = true;
    }
    let size = fitted_size(canvas, Vec2::new(window.width(), window.height()));
    for mut transform in &mut sprites {
        transform.scale = Vec3::splat(size.x / canvas.x);
    }
}

fn reload(
    mut commands: Commands,
    options: Res<Options>,
    mut preview: ResMut<Preview>,
    mut meshes: ResMut<Assets<Mesh>>,
    time: Res<Time<Real>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        commands.write_message(AppExit::Success);
    }
    if keys.just_pressed(KeyCode::KeyS) {
        preview.save = true;
    }
    if !options.watch {
        return;
    }
    if time.elapsed() < preview.next_poll && !keys.just_pressed(KeyCode::KeyR) {
        return;
    }
    preview.next_poll = time.elapsed() + Duration::from_millis(250);
    match fs::read_to_string(&options.config) {
        Ok(source) if source != preview.source || keys.just_pressed(KeyCode::KeyR) => {
            match load(&source, &options) {
                Ok(layout)
                    if layout.width == preview.layout.width
                        && layout.height == preview.layout.height =>
                {
                    *meshes.get_mut(&preview.mesh).unwrap() =
                        geometry::mesh(render(&layout, options.background));
                    for entity in preview.labels.drain(..) {
                        commands.entity(entity).despawn();
                    }
                    preview.labels = labels(&mut commands, &layout, preview.camera.unwrap());
                    preview.layout = layout;
                    preview.source = source;
                    preview.frames = 0;
                    preview.save = true;
                    println!("Reloaded {}", options.config.display());
                }
                Ok(_) => eprintln!("Restart the preview to change its canvas size"),
                Err(error) => {
                    eprintln!("Keeping last valid preview: {error}");
                    preview.source = source;
                }
            }
        }
        Ok(_) => {}
        Err(error) => eprintln!("Cannot read preview file: {error}"),
    }
}

fn capture(mut commands: Commands, options: Res<Options>, mut preview: ResMut<Preview>) {
    preview.frames += 1;
    if preview.capturing || preview.frames < 30 || (options.watch && !preview.save) {
        return;
    }
    preview.capturing = true;
    preview.save = false;
    let quit = !options.watch;
    commands
        .spawn(Screenshot::image(preview.image.clone()))
        .observe(save_to_disk(options.output.as_ref().unwrap().clone()))
        .observe(
            move |_: On<ScreenshotCaptured>,
                  mut preview: ResMut<Preview>,
                  mut exit: MessageWriter<AppExit>| {
                if quit {
                    exit.write(AppExit::Success);
                } else {
                    preview.capturing = false;
                }
            },
        );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options() -> Options {
        Options::parse_from(["preview"])
    }

    #[test]
    fn thicker_strokes_expand_the_outline_and_shrink_counters() {
        let mut contours = svg_contours("M 0 0 H 15 V 18 H 0 Z M 3 3 H 12 V 15 H 3 Z").unwrap();
        thin(&mut contours, -0.15);
        assert!(contours[0].iter().any(|p| p.x < -0.14));
        assert!(contours[0].iter().any(|p| p.y > 18.14));
        assert!(contours[1].iter().all(|p| p.x >= 3.14 && p.x <= 11.86));
        assert!(contours[1].iter().all(|p| p.y >= 3.14 && p.y <= 14.86));
    }

    #[test]
    fn viewer_fits_scaled_monitors_and_preserves_canvas_aspect_ratio() {
        let canvas = Vec2::new(3840., 2160.);
        for (physical, dpi) in [(canvas, 2.), (Vec2::new(1920., 1080.), 1.5)] {
            let bounds = physical / dpi * 0.8;
            let size = fitted_size(canvas, bounds.min(Vec2::new(1600., 900.)));
            assert!(size.cmple(bounds).all());
            assert!((size.x / size.y - canvas.x / canvas.y).abs() < 0.001);
        }
        assert!(
            fitted_size(canvas, Vec2::new(1000., 1000.)).distance(Vec2::new(1000., 562.5)) < 0.001
        );
    }

    #[test]
    fn all_candidates_load_with_real_curves_holes_and_matching_cap_height() {
        let source = include_str!("../tools/lettering-preview.toml");
        let layout = load(source, &options()).unwrap();
        assert_eq!(layout.rows.len(), 5);
        for row in &layout.rows {
            assert!(row.height > 15.);
            assert!(
                row.glyphs
                    .values()
                    .flatten()
                    .flatten()
                    .all(|(p, c)| p.is_finite() && c.to_linear().alpha == 1.)
            );
            assert!(row.glyphs[&'O'].iter().flatten().any(|(p, _)| p.z < 0.));
        }
        assert!(svg_contours(&layout.spec.fonts["rounded"][&'O']).unwrap()[0].len() > 30);
        assert_ne!(layout.rows[1].glyphs[&'O'], layout.rows[2].glyphs[&'O']);
        assert_ne!(layout.rows[1].glyphs[&'O'], layout.rows[3].glyphs[&'O']);
    }

    #[test]
    fn invalid_edits_are_reported_and_svg_paths_require_closed_contours() {
        assert!(svg_contours("M 0 0 L 1 0").is_err());
        assert!(svg_contours("L 1 0 Z").is_err());
        assert!(svg_contours("M 0 0 A 1 1 0 0 0 1 1 Z").is_err());
        assert!(svg_contours("").is_err());
        assert!(svg_contours("M 0 0 C 1e50 1 2 2 3 3 Z").is_err());
        let relative = svg_contours("M 0 0 l 4 0 l 0 4 Z m 8 0 l 4 0 l 0 4 Z").unwrap();
        assert_eq!(relative[1][0], Vec2::new(8., 0.));
        let source = include_str!("../tools/lettering-preview.toml")
            .replace("family = \"original\"", "family = \"missing\"");
        assert!(load(&source, &options()).is_err());
    }
}
