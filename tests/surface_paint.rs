use adars::{
    document::Document,
    persistence,
    raster::Raster,
    scene::{self, Camera, Primitive, SceneObject},
};
#[test]
fn every_palette_shape_has_visible_geometry_and_bounded_surface_coordinates() {
    for kind in Primitive::ALL {
        let object = SceneObject::new(1, kind, [200, 170, 110, 255]);
        let render = scene::render(
            96,
            96,
            &[object],
            &Camera {
                distance: 3.0,
                ..Camera::default()
            },
            None,
            None,
            false,
        );
        assert!(
            render.object_ids.iter().filter(|id| id.is_some()).count() > 100,
            "{}",
            kind.label()
        );
        assert_eq!(render.surface_uvs.len(), 96 * 96);
        assert!(
            render
                .surface_uvs
                .iter()
                .flatten()
                .all(|v| v.is_finite() && (0.0..=1.0).contains(v))
        );
    }
}
#[test]
fn surface_paint_roundtrips_and_old_projects_still_open() {
    let mut doc = Document::new(16, 16);
    doc.add_object(Primitive::Capsule, [20, 180, 130, 255]);
    doc.objects[0].prepare_paint();
    doc.objects[0]
        .paint
        .as_mut()
        .unwrap()
        .stamp(50.0, 90.0, 15.0, [245, 40, 60, 255], 1.0);
    let path =
        std::env::temp_dir().join(format!("ada-surface-{}.ada3dproject", std::process::id()));
    persistence::save_project(&path, &doc).unwrap();
    assert_eq!(persistence::load_project(&path).unwrap(), doc);
    std::fs::remove_file(path).unwrap();
    let mut value = serde_json::to_value(&doc).unwrap();
    value["objects"][0].as_object_mut().unwrap().remove("paint");
    let old: Document = serde_json::from_value(value).unwrap();
    old.validate().unwrap();
    assert!(old.objects[0].paint.is_none());
}
#[test]
fn texture_fragments_are_local_and_alpha_survives_scene_rendering() {
    let mut object = SceneObject::new(1, Primitive::Sphere, [255; 4]);
    let camera = Camera {
        yaw: 0.0,
        pitch: 0.0,
        distance: 3.0,
    };
    let before = scene::render(96, 96, &[object.clone()], &camera, None, None, false);
    let center = 48 * 96 + 48;
    let uv = before.surface_uvs[center];
    let [x, y] = scene::paint_texel(uv);
    object.prepare_paint();
    let paint = object.paint.as_mut().unwrap();
    paint.stamp(x as f32 + 0.5, y as f32 + 0.5, 8.0, [255, 0, 0, 255], 1.0);
    let painted = scene::render(96, 96, &[object.clone()], &camera, None, None, false);
    let pixel = painted.raster.pixel(48, 48);
    assert!(pixel[0] > 100 && pixel[1] < 10);
    assert!(
        painted
            .raster
            .pixels
            .as_chunks::<4>()
            .0
            .iter()
            .any(|p| p[0] > 100 && p[1] > 100)
    );
    object
        .paint
        .as_mut()
        .unwrap()
        .erase_stamp(x as f32 + 0.5, y as f32 + 0.5, 8.0);
    let erased = scene::render(96, 96, &[object], &camera, None, None, false);
    assert_eq!(erased.raster.pixel(48, 48)[3], 0);
}
#[test]
fn paint_memory_is_counted_and_malformed_textures_are_rejected() {
    let mut doc = Document::new(16, 16);
    doc.add_object(Primitive::Cube, [255; 4]);
    let before = doc.estimated_bytes();
    doc.objects[0].prepare_paint();
    assert_eq!(
        doc.estimated_bytes() - before,
        (scene::PAINT_SIZE * scene::PAINT_SIZE * 4) as usize
    );
    doc.objects[0].paint = Some(Raster::new(1024, 1024, [255; 4]));
    assert!(doc.validate().is_err());
}
