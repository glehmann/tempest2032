//! Front-end screens stay responsive without advancing the gameplay simulation.

mod background;
mod lettering;

use crate::{
    difficulty::Difficulty,
    game::{Game, Status},
    level::{Levels, LoadReason},
    scores::Store,
};
use bevy::{
    asset::RenderAssetUsages,
    image::{CompressedImageFormats, ImageSampler, ImageType},
    input::{
        ButtonState,
        keyboard::{Key, KeyboardFocusLost, KeyboardInput},
    },
    prelude::*,
    ui::widget::ViewportNode,
    window::Ime,
};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc::{self, Receiver};

const CONFIG_CHOICES: usize = if cfg!(target_arch = "wasm32") { 2 } else { 3 };

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Screen {
    Welcome,
    Configuration,
    Initials,
    #[default]
    Gameplay,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub(crate) enum Preview {
    Welcome,
    Configuration,
    Initials,
}

#[derive(Resource)]
pub(crate) struct FrontEnd {
    pub(crate) screen: Screen,
    pub(crate) eligible: bool,
    pub(crate) originals: Vec<crate::level::Level>,
    selected: usize,
    initials: [u8; 3],
    cursor: usize,
    typed: usize,
    score: u32,
    highlight: Option<(Difficulty, usize)>,
    #[cfg(not(target_arch = "wasm32"))]
    install: Option<std::sync::Mutex<Receiver<Result<String, String>>>>,
    notice: String,
    blocked: Vec<KeyCode>,
    physical: ButtonInput<KeyCode>,
}

impl FrontEnd {
    pub(crate) fn preview_initials(&mut self) {
        self.enter_initials(12345, "AAA");
    }

    pub(crate) fn new(screen: Screen, eligible: bool, originals: Vec<crate::level::Level>) -> Self {
        Self {
            screen,
            eligible,
            originals,
            selected: 0,
            initials: *b"AAA",
            cursor: 0,
            typed: 0,
            score: 0,
            highlight: None,
            #[cfg(not(target_arch = "wasm32"))]
            install: None,
            notice: String::new(),
            blocked: Vec::new(),
            physical: default(),
        }
    }

    fn enter_initials(&mut self, score: u32, previous: &str) {
        self.screen = Screen::Initials;
        self.score = score;
        self.initials.copy_from_slice(previous.as_bytes());
        self.cursor = 0;
        self.typed = 0;
    }

    fn type_letters(&mut self, text: &str) {
        for letter in text.bytes().filter(u8::is_ascii_alphabetic) {
            if self.typed == 3 {
                break;
            }
            self.initials[self.cursor] = letter.to_ascii_uppercase();
            self.typed += 1;
            self.cursor = (self.cursor + 1).min(2);
        }
    }
}

pub(crate) fn playing(front: Option<Res<FrontEnd>>) -> bool {
    front.is_none_or(|front| front.screen == Screen::Gameplay)
}

pub(crate) fn add_systems(app: &mut App) {
    app.add_systems(Startup, setup)
        .add_systems(
            PreUpdate,
            input
                .after(crate::gamepad::map_controllers)
                .before(crate::pause::input),
        )
        .add_systems(Update, draw)
        .add_systems(Update, draw_game_over_hint);
    lettering::add_systems(app);
    #[cfg(not(target_arch = "wasm32"))]
    app.add_systems(Update, installation.before(draw));
    background::add_systems(app);
}

fn confirm(keys: &ButtonInput<KeyCode>) -> bool {
    keys.any_just_pressed([KeyCode::Enter, KeyCode::NumpadEnter, KeyCode::Space])
}

fn welcome(front: &mut FrontEnd, messages: &mut crate::messages::Messages) {
    front.screen = Screen::Welcome;
    *messages = default();
}

pub(crate) fn abandon(
    front: &mut FrontEnd,
    game: &mut Game,
    messages: &mut crate::messages::Messages,
    commands: &mut Commands,
) {
    *game = Game::default();
    crate::sound::reset(commands);
    welcome(front, messages);
    front.blocked = front.physical.get_pressed().copied().collect();
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn input(
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut text: MessageReader<KeyboardInput>,
    mut ime: MessageReader<Ime>,
    mut front: ResMut<FrontEnd>,
    mut store: ResMut<Store>,
    mut difficulty: ResMut<Difficulty>,
    mut levels: ResMut<Levels>,
    mut game: ResMut<Game>,
    mut messages: ResMut<crate::messages::Messages>,
    mut windows: Query<&mut Window>,
    #[cfg(not(target_arch = "wasm32"))] mut exit: MessageWriter<AppExit>,
    mut focus_lost: MessageReader<KeyboardFocusLost>,
    time: Option<Res<Time<Virtual>>>,
    mut commands: Commands,
) {
    let old_screen = front.screen;
    if focus_lost.read().next().is_some() {
        front.physical.reset_all();
        front.blocked.clear();
    }
    let events: Vec<_> = text.read().cloned().collect();
    for event in &events {
        if event.state == ButtonState::Released {
            front.physical.release(event.key_code);
            front.blocked.retain(|key| *key != event.key_code);
        } else {
            front.physical.press(event.key_code);
        }
    }
    for key in &front.blocked {
        keys.reset(*key);
    }
    if front.screen == Screen::Gameplay {
        ime.clear();
        if keys.just_pressed(KeyCode::Escape) && time.is_none_or(|time| !time.is_paused()) {
            abandon(&mut front, &mut game, &mut messages, &mut commands);
        } else if game.status != Status::GameOver || !confirm(&keys) {
            return;
        } else if front.eligible && store.data.rank(*difficulty, game.score).is_some() {
            front.enter_initials(game.score, &store.data.initials);
        } else {
            welcome(&mut front, &mut messages);
        }
    } else {
        let typed: String = events
            .iter()
            .filter(|event| event.state == ButtonState::Pressed)
            .filter_map(|event| match &event.logical_key {
                Key::Character(character) => Some(character.as_str()),
                _ => event.text.as_deref(),
            })
            .collect();
        let committed: String = ime
            .read()
            .filter_map(|event| {
                if let Ime::Commit { value, .. } = event {
                    Some(value.as_str())
                } else {
                    None
                }
            })
            .collect();
        match front.screen {
            Screen::Welcome => {
                if keys.any_just_pressed([KeyCode::KeyP, KeyCode::Pause]) {
                    front.screen = Screen::Configuration;
                    front.selected = 0;
                } else if confirm(&keys) {
                    levels.0 = front.originals.clone();
                    for level in &mut levels.0 {
                        level.scale_difficulty(difficulty.scale());
                    }
                    game.load = Some(LoadReason::Restart);
                    front.screen = Screen::Gameplay;
                    front.highlight = None;
                } else if keys.just_pressed(KeyCode::Escape) {
                    #[cfg(not(target_arch = "wasm32"))]
                    exit.write(AppExit::Success);
                }
            }
            Screen::Configuration => {
                if keys.any_just_pressed([KeyCode::Escape, KeyCode::KeyP, KeyCode::Pause]) {
                    front.screen = Screen::Welcome;
                } else {
                    let up = keys.any_just_pressed([KeyCode::ArrowUp, KeyCode::KeyW]);
                    let down = keys.any_just_pressed([KeyCode::ArrowDown, KeyCode::KeyS]);
                    if up != down {
                        front.selected = (front.selected + if up { CONFIG_CHOICES - 1 } else { 1 })
                            % CONFIG_CHOICES;
                    }
                    if front.selected == 0 {
                        let left = keys.just_pressed(KeyCode::ArrowLeft);
                        let right = keys.just_pressed(KeyCode::ArrowRight) || confirm(&keys);
                        if left != right {
                            *difficulty = Difficulty::ALL
                                [(difficulty.index() + if left { 2 } else { 1 }) % 3];
                            store.data.difficulty = *difficulty;
                            store.save();
                        }
                    } else if confirm(&keys) {
                        if front.selected == CONFIG_CHOICES - 1 {
                            front.screen = Screen::Welcome;
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        if front.selected == 1 && front.install.is_none() {
                            if std::env::var_os("FLATPAK_ID").is_some() {
                                front.notice = "Steam installation is unavailable inside Flatpak. Use a native build to add the game to Steam.".into();
                            } else {
                                let (sender, receiver) = mpsc::channel();
                                front.install = Some(std::sync::Mutex::new(receiver));
                                front.notice = "Installing in Steam...".into();
                                std::thread::spawn(move || {
                                    let result = crate::steam::install_detected(None, None)
                                        .map(|installed| installed.message())
                                        .map_err(|error| error.to_string());
                                    let _ = sender.send(result);
                                });
                            }
                        }
                    }
                }
            }
            Screen::Initials => {
                front.type_letters(&typed);
                front.type_letters(&committed);
                if keys.just_pressed(KeyCode::Backspace) || keys.just_pressed(KeyCode::Escape) {
                    front.cursor = if front.typed > 0 {
                        front.typed - 1
                    } else {
                        front.cursor.saturating_sub(1)
                    };
                    front.typed = front.cursor;
                    let cursor = front.cursor;
                    front.initials[cursor] = b'A';
                }
                if keys.just_pressed(KeyCode::ArrowLeft) {
                    front.cursor = (front.cursor + 2) % 3;
                    front.typed = front.cursor;
                } else if keys.just_pressed(KeyCode::ArrowRight) {
                    front.cursor = (front.cursor + 1) % 3;
                    front.typed = front.cursor;
                }
                let up = keys.just_pressed(KeyCode::ArrowUp);
                let down = keys.just_pressed(KeyCode::ArrowDown);
                if up != down {
                    let cursor = front.cursor;
                    front.initials[cursor] =
                        b'A' + (front.initials[cursor] - b'A' + if up { 1 } else { 25 }) % 26;
                }
                if keys.any_just_pressed([KeyCode::Enter, KeyCode::NumpadEnter]) {
                    let initials = String::from_utf8(front.initials.to_vec()).unwrap();
                    front.highlight = store
                        .data
                        .insert(*difficulty, front.score, initials)
                        .map(|rank| (*difficulty, rank));
                    store.save();
                    welcome(&mut front, &mut messages);
                }
            }
            Screen::Gameplay => unreachable!(),
        }
    }
    if old_screen != front.screen {
        front.blocked = front.physical.get_pressed().copied().collect();
        for mut window in &mut windows {
            window.ime_enabled = front.screen == Screen::Initials;
            window.ime_position = Vec2::new(window.width() / 2., window.height() * 0.48);
        }
        keys.reset_all();
    } else {
        keys.clear();
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn installation(mut front: ResMut<FrontEnd>) {
    let result =
        front
            .install
            .as_ref()
            .and_then(|receiver| match receiver.lock().unwrap().try_recv() {
                Ok(result) => Some(result),
                Err(mpsc::TryRecvError::Disconnected) => {
                    Some(Err("Installer stopped unexpectedly".into()))
                }
                Err(mpsc::TryRecvError::Empty) => None,
            });
    if let Some(result) = result {
        front.install = None;
        front.notice = result.unwrap_or_else(|error| format!("Steam installation failed: {error}"));
    }
}

#[derive(Component)]
struct Root;
#[derive(Component)]
struct Panel;
#[derive(Component)]
struct Logo;
#[derive(Component)]
struct GameOverHint;
#[derive(Component)]
enum Label {
    Title,
    Body,
    Footer,
    Notice,
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let logo = images.add(
        Image::from_buffer(
            include_bytes!("../assets/steam/logo.png"),
            ImageType::Extension("png"),
            CompressedImageFormats::NONE,
            true,
            ImageSampler::linear(),
            RenderAssetUsages::RENDER_WORLD,
        )
        .unwrap(),
    );
    let lettering = lettering::setup(&mut commands, &mut images, &mut meshes, &mut materials);
    commands.spawn((
        GameOverHint,
        Text::default(),
        TextFont {
            font_size: 18.,
            ..default()
        },
        TextColor(Color::srgb(1., 0.88, 0.48)),
        TextLayout::new_with_justify(Justify::Center),
        Node {
            position_type: PositionType::Absolute,
            bottom: px(24),
            width: percent(100),
            ..default()
        },
        GlobalZIndex(10),
        Visibility::Hidden,
    ));
    commands
        .spawn((
            Root,
            Node {
                width: percent(100),
                height: percent(100),
                position_type: PositionType::Absolute,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            GlobalZIndex(10),
            BackgroundColor(Color::BLACK),
        ))
        .with_children(|root| {
            root.spawn((
                background::Canvas,
                Node {
                    position_type: PositionType::Absolute,
                    width: percent(100),
                    height: percent(100),
                    overflow: Overflow::clip(),
                    ..default()
                },
            ));
            root.spawn((
                Panel,
                Node {
                    width: percent(70),
                    padding: UiRect::all(px(20)),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    row_gap: px(16),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.01, 0.015, 0.035, 0.78)),
            ))
            .with_children(|panel| {
                panel.spawn((
                    Logo,
                    ImageNode::new(logo),
                    Node {
                        height: percent(22),
                        aspect_ratio: Some(1.),
                        flex_shrink: 0.,
                        ..default()
                    },
                ));
                panel.spawn((
                    lettering::Canvas,
                    ViewportNode::new(lettering),
                    Node {
                        width: percent(100),
                        height: percent(62),
                        flex_shrink: 0.,
                        ..default()
                    },
                ));
                for (label, color) in [
                    (Label::Title, Color::srgb(0.4, 0.9, 1.)),
                    (Label::Body, Color::srgb(1., 0.88, 0.48)),
                    (Label::Footer, Color::srgb(0.8, 0.85, 0.95)),
                    (Label::Notice, Color::srgb(1., 0.6, 0.65)),
                ] {
                    panel.spawn((
                        label,
                        Text::default(),
                        TextFont::default(),
                        TextColor(color),
                        TextLayout::new_with_justify(Justify::Center),
                    ));
                }
            });
        });
}

fn draw_game_over_hint(
    front: Res<FrontEnd>,
    game: Res<Game>,
    store: Res<Store>,
    difficulty: Res<Difficulty>,
    mut hint: Single<(&mut Visibility, &mut Text), With<GameOverHint>>,
) {
    let (visible, text) = &mut *hint;
    **visible = if front.screen == Screen::Gameplay && game.status == Status::GameOver {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    let value = if front.eligible && store.data.rank(*difficulty, game.score).is_some() {
        "ENTER / A  ENTER HIGH-SCORE INITIALS"
    } else {
        "ENTER / A  RETURN TO WELCOME"
    };
    if text.0 != value {
        text.0 = value.into();
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn draw(
    front: Res<FrontEnd>,
    store: Res<Store>,
    difficulty: Res<Difficulty>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut root: Query<&mut Node, With<Root>>,
    mut panels: Query<&mut Node, (With<Panel>, Without<Root>)>,
    mut illustrated_nodes: Query<
        &mut Node,
        (
            Or<(With<Logo>, With<lettering::Canvas>)>,
            Without<Root>,
            Without<Panel>,
            Without<Label>,
        ),
    >,
    mut labels: Query<
        (&Label, &mut Text, &mut TextFont, &mut Node),
        (Without<Root>, Without<Panel>),
    >,
) {
    let screen = front.screen;
    let illustrated = matches!(screen, Screen::Welcome | Screen::Initials);
    let scale = windows.iter().next().map_or(1., |window| {
        (window.height() / 800.).min(window.width() / 1100.)
    });
    for mut node in &mut root {
        node.display = if screen == Screen::Gameplay {
            Display::None
        } else {
            Display::Flex
        };
    }
    if screen == Screen::Gameplay {
        return;
    }
    for mut panel in &mut panels {
        panel.height = if illustrated { percent(86) } else { Val::Auto };
        panel.padding = UiRect::all(px(20. * scale));
        panel.row_gap = px(16. * scale);
    }
    for mut node in &mut illustrated_nodes {
        node.display = if illustrated {
            Display::Flex
        } else {
            Display::None
        };
    }
    let (title, body, footer) = match screen {
        Screen::Welcome => (
            String::new(),
            String::new(),
            if cfg!(target_arch = "wasm32") {
                "ENTER / A  PLAY\nP / START  CONFIGURATION".into()
            } else {
                "ENTER / A  PLAY\nP / START  CONFIGURATION     ESC / B  EXIT".into()
            },
        ),
        Screen::Configuration => {
            let choices = [
                format!("DIFFICULTY     < {} >", difficulty.label()),
                #[cfg(not(target_arch = "wasm32"))]
                if front.install.is_some() {
                    "INSTALLING IN STEAM...".into()
                } else {
                    "INSTALL IN STEAM".into()
                },
                "BACK".into(),
            ];
            let body = choices
                .iter()
                .enumerate()
                .map(|(index, choice)| {
                    format!(
                        "{} {choice}",
                        if index == front.selected { ">" } else { " " }
                    )
                })
                .collect::<Vec<_>>()
                .join("\n\n");
            ("CONFIGURATION".into(), body,
                "UP / DOWN  SELECT     LEFT / RIGHT  DIFFICULTY\nENTER / A  CONFIRM     P / START / ESC / B  BACK".into())
        }
        Screen::Initials => (
            String::new(),
            String::new(),
            if cfg!(target_arch = "wasm32") {
                "LEFT / RIGHT  SLOT     UP / DOWN  LETTER\nENTER / A  SAVE     BACKSPACE / B  ERASE\nOR TYPE THREE LETTERS.".into()
            } else {
                "LEFT / RIGHT  SLOT     UP / DOWN  LETTER\nENTER / A  SAVE     BACKSPACE / B  ERASE\nOR TYPE THREE LETTERS. STEAM DECK: STEAM + X".into()
            },
        ),
        Screen::Gameplay => unreachable!(),
    };
    let notice = [&front.notice, &store.notice]
        .into_iter()
        .filter(|value| !value.is_empty())
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join("\n");
    for (label, mut text, mut font, mut node) in &mut labels {
        let (value, size) = match label {
            Label::Title => (&title, 44.),
            Label::Body => (&body, 25.),
            Label::Footer => (&footer, 16.),
            Label::Notice => (&notice, 16.),
        };
        node.display = if value.is_empty() {
            Display::None
        } else {
            Display::Flex
        };
        if text.0 != *value {
            text.0.clone_from(value);
        }
        if font.font_size != size * scale {
            font.font_size = size * scale;
        }
    }
}

#[cfg(test)]
mod tests;
