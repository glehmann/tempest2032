//! Approved rounded announcement fronts, shared with the typography lab.

use bevy::prelude::*;
use lyon_tessellation::{
    math::point,
    path::{Path as LyonPath, iterator::PathIterator},
};
use serde::Deserialize;
use std::collections::HashMap;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub(super) fn contours() -> HashMap<char, Vec<Vec<Vec2>>> {
    #[derive(Deserialize)]
    struct Font {
        inset: f32,
        glyphs: HashMap<char, String>,
    }
    let font: Font = toml::from_str(include_str!("../../assets/messages/rounded.toml")).unwrap();
    font.glyphs
        .into_iter()
        .map(|(c, path)| {
            let mut contours = svg_contours(&path).unwrap();
            thin(&mut contours, font.inset);
            (c, contours)
        })
        .collect()
}

pub(crate) fn svg_contours(text: &str) -> Result<Vec<Vec<Vec2>>> {
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

pub(crate) fn thin(contours: &mut [Vec<Vec2>], amount: f32) {
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
