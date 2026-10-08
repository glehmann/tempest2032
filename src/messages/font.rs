//! Rounded vector contours with the original glyph proportions and advance.

use bevy::prelude::*;
use lyon_tessellation::{
    BuffersBuilder, FillOptions, FillRule, FillTessellator, FillVertex, VertexBuffers, math::point,
    path::Path,
};
use std::collections::HashMap;

#[derive(Resource)]
pub(super) struct Font(HashMap<char, Vec<[Vec2; 3]>>);

pub(super) struct GlyphTriangle {
    pub(super) points: [Vec2; 3],
    pub(super) origin: Vec2,
}

pub(super) fn color(y: f32) -> Color {
    let gradient = [0x01, 0x18, 0x26, 0x53, 0x62, 0x71];
    let position = (y / 18. * 5.).clamp(0., 5.);
    let index = position as usize;
    let a = crate::cry::color(gradient[index], 255).to_srgba();
    let b = crate::cry::color(gradient[(index + 1).min(5)], 200).to_srgba();
    let t = position.fract();
    Color::srgb(
        a.red + (b.red - a.red) * t,
        a.green + (b.green - a.green) * t,
        a.blue + (b.blue - a.blue) * t,
    )
}

impl Default for Font {
    fn default() -> Self {
        let mut data = &include_bytes!("../../assets/messages/afont.bin")[4..];
        let mut glyphs = HashMap::new();
        fn word(data: &mut &[u8]) -> u16 {
            let value = u16::from_be_bytes(data[..2].try_into().unwrap());
            *data = &data[2..];
            value
        }
        while !data.is_empty() {
            let character = data[0] as char;
            data = &data[1..];
            let count = word(&mut data);
            let mut path = Path::builder();
            for _ in 0..count {
                let count = word(&mut data);
                let points: Vec<_> = (0..count)
                    .map(|_| {
                        Vec2::new(word(&mut data) as i16 as f32, word(&mut data) as i16 as f32)
                    })
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
                            .with_tolerance(0.02),
                        &mut BuffersBuilder::new(&mut geometry, |vertex: FillVertex| {
                            Vec2::new(vertex.position().x, vertex.position().y)
                        }),
                    )
                    .unwrap();
            }
            glyphs.insert(
                character,
                geometry
                    .indices
                    .as_chunks::<3>()
                    .0
                    .iter()
                    .map(|face| std::array::from_fn(|i| geometry.vertices[face[i] as usize]))
                    .collect(),
            );
        }
        Self(glyphs)
    }
}

impl Font {
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
                            .flatten()
                            .map(move |triangle| GlyphTriangle {
                                points: triangle.map(|p| p - Vec2::new(7.5, 9.)),
                                origin: Vec2::new(
                                    left + 7.5 + index as f32 * 17.,
                                    line as f32 * 20.,
                                ),
                            })
                    })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::ValueEnum;
    #[test]
    fn catalog_letters_are_vector_geometry_with_real_counters() {
        let font = Font::default();
        for id in crate::messages::Id::value_variants() {
            for character in id.text().chars().filter(|c| !c.is_whitespace()) {
                assert!(font.0.contains_key(&character), "{character}");
            }
        }
        assert!(font.0[&'O'].len() > 20);
        assert!(font.0[&' '].is_empty());
        let inside = |p: Vec2, triangle: &[Vec2; 3]| {
            let sides = std::array::from_fn::<_, 3, _>(|i| {
                (triangle[(i + 1) % 3] - triangle[i]).perp_dot(p - triangle[i])
            });
            sides.iter().all(|s| *s >= 0.) || sides.iter().all(|s| *s <= 0.)
        };
        assert!(
            !font.0[&'O'].iter().any(|t| inside(Vec2::new(7.5, 9.), t)),
            "O's counter must stay open"
        );
    }
}
