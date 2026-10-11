use super::*;
use clap::ValueEnum;

#[test]
fn all_catalog_letters_have_opaque_finite_beveled_geometry_with_real_depth() {
    let glyphs = Glyphs::default();
    for id in crate::messages::Id::value_variants() {
        for c in id.text().chars().filter(|c| !c.is_whitespace()) {
            assert!(glyphs.0.contains_key(&(c, Style::Chrome)), "missing {c}");
            for style in [Style::Chrome, Style::Rainbow, Style::Gold] {
                let glyph = glyphs.get(c, style);
                assert!(!glyph.triangles.is_empty(), "empty {c:?}/{style:?}");
                assert!(
                    glyph
                        .triangles
                        .iter()
                        .flatten()
                        .all(|(p, color)| p.is_finite() && color.to_linear().alpha == 1.)
                );
                assert!(glyph.triangles.iter().flatten().any(|(p, _)| p.z > 0.));
                assert!(glyph.triangles.iter().flatten().any(|(p, _)| p.z < -1.));
            }
        }
    }
    assert!(glyphs.get(' ', Style::Chrome).triangles.is_empty());
}

#[test]
fn digit_and_letter_counters_stay_open_after_beveling() {
    let glyphs = Glyphs::default();
    let inside = |p: Vec2, t: &Triangle| {
        let signs: Vec<_> = (0..3)
            .map(|i| {
                (t[(i + 1) % 3].0 - t[i].0)
                    .truncate()
                    .perp_dot(p - t[i].0.truncate())
            })
            .collect();
        signs.iter().all(|&v| v >= 0.) || signs.iter().all(|&v| v <= 0.)
    };
    for (c, p) in [
        ('0', Vec2::new(7.5, 9.)),
        ('O', Vec2::new(7.5, 9.)),
        ('A', Vec2::new(7.5, 6.)),
        ('8', Vec2::new(7.5, 5.5)),
        ('8', Vec2::new(7.5, 12.5)),
        ('P', Vec2::new(7.5, 6.)),
    ] {
        let glyph = glyphs.get(c, Style::Chrome);
        assert!(
            !glyph
                .triangles
                .iter()
                .filter(|t| t.iter().all(|(p, _)| p.z > 0.4))
                .any(|t| inside(p, t)),
            "filled counter in {c}"
        );
    }
}

#[test]
fn fragmentation_preserves_surface_area_depth_and_fixed_cell_bounds() {
    let glyphs = Glyphs::default();
    let glyph = glyphs.get('A', Style::Chrome);
    let area = |t: &Triangle| (t[1].0 - t[0].0).cross(t[2].0 - t[0].0).length() / 2.;
    let whole: f32 = glyph.triangles.iter().map(area).sum();
    let pieces: f32 = glyph.fragments.iter().map(|f| area(&f.triangle)).sum();
    assert!((whole - pieces).abs() < whole * 0.0001);
    assert!(glyph.fragments.iter().all(|f| {
        f.triangle
            .iter()
            .all(|(p, _)| (p.truncate() - f.center).abs().max_element() <= 0.501)
    }));
    assert!(
        glyph
            .fragments
            .iter()
            .flat_map(|f| f.triangle)
            .any(|(p, _)| p.z < 0.)
    );
    assert!(
        glyph
            .fragments
            .iter()
            .flat_map(|f| f.triangle)
            .any(|(p, _)| p.z > 0.)
    );
}

#[test]
fn opaque_mesh_has_finite_normals_and_has_no_low_resolution_texture_coordinates() {
    use bevy::mesh::VertexAttributeValues;
    let glyphs = Glyphs::default();
    let mesh = mesh(
        glyphs
            .get('0', Style::Rainbow)
            .triangles
            .iter()
            .map(|t| t.map(|(p, c)| (Vec3::new(p.x, -p.y, p.z), c)))
            .collect(),
    );
    let Some(VertexAttributeValues::Float32x3(normals)) = mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
    else {
        panic!()
    };
    assert!(normals.iter().all(|p| Vec3::from_array(*p).is_finite()));
    assert!(normals.iter().any(|p| p[2] > 0.99));
    assert!(normals.iter().any(|p| p[2] < -0.99));
    assert!(normals.iter().any(|p| p[0].abs() > 0.5));
    assert!(mesh.attribute(Mesh::ATTRIBUTE_UV_0).is_none());
}
