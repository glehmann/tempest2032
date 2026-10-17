use super::*;

#[test]
fn opaque_cells_keep_their_size_and_open_real_gaps() {
    let quad = |center: Vec2, scale| {
        [
            Vec2::new(-0.5, -0.5),
            Vec2::new(0.5, -0.5),
            Vec2::new(0.5, 0.5),
            Vec2::new(-0.5, 0.5),
        ]
        .map(|offset| cell_point(center + offset, center, scale, 0.))
    };
    for factor in [1., 1.5, 2., 4., 26.] {
        let scale = Vec2::new(1., factor);
        let a = quad(Vec2::new(0.5, 0.5), scale);
        let b = quad(Vec2::new(0.5, 1.5), scale);
        assert_eq!(a[2].y - a[1].y, 1.);
        assert_eq!(a[1].x - a[0].x, 1.);
        assert!((b[0].y - a[3].y - (factor - 1.)).abs() < 1e-6);
    }
    let p = Vec2::new(3.2, -2.6);
    let result = cell_point(p, Vec2::new(3.5, -2.5), Vec2::ONE, 0.75);
    assert!(result.distance(Vec2::new(p.x + p.y * 0.75, p.y - p.x * 0.375)) < 1e-6);
    let p = Vec2::new(4.1, -2.2);
    assert!(cell_point(p, Vec2::new(4.5, -2.5), Vec2::splat(0.5), 0.).distance(p * 0.5) < 1e-6);
}

#[test]
fn approaching_reward_wordart_keeps_its_cells_and_area_as_it_grows() {
    let font = Font::default();
    for (id, width, height) in [
        (super::super::Id::Points2000, 197, 65),
        (super::super::Id::Excellent, 178, 42),
        (super::super::Id::OneUp, 50, 45),
        (super::super::Id::Points250, 51, 18),
        (super::super::Id::Points500, 51, 18),
        (super::super::Id::Points750, 51, 18),
    ] {
        let layout = font.layout(id);
        assert!(!layout.is_empty(), "{id:?} lost its source pixels");
        assert!(
            layout.len() <= width * height * 8,
            "{id:?} cell layout grew to {} triangles",
            layout.len()
        );
        for glyph in layout {
            assert!(
                glyph
                    .points
                    .iter()
                    .all(|p| (*p - glyph.cell).abs().max_element() <= 0.501)
            );
            let area =
                (glyph.points[1] - glyph.points[0]).perp_dot(glyph.points[2] - glyph.points[0]);
            let points = glyph
                .points
                .map(|p| cell_point(p, glyph.cell, Vec2::splat(8.), 0.));
            let expanded_area = (points[1] - points[0]).perp_dot(points[2] - points[0]);
            assert!((area - expanded_area).abs() < 1e-4);
        }
    }
}
