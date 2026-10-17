//! Colored triangle meshes, transforms for wireframe beam segments, and material tinting.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::PrimitiveTopology;
use bevy::prelude::*;

pub(crate) fn colored_mesh(triangles: impl IntoIterator<Item = [(Vec3, Color); 3]>) -> Mesh {
    let triangles = triangles.into_iter();
    let vertices = triangles.size_hint().0 * 3;
    let mut positions = Vec::with_capacity(vertices);
    let mut normals = Vec::with_capacity(vertices);
    let mut colors = Vec::with_capacity(vertices);
    for triangle in triangles {
        let normal = (triangle[1].0 - triangle[0].0)
            .cross(triangle[2].0 - triangle[0].0)
            .normalize_or_zero();
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

/// Mutable access marks a material for re-preparation, so skip unchanged colors.
pub(crate) fn set_base_color(
    materials: &mut Assets<StandardMaterial>,
    handle: &Handle<StandardMaterial>,
    color: Color,
) {
    if materials.get(handle).is_some_and(|m| m.base_color != color)
        && let Some(material) = materials.get_mut(handle)
    {
        material.base_color = color;
    }
}

pub(crate) fn segment_transform(start: Vec3, end: Vec3, width: f32) -> Transform {
    let direction = end - start;
    Transform::from_translation((start + end) * 0.5)
        .with_rotation(Quat::from_rotation_arc(Vec3::Y, direction.normalize()))
        .with_scale(Vec3::new(width, direction.length(), width))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapsed_triangles_keep_finite_normals() {
        let mesh = colored_mesh([
            [Vec3::ZERO; 3].map(|p| (p, Color::WHITE)),
            [Vec3::ZERO, Vec3::X, Vec3::X * 2.].map(|p| (p, Color::WHITE)),
        ]);
        let bevy::mesh::VertexAttributeValues::Float32x3(normals) =
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL).unwrap()
        else {
            panic!("missing normals")
        };
        assert!(normals.iter().flatten().all(|n| n.is_finite()));
    }
}
