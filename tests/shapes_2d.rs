use adars::raster::{Raster, ShapeKind};
#[test]
fn every_shape_draws_finite_rgba_and_reversed_bounds_match() {
    for kind in ShapeKind::ALL {
        for filled in [false, true] {
            let mut a = Raster::new(80, 80, [0; 4]);
            a.draw_shape(
                kind,
                [12.0, 12.0],
                [68.0, 68.0],
                [30, 90, 180, 128],
                3.0,
                filled,
            );
            assert!(
                a.pixels.as_chunks::<4>().0.iter().any(|p| p[3] > 0),
                "{}",
                kind.label()
            );
            assert!(
                a.pixels.as_chunks::<4>().0.iter().all(|p| p[3] <= 128),
                "overlapping alpha: {}",
                kind.label()
            );
            assert_eq!(a.pixel(0, 0), [0; 4]);
            let mut b = Raster::new(80, 80, [0; 4]);
            b.draw_shape(
                kind,
                [68.0, 68.0],
                [12.0, 12.0],
                [30, 90, 180, 128],
                3.0,
                filled,
            );
            assert_eq!(a, b, "reversed bounds: {}", kind.label());
        }
    }
}
#[test]
fn concave_shapes_keep_cutouts_and_outlines_keep_the_center_clear() {
    for kind in [
        ShapeKind::Star,
        ShapeKind::Arrow,
        ShapeKind::Cross,
        ShapeKind::Heart,
    ] {
        let mut a = Raster::new(80, 80, [0; 4]);
        a.draw_shape(kind, [10.0, 10.0], [70.0, 70.0], [255; 4], 2.0, true);
        assert_eq!(a.pixel(11, 11), [0; 4], "{}", kind.label());
        assert!(a.pixel(40, 40)[3] > 0, "{}", kind.label());
        let mut outline = Raster::new(80, 80, [0; 4]);
        outline.draw_shape(kind, [10.0, 10.0], [70.0, 70.0], [255; 4], 2.0, false);
        assert_eq!(outline.pixel(40, 40), [0; 4], "{}", kind.label());
        let before = outline.clone();
        outline.draw_shape(kind, [f32::NAN, 0.0], [20.0, 20.0], [255; 4], 2.0, true);
        assert_eq!(outline, before);
    }
}
