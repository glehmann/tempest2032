//! Simplified vector contours with the original glyph proportions and advance.

use bevy::prelude::*;
use lyon_tessellation::{
    BuffersBuilder, FillOptions, FillRule, FillTessellator, FillVertex, VertexBuffers, math::point,
    path::Path,
};
use std::collections::HashMap;

#[derive(Resource)]
pub(super) struct Font(HashMap<char, Glyph>, HashMap<super::Id, WordArt>);

struct Glyph {
    triangles: Vec<CellTriangle>,
    shading: [Color; 15 * 18],
}

struct CellTriangle {
    points: [Vec2; 3],
    center: Vec2,
}

impl Glyph {
    fn color(&self, position: Vec2) -> Color {
        shade(&self.shading, 15, 18, position)
    }
}

fn shade(shading: &[Color], width: usize, height: usize, position: Vec2) -> Color {
    let p = (position - Vec2::splat(0.5)).clamp(
        Vec2::ZERO,
        Vec2::new((width - 1) as f32, (height - 1) as f32),
    );
    let x = p.x as usize;
    let y = p.y as usize;
    let sample = |x: usize, y: usize| shading[y * width + x].to_srgba().to_f32_array();
    let a = sample(x, y);
    let b = sample((x + 1).min(width - 1), y);
    let c = sample(x, (y + 1).min(height - 1));
    let d = sample((x + 1).min(width - 1), (y + 1).min(height - 1));
    let rgb: [f32; 3] = std::array::from_fn(|i| {
        let top = a[i] + (b[i] - a[i]) * p.x.fract();
        let bottom = c[i] + (d[i] - c[i]) * p.x.fract();
        top + (bottom - top) * p.y.fract()
    });
    Color::srgb(rgb[0], rgb[1], rgb[2])
}

struct WordArt {
    size: Vec2,
    triangles: Vec<[Vec2; 3]>,
    colors: Vec<Color>,
    pixels: Vec<u16>,
}

pub(super) struct GlyphTriangle {
    pub(super) points: [Vec2; 3],
    pub(super) origin: Vec2,
    pub(super) colors: [Color; 3],
    pub(super) cell: Vec2,
}

impl Default for Font {
    fn default() -> Self {
        assert_eq!(
            &include_bytes!("../../assets/messages/afont.bin")[..4],
            b"T2KS"
        );
        let mut data = &include_bytes!("../../assets/messages/afont.bin")[4..];
        let mut glyphs = HashMap::new();
        while !data.is_empty() {
            let character = data[0] as char;
            data = &data[1..];
            let mut triangles = Vec::new();
            for triangle in decode(&mut data, 0.02) {
                split_cells(triangle, &mut triangles);
            }
            let shading = std::array::from_fn(|_| {
                let raw = word(&mut data);
                crate::cry::color((raw >> 8) as u8, raw as u8)
            });
            glyphs.insert(character, Glyph { triangles, shading });
        }
        assert_eq!(
            &include_bytes!("../../assets/messages/wordart.bin")[..4],
            b"T2KC"
        );
        let mut data = &include_bytes!("../../assets/messages/wordart.bin")[4..];
        let mut art = HashMap::new();
        let ids = [
            super::Id::Excellent,
            super::Id::Points2000,
            super::Id::GameOver,
            super::Id::OneUp,
            super::Id::Points250,
            super::Id::Points500,
            super::Id::Points750,
        ];
        let mut numeral = None;
        while !data.is_empty() {
            let key = data[0] as usize;
            data = &data[1..];
            let size = Vec2::new(f32::from(word(&mut data)), f32::from(word(&mut data)));
            let colors = (0..size.y as usize)
                .map(|_| {
                    let raw = word(&mut data);
                    crate::cry::color((raw >> 8) as u8, raw as u8)
                })
                .collect();
            let triangles = decode(&mut data, 0.002);
            let pixels = if key == 0 || key == 1 {
                (0..(size.x * size.y) as usize)
                    .map(|_| word(&mut data))
                    .collect()
            } else {
                Vec::new()
            };
            let graphic = WordArt {
                size,
                colors,
                triangles,
                pixels,
            };
            if key == 7 {
                numeral = Some(graphic);
            } else {
                art.insert(ids[key], graphic);
            }
        }
        // The original badge draws UP and a separate, half-sized numeral above it.
        if let Some(numeral) = numeral {
            let badge = art.get_mut(&super::Id::OneUp).unwrap();
            badge
                .triangles
                .extend(numeral.triangles.iter().map(|triangle| {
                    triangle.map(|p| {
                        (p - numeral.size / 2.) * 0.5 + badge.size / 2. + Vec2::new(0., -18.)
                    })
                }));
        }
        Self(glyphs, art)
    }
}

impl Font {
    pub(super) fn solid_art(&self, id: super::Id) -> (Vec<[Vec2; 3]>, Vec<Color>, Vec2) {
        let art = &self.1[&id];
        (
            art.triangles
                .iter()
                .map(|t| t.map(|p| p - art.size / 2.))
                .collect(),
            art.colors.clone(),
            art.size,
        )
    }
    pub(super) fn layout(&self, id: super::Id) -> Vec<GlyphTriangle> {
        self.1.get(&id).map_or_else(
            || self.triangles(id.text()),
            |art| {
                if matches!(id, super::Id::Points2000 | super::Id::Excellent) {
                    return art
                        .pixels
                        .iter()
                        .enumerate()
                        .filter(|(_, pixel)| **pixel & 255 > 80)
                        .flat_map(|(index, pixel)| {
                            let origin = Vec2::new(
                                (index % art.size.x as usize) as f32,
                                (index / art.size.x as usize) as f32,
                            ) - art.size / 2.;
                            let corners =
                                [Vec2::ZERO, Vec2::X, Vec2::ONE, Vec2::Y].map(|p| p + origin);
                            let color = Color::LinearRgba(
                                crate::cry::color((pixel >> 8) as u8, *pixel as u8).to_linear(),
                            );
                            [
                                [corners[0], corners[1], corners[2]],
                                [corners[0], corners[2], corners[3]],
                            ]
                            .map(|points| GlyphTriangle {
                                points,
                                origin: Vec2::ZERO,
                                colors: [color; 3],
                                cell: origin + Vec2::splat(0.5),
                            })
                        })
                        .collect();
                }
                art.triangles
                    .iter()
                    .map(|triangle| GlyphTriangle {
                        points: triangle.map(|p| p - art.size / 2.),
                        origin: Vec2::ZERO,
                        colors: triangle.map(|p| {
                            Color::LinearRgba(
                                art.colors[(p.y as usize).min(art.colors.len() - 1)].to_linear(),
                            )
                        }),
                        cell: Vec2::ZERO,
                    })
                    .collect()
            },
        )
    }
    pub(super) fn triangles(&self, text: &str) -> Vec<GlyphTriangle> {
        text.lines()
            .enumerate()
            .flat_map(|(line, text)| {
                let left = -(text.chars().count() as f32) * 17. / 2.;
                text.chars()
                    .enumerate()
                    .flat_map(move |(index, character)| {
                        self.0
                            .get(&character.to_ascii_uppercase())
                            .into_iter()
                            .flat_map(move |glyph| {
                                glyph.triangles.iter().map(move |triangle| GlyphTriangle {
                                    points: triangle.points.map(|p| p - Vec2::new(7.5, 9.)),
                                    colors: triangle
                                        .points
                                        .map(|p| Color::LinearRgba(glyph.color(p).to_linear())),
                                    cell: triangle.center - Vec2::new(7.5, 9.),
                                    origin: Vec2::new(
                                        left + 7.5 + index as f32 * 17.,
                                        line as f32 * 20.,
                                    ),
                                })
                            })
                    })
            })
            .collect()
    }
}

#[cfg(test)]
pub(super) fn original_contours() -> HashMap<char, Vec<Vec<Vec2>>> {
    let mut data = &include_bytes!("../../assets/messages/afont.bin")[4..];
    let mut glyphs = HashMap::new();
    while !data.is_empty() {
        let character = data[0] as char;
        data = &data[1..];
        let contours = (0..word(&mut data))
            .map(|_| {
                let points: Vec<_> = (0..word(&mut data))
                    .map(|_| {
                        Vec2::new(word(&mut data) as i16 as f32, word(&mut data) as i16 as f32)
                    })
                    .collect();
                simplify_contour(&points)
            })
            .collect();
        data = &data[15 * 18 * 2..];
        glyphs.insert(character, contours);
    }
    glyphs
}

// Clip the smooth outline into fixed source-pixel cells, once at startup.
fn split_cells(triangle: [Vec2; 3], output: &mut Vec<CellTriangle>) {
    let min = triangle
        .iter()
        .copied()
        .fold(Vec2::splat(f32::INFINITY), Vec2::min)
        .floor();
    let max = triangle
        .iter()
        .copied()
        .fold(Vec2::splat(f32::NEG_INFINITY), Vec2::max)
        .ceil();
    for y in min.y as i32..max.y as i32 {
        for x in min.x as i32..max.x as i32 {
            let center = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
            let mut polygon = triangle.to_vec();
            for (axis, bound, side) in [
                (0, x as f32, 1.),
                (0, x as f32 + 1., -1.),
                (1, y as f32, 1.),
                (1, y as f32 + 1., -1.),
            ] {
                let mut clipped = Vec::new();
                for i in 0..polygon.len() {
                    let a = polygon[i];
                    let b = polygon[(i + 1) % polygon.len()];
                    let inside_a = (a[axis] - bound) * side >= 0.;
                    let inside_b = (b[axis] - bound) * side >= 0.;
                    if inside_a {
                        clipped.push(a);
                    }
                    if inside_a != inside_b {
                        clipped.push(a + (b - a) * ((bound - a[axis]) / (b[axis] - a[axis])));
                    }
                }
                polygon = clipped;
            }
            for i in 1..polygon.len().saturating_sub(1) {
                let points = [polygon[0], polygon[i], polygon[i + 1]];
                if (points[1] - points[0])
                    .perp_dot(points[2] - points[0])
                    .abs()
                    > 1e-7
                {
                    output.push(CellTriangle { points, center });
                }
            }
        }
    }
}

fn word(data: &mut &[u8]) -> u16 {
    let value = u16::from_be_bytes(data[..2].try_into().unwrap());
    *data = &data[2..];
    value
}

fn simplify_contour(points: &[Vec2]) -> Vec<Vec2> {
    fn distance(p: Vec2, a: Vec2, b: Vec2) -> f32 {
        let edge = b - a;
        let t = ((p - a).dot(edge) / edge.length_squared().max(f32::EPSILON)).clamp(0., 1.);
        p.distance_squared(a + edge * t)
    }
    fn segment(points: &[Vec2], output: &mut Vec<Vec2>) {
        let a = points[0];
        let b = points[points.len() - 1];
        let farthest = (1..points.len() - 1)
            .map(|i| (i, distance(points[i], a, b)))
            .max_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((i, _)) = farthest.filter(|(_, distance)| *distance > 0.8 * 0.8) {
            segment(&points[..=i], output);
            segment(&points[i..], output);
        } else {
            output.push(a);
        }
    }
    // Split the closed loop before simplifying so its seam cannot erase a corner.
    let split = (1..points.len())
        .max_by(|&a, &b| {
            points[a]
                .distance_squared(points[0])
                .total_cmp(&points[b].distance_squared(points[0]))
        })
        .unwrap();
    let mut result = Vec::new();
    segment(&points[..=split], &mut result);
    let mut remaining = points[split..].to_vec();
    remaining.push(points[0]);
    segment(&remaining, &mut result);
    // The split endpoints can land on a pixel step rather than a real corner.
    while result.len() > 3 {
        let redundant = (0..result.len()).find(|&i| {
            distance(
                result[i],
                result[(i + result.len() - 1) % result.len()],
                result[(i + 1) % result.len()],
            ) <= 0.8 * 0.8
        });
        let Some(i) = redundant else { break };
        result.remove(i);
    }
    if result.len() < 3 {
        points.to_vec()
    } else {
        result
    }
}

fn decode(data: &mut &[u8], tolerance: f32) -> Vec<[Vec2; 3]> {
    let count = word(data);
    let mut path = Path::builder();
    for _ in 0..count {
        let points: Vec<_> = (0..word(data))
            .map(|_| Vec2::new(word(data) as i16 as f32, word(data) as i16 as f32))
            .collect();
        let points = simplify_contour(&points);
        // Round actual corners locally, without bending the long fitted diagonals.
        let corner = |i: usize| {
            let p = points[i];
            let before = points[(i + points.len() - 1) % points.len()] - p;
            let after = points[(i + 1) % points.len()] - p;
            let radius = 0.75_f32.min(before.length() / 2.).min(after.length() / 2.);
            (
                p + before.normalize_or_zero() * radius,
                p + after.normalize_or_zero() * radius,
            )
        };
        let start = corner(points.len() - 1).1;
        path.begin(point(start.x, start.y));
        for (i, control) in points.iter().enumerate() {
            let (begin, end) = corner(i);
            path.line_to(point(begin.x, begin.y));
            path.quadratic_bezier_to(point(control.x, control.y), point(end.x, end.y));
        }
        path.end(true);
    }
    let mut geometry = VertexBuffers::<Vec2, u32>::new();
    if count > 0 {
        FillTessellator::new()
            .tessellate_path(
                &path.build(),
                &FillOptions::default()
                    .with_fill_rule(FillRule::EvenOdd)
                    .with_tolerance(tolerance),
                &mut BuffersBuilder::new(&mut geometry, |v: FillVertex| {
                    Vec2::new(v.position().x, v.position().y)
                }),
            )
            .unwrap();
    }
    geometry
        .indices
        .as_chunks::<3>()
        .0
        .iter()
        .map(|face| std::array::from_fn(|i| geometry.vertices[face[i] as usize]))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::ValueEnum;
    #[test]
    fn pixel_staircase_becomes_a_straight_diagonal_without_erasing_corners() {
        let mut outline = vec![Vec2::ZERO, Vec2::new(12., 0.)];
        for i in 0..12 {
            outline.push(Vec2::new(12. - i as f32, i as f32 + 1.));
            outline.push(Vec2::new(11. - i as f32, i as f32 + 1.));
        }
        let simplified = simplify_contour(&outline);
        assert_eq!(simplified.len(), 3);
        assert!(simplified.contains(&Vec2::ZERO));
        assert!(simplified.contains(&Vec2::new(12., 0.)));
        assert!(simplified.contains(&Vec2::new(0., 12.)));
        let small = [Vec2::ZERO, Vec2::X, Vec2::ONE, Vec2::Y];
        assert_eq!(simplify_contour(&small), small);
    }
    #[test]
    fn clipping_preserves_the_outline_area_without_duplicate_cell_edges() {
        let mut cells = Vec::new();
        split_cells(
            [Vec2::ZERO, Vec2::new(2., 0.), Vec2::new(0., 2.)],
            &mut cells,
        );
        let area: f32 = cells
            .iter()
            .map(|cell| {
                (cell.points[1] - cell.points[0])
                    .perp_dot(cell.points[2] - cell.points[0])
                    .abs()
                    / 2.
            })
            .sum();
        assert!((area - 2.).abs() < 1e-6);
        assert!(cells.iter().all(|cell| cell.center.x + cell.center.y <= 2.));
    }
    #[test]
    fn original_shading_retains_white_reflections_and_local_bevel_contrast() {
        let font = Font::default();
        let layout = font.triangles("COLLECT POWERUPS!");
        assert!(
            layout.iter().flat_map(|glyph| glyph.colors).any(|color| {
                let color = color.to_srgba();
                color.red > 0.7 && color.green > 0.7 && color.blue > 0.7
            }),
            "visible lettering should contain bright neutral reflections"
        );
        let a = &font.0[&'A'];
        assert_ne!(
            a.color(Vec2::new(2.5, 3.5)),
            a.color(Vec2::new(7.5, 3.5)),
            "bevel shading varies across a row, not only with height"
        );
        assert!(a.triangles.iter().all(|triangle| {
            triangle
                .points
                .iter()
                .all(|p| (p - triangle.center).abs().max_element() <= 0.501)
        }));
    }
    #[test]
    fn catalog_letters_are_vector_geometry_with_real_counters() {
        let font = Font::default();
        for id in crate::messages::Id::value_variants() {
            for character in id.text().chars().filter(|c| !c.is_whitespace()) {
                assert!(font.0.contains_key(&character), "{character}");
            }
        }
        assert!(font.0[&'O'].triangles.len() > 20);
        assert!(font.0[&' '].triangles.is_empty());
        assert!(
            font.1[&crate::messages::Id::OneUp]
                .triangles
                .iter()
                .flatten()
                .any(|p| p.y < 0.),
            "the separate 1 must accompany UP"
        );
        let inside = |p: Vec2, triangle: &[Vec2; 3]| {
            let sides = std::array::from_fn::<_, 3, _>(|i| {
                (triangle[(i + 1) % 3] - triangle[i]).perp_dot(p - triangle[i])
            });
            sides.iter().all(|s| *s >= 0.) || sides.iter().all(|s| *s <= 0.)
        };
        assert!(
            !font.0[&'O']
                .triangles
                .iter()
                .any(|t| inside(Vec2::new(7.5, 9.), &t.points)),
            "O's counter must stay open"
        );
        assert!(
            !font.0[&'A']
                .triangles
                .iter()
                .any(|t| inside(Vec2::new(7.5, 6.5), &t.points)),
            "A's counter must stay open"
        );
    }
}
