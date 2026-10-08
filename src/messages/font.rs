//! Rounded vector contours with the original glyph proportions and advance.

use bevy::prelude::*;
use lyon_tessellation::{
    BuffersBuilder, FillOptions, FillRule, FillTessellator, FillVertex, VertexBuffers, math::point,
    path::Path,
};
use std::collections::HashMap;

#[derive(Resource)]
pub(super) struct Font(HashMap<char, Glyph>, HashMap<super::Id, WordArt>);

struct Glyph {
    triangles: Vec<[Vec2; 3]>,
    shading: [Color; 15 * 18],
}

impl Glyph {
    fn color(&self, position: Vec2) -> Color {
        let p = (position - Vec2::splat(0.5)).clamp(Vec2::ZERO, Vec2::new(14., 17.));
        let x = p.x as usize;
        let y = p.y as usize;
        let sample = |x: usize, y: usize| self.shading[y * 15 + x].to_srgba().to_f32_array();
        let a = sample(x, y);
        let b = sample((x + 1).min(14), y);
        let c = sample(x, (y + 1).min(17));
        let d = sample((x + 1).min(14), (y + 1).min(17));
        let rgb: [f32; 3] = std::array::from_fn(|i| {
            let top = a[i] + (b[i] - a[i]) * p.x.fract();
            let bottom = c[i] + (d[i] - c[i]) * p.x.fract();
            top + (bottom - top) * p.y.fract()
        });
        Color::srgb(rgb[0], rgb[1], rgb[2])
    }
}

struct WordArt {
    size: Vec2,
    triangles: Vec<[Vec2; 3]>,
    colors: Vec<Color>,
}

pub(super) struct GlyphTriangle {
    pub(super) points: [Vec2; 3],
    pub(super) origin: Vec2,
    pub(super) colors: [Color; 3],
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
            for triangle in decode(&mut data) {
                subdivide(triangle, &mut triangles);
            }
            let shading = std::array::from_fn(|_| {
                let raw = word(&mut data);
                crate::cry::color((raw >> 8) as u8, raw as u8)
            });
            glyphs.insert(character, Glyph { triangles, shading });
        }
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
            let graphic = WordArt {
                size,
                colors,
                triangles: decode(&mut data),
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
    pub(super) fn width(&self, id: super::Id) -> f32 {
        self.1.get(&id).map_or_else(
            || id.text().lines().map(|l| l.len()).max().unwrap_or(0) as f32 * 17.,
            |art| art.size.x,
        )
    }
    pub(super) fn layout(&self, id: super::Id) -> Vec<GlyphTriangle> {
        self.1.get(&id).map_or_else(
            || self.triangles(id.text()),
            |art| {
                art.triangles
                    .iter()
                    .map(|triangle| GlyphTriangle {
                        points: triangle.map(|p| p - art.size / 2.),
                        origin: Vec2::ZERO,
                        colors: triangle
                            .map(|p| art.colors[(p.y as usize).min(art.colors.len() - 1)]),
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
                                    points: triangle.map(|p| p - Vec2::new(7.5, 9.)),
                                    colors: triangle.map(|p| glyph.color(p)),
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

// Interior vertices preserve the bitmap's local bevel lighting on smooth outlines.
fn subdivide(triangle: [Vec2; 3], output: &mut Vec<[Vec2; 3]>) {
    let edge = (0..3)
        .max_by(|a, b| {
            triangle[*a]
                .distance_squared(triangle[(*a + 1) % 3])
                .total_cmp(&triangle[*b].distance_squared(triangle[(*b + 1) % 3]))
        })
        .unwrap();
    let a = triangle[edge];
    let b = triangle[(edge + 1) % 3];
    let c = triangle[(edge + 2) % 3];
    if a.distance_squared(b) <= 0.75 * 0.75 {
        output.push(triangle);
        return;
    }
    let midpoint = (a + b) / 2.;
    subdivide([a, midpoint, c], output);
    subdivide([midpoint, b, c], output);
}

fn word(data: &mut &[u8]) -> u16 {
    let value = u16::from_be_bytes(data[..2].try_into().unwrap());
    *data = &data[2..];
    value
}

fn decode(data: &mut &[u8]) -> Vec<[Vec2; 3]> {
    let count = word(data);
    let mut path = Path::builder();
    for _ in 0..count {
        let points: Vec<_> = (0..word(data))
            .map(|_| Vec2::new(word(data) as i16 as f32, word(data) as i16 as f32))
            .collect();
        let midpoint = (points[0] + points[points.len() - 1]) / 2.;
        path.begin(point(midpoint.x, midpoint.y));
        for (i, control) in points.iter().enumerate() {
            let end = (*control + points[(i + 1) % points.len()]) / 2.;
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
                    .with_tolerance(0.002),
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
        assert!(
            a.triangles.iter().all(
                |triangle| (0..3).all(|i| triangle[i].distance(triangle[(i + 1) % 3]) <= 0.751)
            )
        );
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
                .any(|t| inside(Vec2::new(7.5, 9.), t)),
            "O's counter must stay open"
        );
        assert!(
            !font.0[&'A']
                .triangles
                .iter()
                .any(|t| inside(Vec2::new(7.5, 6.5), t)),
            "A's counter must stay open"
        );
    }
}
