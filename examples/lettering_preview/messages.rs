//! Reuse the game's actual palette and extrusion without importing its app.
#![allow(dead_code)]

#[path = "../../src/messages/catalog.rs"]
mod catalog;
pub(crate) use catalog::Id;
#[path = "../../src/messages/font.rs"]
mod font;
#[path = "../../src/messages/geometry.rs"]
pub(crate) mod geometry;

use bevy::prelude::*;

pub(crate) fn candidate(contours: &[Vec<Vec2>]) -> Vec<geometry::Triangle> {
    geometry::extrude(contours, &geometry::Paint::new(geometry::Style::Chrome)).triangles
}
