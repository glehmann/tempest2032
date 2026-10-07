//! Colored triangle meshes and transforms for wireframe beam segments.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::PrimitiveTopology;
use bevy::prelude::*;

pub(crate) fn colored_mesh(triangles: impl IntoIterator<Item = [(Vec3, Color); 3]>) -> Mesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut colors = Vec::new();
    for triangle in triangles {
        let normal = (triangle[1].0 - triangle[0].0)
            .cross(triangle[2].0 - triangle[0].0)
            .normalize();
        for (position, color) in triangle {
            positions.push(position.to_array());
            normals.push(normal.to_array());
            colors.push(LinearRgba::from(color).to_f32_array());
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

pub(crate) fn segment_transform(start: Vec3, end: Vec3, width: f32) -> Transform {
    let direction = end - start;
    Transform::from_translation((start + end) * 0.5)
        .with_rotation(Quat::from_rotation_arc(Vec3::Y, direction.normalize()))
        .with_scale(Vec3::new(width, direction.length(), width))
}
