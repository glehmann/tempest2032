//! Clean block-letter outlines, extruded sides, bevels, and fixed studio shading.

use bevy::{asset::RenderAssetUsages, mesh::PrimitiveTopology, prelude::*};
use lyon_tessellation::{
    BuffersBuilder, FillOptions, FillRule, FillTessellator, FillVertex, VertexBuffers, math::point,
    path::Path,
};
use std::collections::{BTreeMap, HashMap};

pub(crate) type Triangle = [(Vec3, Color); 3];
pub(crate) const ADVANCE: f32 = 17.;

#[derive(Component)]
pub(crate) struct OverlayCamera;

pub(crate) type GameCameras = (With<Camera3d>, Without<OverlayCamera>);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Style {
    Chrome,
    Rainbow,
    Gold,
}

#[derive(Clone)]
pub(super) struct Fragment {
    pub triangle: Triangle,
    pub center: Vec2,
}

pub(crate) struct Glyph {
    pub triangles: Vec<Triangle>,
    pub(super) fragments: Vec<Fragment>,
}

#[derive(Resource)]
pub(crate) struct Glyphs(HashMap<(char, Style), Glyph>);

impl Default for Glyphs {
    fn default() -> Self {
        let mut glyphs = HashMap::new();
        for character in " !(),-.:?0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ".chars() {
            let contours = outline(character);
            for style in [Style::Chrome, Style::Rainbow, Style::Gold] {
                let gold_a = vec![
                    polygon(&[
                        (0., 18.),
                        (0., 4.),
                        (4., 0.),
                        (11., 0.),
                        (15., 4.),
                        (15., 18.),
                        (11., 18.),
                        (11., 12.),
                        (4., 12.),
                        (4., 18.),
                    ]),
                    rect(4., 4., 7., 4.),
                ];
                glyphs.insert(
                    (character, style),
                    extrude(
                        if style == Style::Gold && character == 'A' {
                            &gold_a
                        } else {
                            &contours
                        },
                        &Paint::new(style),
                    ),
                );
            }
        }
        Self(glyphs)
    }
}

impl Glyphs {
    pub fn get(&self, character: char, style: Style) -> &Glyph {
        self.0
            .get(&(character.to_ascii_uppercase(), style))
            .unwrap_or(&self.0[&('?', style)])
    }
}

pub(super) struct Paint {
    style: Style,
    pub rows: Vec<Color>,
    pub top: f32,
    pub height: f32,
}

impl Paint {
    pub fn new(style: Style) -> Self {
        Self {
            style,
            rows: Vec::new(),
            top: 0.,
            height: 18.,
        }
    }

    fn front(&self, y: f32) -> Color {
        if !self.rows.is_empty() {
            let p = (y - self.top).clamp(0., self.rows.len() as f32 - 1.);
            return mix(
                self.rows[p as usize],
                self.rows[(p as usize + 1).min(self.rows.len() - 1)],
                p.fract(),
            );
        }
        let stops: &[(f32, [f32; 3])] = match self.style {
            Style::Chrome => &[
                (0., [0.18, 0.40, 0.83]),
                (0.12, [0.76, 0.89, 1.]),
                (0.20, [0.26, 0.56, 0.91]),
                (0.48, [0.29, 0.22, 0.66]),
                (0.72, [0.58, 0.16, 0.66]),
                (1., [0.31, 0.05, 0.40]),
            ],
            Style::Rainbow => &[
                (0., [0.96, 1., 0.02]),
                (0.17, [0.70, 0.93, 0.07]),
                (0.24, [0.98, 1., 0.80]),
                (0.29, [0.40, 0.92, 0.41]),
                (0.65, [0.04, 0.90, 0.91]),
                (0.73, [0.90, 0.99, 1.]),
                (0.82, [0.86, 0.40, 0.60]),
                (1., [0.64, 0.18, 0.38]),
            ],
            Style::Gold => &[(0., [0.64, 0.015, 0.008]), (1., [0.43, 0.01, 0.008])],
        };
        let t = ((y - self.top) / self.height).clamp(0., 1.);
        for pair in stops.windows(2) {
            if t <= pair[1].0 {
                return mix(
                    Color::srgb_from_array(pair[0].1),
                    Color::srgb_from_array(pair[1].1),
                    (t - pair[0].0) / (pair[1].0 - pair[0].0),
                );
            }
        }
        Color::srgb_from_array(stops.last().unwrap().1)
    }

    fn surface(&self, y: f32, normal: Vec3, bevel: bool) -> Color {
        let light = Vec3::new(-0.45, 0.70, 0.85).normalize();
        let diffuse = normal.dot(light).max(0.);
        let base = match (self.style, bevel, self.rows.is_empty()) {
            (Style::Gold, true, _) => Color::srgb(1., 0.73, 0.08),
            (Style::Gold, false, _) => Color::srgb(0.14, 0.23, 0.08),
            (Style::Rainbow, false, _) => Color::srgb(0.64, 0.23, 0.48),
            (Style::Chrome, false, true) => Color::srgb(0.17, 0.06, 0.31),
            _ => self.front(y),
        }
        .to_linear();
        let strength = if bevel {
            0.42 + diffuse * 0.65
        } else {
            0.36 + diffuse * 0.42
        };
        let glint = if bevel { diffuse.powi(14) * 0.65 } else { 0. };
        Color::linear_rgb(
            (base.red * strength + glint).min(1.),
            (base.green * strength + glint).min(1.),
            (base.blue * strength + glint).min(1.),
        )
    }
}

fn mix(a: Color, b: Color, t: f32) -> Color {
    let a = a.to_linear();
    let b = b.to_linear();
    Color::linear_rgba(
        a.red + (b.red - a.red) * t,
        a.green + (b.green - a.green) * t,
        a.blue + (b.blue - a.blue) * t,
        a.alpha + (b.alpha - a.alpha) * t,
    )
}

fn polygon(points: &[(f32, f32)]) -> Vec<Vec2> {
    points.iter().map(|&(x, y)| Vec2::new(x, y)).collect()
}
fn rect(x: f32, y: f32, w: f32, h: f32) -> Vec<Vec2> {
    polygon(&[(x, y), (x + w, y), (x + w, y + h), (x, y + h)])
}
fn ring() -> Vec<Vec2> {
    polygon(&[
        (3., 0.),
        (12., 0.),
        (15., 3.),
        (15., 15.),
        (12., 18.),
        (3., 18.),
        (0., 15.),
        (0., 3.),
    ])
}
fn counter() -> Vec<Vec2> {
    polygon(&[
        (5., 4.),
        (10., 4.),
        (11., 5.),
        (11., 13.),
        (10., 14.),
        (5., 14.),
        (4., 13.),
        (4., 5.),
    ])
}

fn outline(character: char) -> Vec<Vec<Vec2>> {
    let p: &[(f32, f32)] = match character {
        ' ' => return Vec::new(),
        '0' | 'O' => return vec![ring(), counter()],
        '8' | 'B' => return vec![ring(), rect(4., 4., 7., 3.), rect(4., 11., 7., 3.)],
        '1' => &[
            (5., 0.),
            (10., 0.),
            (10., 14.),
            (14., 14.),
            (14., 18.),
            (1., 18.),
            (1., 14.),
            (5., 14.),
            (5., 5.),
            (1., 7.),
            (0., 4.),
        ],
        '2' => &[
            (1., 0.),
            (12., 0.),
            (15., 3.),
            (15., 7.),
            (4., 14.),
            (15., 14.),
            (15., 18.),
            (0., 18.),
            (0., 12.),
            (11., 5.),
            (11., 4.),
            (0., 4.),
        ],
        '3' => &[
            (0., 0.),
            (11., 0.),
            (15., 3.),
            (15., 15.),
            (12., 18.),
            (0., 18.),
            (0., 14.),
            (10., 14.),
            (11., 13.),
            (11., 11.),
            (5., 11.),
            (5., 7.),
            (11., 7.),
            (11., 5.),
            (10., 4.),
            (0., 4.),
        ],
        '4' => {
            return vec![
                polygon(&[
                    (7., 0.),
                    (15., 0.),
                    (15., 18.),
                    (11., 18.),
                    (11., 12.),
                    (0., 12.),
                    (0., 8.),
                ]),
                polygon(&[(4., 8.), (11., 8.), (11., 3.), (9., 3.)]),
            ];
        }
        '5' => &[
            (0., 0.),
            (15., 0.),
            (15., 4.),
            (4., 4.),
            (4., 7.),
            (11., 7.),
            (15., 10.),
            (15., 15.),
            (12., 18.),
            (0., 18.),
            (0., 14.),
            (10., 14.),
            (11., 13.),
            (11., 12.),
            (10., 11.),
            (0., 11.),
        ],
        '6' | '9' => {
            let mut result = vec![
                polygon(&[
                    (4., 0.),
                    (15., 0.),
                    (15., 4.),
                    (5., 4.),
                    (4., 5.),
                    (4., 7.),
                    (12., 7.),
                    (15., 10.),
                    (15., 15.),
                    (12., 18.),
                    (3., 18.),
                    (0., 15.),
                    (0., 4.),
                ]),
                rect(4., 11., 7., 3.),
            ];
            if character == '9' {
                for contour in &mut result {
                    for p in contour {
                        *p = Vec2::new(15., 18.) - *p;
                    }
                }
            }
            return result;
        }
        '7' => &[
            (0., 0.),
            (15., 0.),
            (15., 4.),
            (7., 18.),
            (2., 18.),
            (10., 4.),
            (0., 4.),
        ],
        'A' => {
            return vec![
                polygon(&[
                    (0., 18.),
                    (4., 0.),
                    (11., 0.),
                    (15., 18.),
                    (11.5, 18.),
                    (10.5, 13.),
                    (4.5, 13.),
                    (3.5, 18.),
                ]),
                polygon(&[(5.3, 9.), (9.7, 9.), (8.7, 4.), (6.3, 4.)]),
            ];
        }
        'C' => &[
            (4., 0.),
            (15., 0.),
            (15., 4.),
            (5., 4.),
            (4., 5.),
            (4., 13.),
            (5., 14.),
            (15., 14.),
            (15., 18.),
            (4., 18.),
            (0., 14.),
            (0., 4.),
        ],
        'D' => {
            return vec![
                polygon(&[
                    (0., 0.),
                    (11., 0.),
                    (15., 4.),
                    (15., 14.),
                    (11., 18.),
                    (0., 18.),
                ]),
                counter(),
            ];
        }
        'E' => &[
            (0., 0.),
            (15., 0.),
            (15., 4.),
            (4., 4.),
            (4., 7.),
            (12., 7.),
            (12., 11.),
            (4., 11.),
            (4., 14.),
            (15., 14.),
            (15., 18.),
            (0., 18.),
        ],
        'F' => &[
            (0., 0.),
            (15., 0.),
            (15., 4.),
            (4., 4.),
            (4., 7.),
            (12., 7.),
            (12., 11.),
            (4., 11.),
            (4., 18.),
            (0., 18.),
        ],
        'G' => &[
            (4., 0.),
            (15., 0.),
            (15., 4.),
            (5., 4.),
            (4., 5.),
            (4., 13.),
            (5., 14.),
            (11., 14.),
            (11., 11.),
            (8., 11.),
            (8., 7.),
            (15., 7.),
            (15., 18.),
            (4., 18.),
            (0., 14.),
            (0., 4.),
        ],
        'H' => &[
            (0., 0.),
            (4., 0.),
            (4., 7.),
            (11., 7.),
            (11., 0.),
            (15., 0.),
            (15., 18.),
            (11., 18.),
            (11., 11.),
            (4., 11.),
            (4., 18.),
            (0., 18.),
        ],
        'I' => &[
            (0., 0.),
            (15., 0.),
            (15., 4.),
            (9.5, 4.),
            (9.5, 14.),
            (15., 14.),
            (15., 18.),
            (0., 18.),
            (0., 14.),
            (5.5, 14.),
            (5.5, 4.),
            (0., 4.),
        ],
        'J' => &[
            (4., 0.),
            (15., 0.),
            (15., 14.),
            (11., 18.),
            (4., 18.),
            (0., 14.),
            (0., 10.),
            (4., 10.),
            (4., 13.),
            (5., 14.),
            (10., 14.),
            (11., 13.),
            (11., 4.),
            (4., 4.),
        ],
        'K' => &[
            (0., 0.),
            (4., 0.),
            (4., 7.),
            (10., 0.),
            (15., 0.),
            (8., 9.),
            (15., 18.),
            (10., 18.),
            (4., 11.),
            (4., 18.),
            (0., 18.),
        ],
        'L' => &[
            (0., 0.),
            (4., 0.),
            (4., 14.),
            (15., 14.),
            (15., 18.),
            (0., 18.),
        ],
        'M' => &[
            (0., 0.),
            (4., 0.),
            (7.5, 6.),
            (11., 0.),
            (15., 0.),
            (15., 18.),
            (11., 18.),
            (11., 7.),
            (7.5, 12.),
            (4., 7.),
            (4., 18.),
            (0., 18.),
        ],
        'N' => &[
            (0., 0.),
            (4., 0.),
            (11., 11.),
            (11., 0.),
            (15., 0.),
            (15., 18.),
            (11., 18.),
            (4., 7.),
            (4., 18.),
            (0., 18.),
        ],
        'P' | 'R' => {
            return vec![
                polygon(if character == 'P' {
                    &[
                        (0., 0.),
                        (11., 0.),
                        (15., 4.),
                        (15., 8.),
                        (11., 12.),
                        (4., 12.),
                        (4., 18.),
                        (0., 18.),
                    ]
                } else {
                    &[
                        (0., 0.),
                        (11., 0.),
                        (15., 4.),
                        (15., 8.),
                        (11., 11.),
                        (15., 18.),
                        (10., 18.),
                        (6., 12.),
                        (4., 12.),
                        (4., 18.),
                        (0., 18.),
                    ]
                }),
                rect(4., 4., 7., 4.),
            ];
        }
        'Q' => {
            return vec![
                polygon(&[
                    (3., 0.),
                    (12., 0.),
                    (15., 3.),
                    (15., 14.),
                    (13., 16.),
                    (15., 18.),
                    (10., 18.),
                    (8., 17.),
                    (3., 18.),
                    (0., 15.),
                    (0., 3.),
                ]),
                counter(),
            ];
        }
        'S' => &[
            (3., 0.),
            (15., 0.),
            (15., 4.),
            (5., 4.),
            (4., 5.),
            (4., 7.),
            (11., 7.),
            (15., 11.),
            (15., 15.),
            (12., 18.),
            (0., 18.),
            (0., 14.),
            (10., 14.),
            (11., 13.),
            (11., 12.),
            (10., 11.),
            (4., 11.),
            (0., 7.),
            (0., 3.),
        ],
        'T' => &[
            (0., 0.),
            (15., 0.),
            (15., 4.),
            (9.5, 4.),
            (9.5, 18.),
            (5.5, 18.),
            (5.5, 4.),
            (0., 4.),
        ],
        'U' => &[
            (0., 0.),
            (4., 0.),
            (4., 13.),
            (5., 14.),
            (10., 14.),
            (11., 13.),
            (11., 0.),
            (15., 0.),
            (15., 14.),
            (11., 18.),
            (4., 18.),
            (0., 14.),
        ],
        'V' => &[
            (0., 0.),
            (4., 0.),
            (7.5, 12.),
            (11., 0.),
            (15., 0.),
            (9.5, 18.),
            (5.5, 18.),
        ],
        'W' => &[
            (0., 0.),
            (3.5, 0.),
            (5., 12.),
            (6., 8.),
            (9., 8.),
            (10., 12.),
            (11.5, 0.),
            (15., 0.),
            (12., 18.),
            (8.5, 18.),
            (7.5, 14.),
            (6.5, 18.),
            (3., 18.),
        ],
        'X' => &[
            (0., 0.),
            (5., 0.),
            (7.5, 5.5),
            (10., 0.),
            (15., 0.),
            (10., 9.),
            (15., 18.),
            (10., 18.),
            (7.5, 12.5),
            (5., 18.),
            (0., 18.),
            (5., 9.),
        ],
        'Y' => &[
            (0., 0.),
            (4.5, 0.),
            (7.5, 6.5),
            (10.5, 0.),
            (15., 0.),
            (9.5, 11.),
            (9.5, 18.),
            (5.5, 18.),
            (5.5, 11.),
        ],
        'Z' => &[
            (0., 0.),
            (15., 0.),
            (15., 4.),
            (5., 14.),
            (15., 14.),
            (15., 18.),
            (0., 18.),
            (0., 14.),
            (10., 4.),
            (0., 4.),
        ],
        '!' => return vec![rect(5., 0., 4., 11.), rect(5., 14., 4., 4.)],
        '.' => return vec![rect(5., 14., 4., 4.)],
        ':' => return vec![rect(5., 4., 4., 4.), rect(5., 14., 4., 4.)],
        ',' => &[(5., 14.), (9., 14.), (9., 18.), (5., 20.), (5., 18.)],
        '-' => return vec![rect(2., 7., 11., 4.)],
        '(' => &[
            (10., 0.),
            (5., 0.),
            (1., 4.),
            (1., 14.),
            (5., 18.),
            (10., 18.),
            (10., 14.),
            (6., 14.),
            (5., 13.),
            (5., 5.),
            (6., 4.),
            (10., 4.),
        ],
        ')' => {
            return outline('(')
                .into_iter()
                .map(|p| p.into_iter().map(|p| Vec2::new(15. - p.x, p.y)).collect())
                .collect();
        }
        _ => {
            return vec![
                polygon(&[
                    (0., 0.),
                    (12., 0.),
                    (15., 3.),
                    (15., 7.),
                    (9., 12.),
                    (5., 12.),
                    (5., 9.),
                    (11., 5.),
                    (11., 4.),
                    (0., 4.),
                ]),
                rect(5., 14., 4., 4.),
            ];
        }
    };
    vec![polygon(p)]
}

fn area(contour: &[Vec2]) -> f32 {
    (0..contour.len())
        .map(|i| contour[i].perp_dot(contour[(i + 1) % contour.len()]))
        .sum::<f32>()
        / 2.
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

fn inset(contour: &[Vec2], amount: f32) -> Vec<Vec2> {
    (0..contour.len())
        .map(|i| {
            let a = contour[(i + contour.len() - 1) % contour.len()];
            let p = contour[i];
            let b = contour[(i + 1) % contour.len()];
            let n0 = (p - a).normalize_or_zero().perp();
            let n1 = (b - p).normalize_or_zero().perp();
            let bisector = (n0 + n1).normalize_or_zero();
            p + bisector * (amount / bisector.dot(n1).max(0.35))
        })
        .collect()
}

fn fill(contours: &[Vec<Vec2>]) -> Vec<[Vec2; 3]> {
    let mut path = Path::builder();
    for contour in contours {
        path.begin(point(contour[0].x, contour[0].y));
        for p in &contour[1..] {
            path.line_to(point(p.x, p.y));
        }
        path.end(true);
    }
    let mut geometry = VertexBuffers::<Vec2, u32>::new();
    FillTessellator::new()
        .tessellate_path(
            &path.build(),
            &FillOptions::default().with_fill_rule(FillRule::EvenOdd),
            &mut BuffersBuilder::new(&mut geometry, |v: FillVertex| {
                Vec2::new(v.position().x, v.position().y)
            }),
        )
        .unwrap();
    geometry
        .indices
        .as_chunks::<3>()
        .0
        .iter()
        .map(|face| face.map(|i| geometry.vertices[i as usize]))
        .collect()
}

pub(super) fn extrude(contours: &[Vec<Vec2>], paint: &Paint) -> Glyph {
    if contours.is_empty() {
        return Glyph {
            triangles: Vec::new(),
            fragments: Vec::new(),
        };
    }
    let mut contours = contours.to_vec();
    for i in 0..contours.len() {
        let hole = (0..contours.len())
            .filter(|&j| i != j && contains(&contours[j], contours[i][0]))
            .count()
            % 2
            == 1;
        if (area(&contours[i]) < 0.) != hole {
            contours[i].reverse();
        }
    }
    let front: Vec<_> = contours.iter().map(|p| inset(p, 0.55)).collect();
    let mut triangles = Vec::new();
    for mut face in fill(&contours) {
        if (face[1] - face[0]).perp_dot(face[2] - face[0]) < 0. {
            face.swap(1, 2);
        }
        push(
            &mut triangles,
            face.map(|p| {
                (
                    (p + Vec2::new(0.85, 1.15)).extend(-1.5),
                    paint.surface(p.y, -Vec3::Z, false),
                )
            }),
        );
    }
    for (contour, inner) in contours.iter().zip(&front) {
        for i in 0..contour.len() {
            let j = (i + 1) % contour.len();
            let a = contour[i];
            let b = contour[j];
            for (vertices, bevel) in [
                (
                    [
                        a.extend(0.),
                        b.extend(0.),
                        (b + Vec2::new(0.85, 1.15)).extend(-1.5),
                        (a + Vec2::new(0.85, 1.15)).extend(-1.5),
                    ],
                    false,
                ),
                (
                    [
                        a.extend(0.),
                        b.extend(0.),
                        inner[j].extend(0.42),
                        inner[i].extend(0.42),
                    ],
                    true,
                ),
            ] {
                let faces = if bevel {
                    [[0, 2, 1], [0, 3, 2]]
                } else {
                    [[0, 1, 2], [0, 2, 3]]
                };
                let source_normal = (vertices[faces[0][1]] - vertices[0])
                    .cross(vertices[faces[0][2]] - vertices[0])
                    .normalize_or_zero();
                let normal = Vec3::new(-source_normal.x, source_normal.y, -source_normal.z);
                let corners = vertices.map(|p| (p, paint.surface(p.y, normal, bevel)));
                for face in faces {
                    push(&mut triangles, face.map(|i| corners[i]));
                }
            }
        }
    }
    // Split fronts at analytic colour-band boundaries, not a source bitmap grid.
    let bands = if paint.rows.is_empty() {
        36
    } else {
        paint.rows.len().max(2)
    };
    let lo_y = front
        .iter()
        .flatten()
        .map(|p| p.y)
        .fold(f32::INFINITY, f32::min);
    let hi_y = front
        .iter()
        .flatten()
        .map(|p| p.y)
        .fold(f32::NEG_INFINITY, f32::max);
    for face in fill(&front) {
        for band in 0..bands {
            let lo = lo_y + (hi_y - lo_y) * band as f32 / bands as f32;
            let hi = lo_y + (hi_y - lo_y) * (band + 1) as f32 / bands as f32;
            let mut points: Vec<_> = face
                .iter()
                .map(|p| (p.extend(0.42), paint.front(p.y)))
                .collect();
            points = clip(&points, 1, lo, 1.);
            points = clip(&points, 1, hi, -1.);
            for i in 1..points.len().saturating_sub(1) {
                let a = points[0].0;
                let b = points[i + 1].0;
                let c = points[i].0;
                let mut triangle = [a, b, c].map(|p| (p, paint.front(p.y)));
                if (triangle[1].0 - triangle[0].0)
                    .cross(triangle[2].0 - triangle[0].0)
                    .z
                    > 0.
                {
                    triangle.swap(1, 2);
                }
                push(&mut triangles, triangle);
            }
        }
    }
    let fragments = if paint.style == Style::Chrome && paint.rows.is_empty() {
        fragments(&triangles)
    } else {
        Vec::new()
    };
    Glyph {
        triangles,
        fragments,
    }
}

fn push(triangles: &mut Vec<Triangle>, triangle: Triangle) {
    if (triangle[1].0 - triangle[0].0)
        .cross(triangle[2].0 - triangle[0].0)
        .length_squared()
        > 1e-10
    {
        triangles.push(triangle);
    }
}

fn clip(points: &[(Vec3, Color)], axis: usize, bound: f32, sign: f32) -> Vec<(Vec3, Color)> {
    let mut output = Vec::new();
    for i in 0..points.len() {
        let (a, ca) = points[i];
        let (b, cb) = points[(i + 1) % points.len()];
        let da = (a[axis] - bound) * sign;
        let db = (b[axis] - bound) * sign;
        if da >= 0. {
            output.push((a, ca));
        }
        if (da >= 0.) != (db >= 0.) {
            let t = da / (da - db);
            output.push((a.lerp(b, t), mix(ca, cb, t)));
        }
    }
    output
}

fn fragments(triangles: &[Triangle]) -> Vec<Fragment> {
    let mut output = Vec::new();
    for triangle in triangles {
        let min = triangle
            .iter()
            .map(|(p, _)| p.truncate())
            .fold(Vec2::splat(f32::INFINITY), Vec2::min)
            .floor();
        let max = triangle
            .iter()
            .map(|(p, _)| p.truncate())
            .fold(Vec2::splat(f32::NEG_INFINITY), Vec2::max)
            .ceil();
        for y in min.y as i32..max.y as i32 {
            for x in min.x as i32..max.x as i32 {
                let mut points = triangle.to_vec();
                for (axis, bound, sign) in [
                    (0, x as f32, 1.),
                    (0, x as f32 + 1., -1.),
                    (1, y as f32, 1.),
                    (1, y as f32 + 1., -1.),
                ] {
                    points = clip(&points, axis, bound, sign);
                }
                for i in 1..points.len().saturating_sub(1) {
                    let triangle = [points[0], points[i], points[i + 1]];
                    if (triangle[1].0 - triangle[0].0)
                        .cross(triangle[2].0 - triangle[0].0)
                        .length_squared()
                        > 1e-10
                    {
                        output.push(Fragment {
                            triangle,
                            center: Vec2::new(x as f32 + 0.5, y as f32 + 0.5),
                        });
                    }
                }
            }
        }
    }
    output
}

pub(super) fn contours_from_triangles(triangles: &[[Vec2; 3]]) -> Vec<Vec<Vec2>> {
    let key = |p: Vec2| (p.x.to_bits(), p.y.to_bits());
    let mut edges = BTreeMap::new();
    for triangle in triangles {
        for i in 0..3 {
            let a = triangle[i];
            let b = triangle[(i + 1) % 3];
            let mut pair = [key(a), key(b)];
            pair.sort_unstable();
            if edges.remove(&pair).is_none() {
                edges.insert(pair, (a, b));
            }
        }
    }
    let mut links: BTreeMap<_, Vec<Vec2>> = BTreeMap::new();
    for (_, (a, b)) in edges {
        links.entry(key(a)).or_default().push(b);
    }
    let mut contours = Vec::new();
    while let Some((&start, _)) = links.first_key_value() {
        let mut current = start;
        let mut contour = Vec::new();
        loop {
            contour.push(Vec2::new(
                f32::from_bits(current.0),
                f32::from_bits(current.1),
            ));
            let Some(targets) = links.get_mut(&current) else {
                break;
            };
            let next = targets.pop().unwrap();
            if targets.is_empty() {
                links.remove(&current);
            }
            current = key(next);
            if current == start {
                break;
            }
        }
        if contour.len() >= 3 {
            contours.push(contour);
        }
    }
    contours
}

pub(crate) fn mesh(triangles: Vec<Triangle>) -> Mesh {
    let mut positions = Vec::with_capacity(triangles.len() * 3);
    let mut normals = Vec::with_capacity(triangles.len() * 3);
    let mut colors = Vec::with_capacity(triangles.len() * 3);
    for triangle in triangles {
        let normal = (triangle[1].0 - triangle[0].0)
            .cross(triangle[2].0 - triangle[0].0)
            .normalize_or_zero()
            .to_array();
        for (position, color) in triangle {
            positions.push(position.to_array());
            normals.push(normal);
            colors.push(color.to_linear().to_f32_array());
        }
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
}

#[cfg(test)]
#[path = "../tests/geometry.rs"]
mod tests;
