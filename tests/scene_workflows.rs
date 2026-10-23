use adars::raster::Raster;
use adars::scene::{Camera, Primitive, SceneObject, export_obj, render};

fn front_camera() -> Camera {
    Camera {
        yaw: 0.0,
        pitch: 0.0,
        distance: 7.0,
    }
}

fn center_id(scene: &adars::scene::RenderedScene) -> Option<u64> {
    let raster = &scene.raster;
    scene.object_ids[((raster.height / 2) * raster.width + raster.width / 2) as usize]
}

#[test]
fn all_primitives_render_visible_pickable_geometry() {
    for primitive in [
        Primitive::Cube,
        Primitive::Sphere,
        Primitive::Cylinder,
        Primitive::Cone,
    ] {
        let object = SceneObject::new(4, primitive, [215, 117, 64, 255]);
        let result = render(128, 96, &[object], &Camera::default(), None, None, false);
        assert_eq!(result.raster.pixels.len(), 128 * 96 * 4);
        assert_eq!(result.object_ids.len(), 128 * 96);
        assert!(
            result.object_ids.contains(&Some(4)),
            "{} was invisible",
            primitive.label()
        );
        assert!(
            result
                .raster
                .pixels
                .as_chunks::<4>()
                .0
                .iter()
                .any(|p| p[3] != 0)
        );
        assert_eq!(
            result.raster.pixels[3], 0,
            "empty canvas must remain transparent"
        );
    }
}

#[test]
fn picking_uses_nearest_surface_independent_of_submission_order() {
    let mut front = SceneObject::new(11, Primitive::Cube, [255, 100, 20, 255]);
    front.transform.position[2] = 1.0;
    let mut back = SceneObject::new(22, Primitive::Cube, [20, 100, 255, 255]);
    back.transform.position[2] = -1.0;
    let a = render(
        128,
        128,
        &[front.clone(), back.clone()],
        &front_camera(),
        None,
        None,
        false,
    );
    let b = render(128, 128, &[back, front], &front_camera(), None, None, false);
    assert_eq!(center_id(&a), Some(11));
    assert_eq!(center_id(&b), Some(11));
    assert_eq!(a.object_ids, b.object_ids);
    assert_eq!(a.raster.pixels, b.raster.pixels);
}

#[test]
fn scale_and_translation_change_the_visible_footprint() {
    let object = SceneObject::new(3, Primitive::Cube, [220, 160, 60, 255]);
    let original = render(
        160,
        120,
        std::slice::from_ref(&object),
        &front_camera(),
        None,
        None,
        false,
    );
    let mut changed = object;
    changed.transform.scale[0] = 2.0;
    let scaled = render(
        160,
        120,
        std::slice::from_ref(&changed),
        &front_camera(),
        None,
        None,
        false,
    );
    let visible = |scene: &adars::scene::RenderedScene| {
        scene.object_ids.iter().filter(|id| id.is_some()).count()
    };
    assert!(visible(&scaled) > visible(&original) * 3 / 2);
    changed.transform.position[0] = 2.0;
    let moved = render(160, 120, &[changed], &front_camera(), None, None, false);
    assert_eq!(center_id(&original), Some(3));
    assert_eq!(center_id(&moved), None);
}

#[test]
fn invalid_transforms_and_fully_transparent_objects_do_not_render_or_pick() {
    let mut object = SceneObject::new(5, Primitive::Sphere, [120, 90, 220, 255]);
    for value in [f32::NAN, f32::INFINITY, -1.0, 0.0] {
        object.transform.scale[0] = value;
        assert!(object.validate().is_err());
        let scene = render(
            32,
            32,
            std::slice::from_ref(&object),
            &Camera::default(),
            None,
            None,
            false,
        );
        assert!(scene.object_ids.iter().all(Option::is_none));
    }
    object.transform.scale = [1.0; 3];
    object.transform.position[1] = f32::NEG_INFINITY;
    assert!(object.validate().is_err());
    object.transform.position = [0.0; 3];
    object.transform.rotation[2] = f32::NAN;
    assert!(object.validate().is_err());
    object.transform.rotation = [0.0; 3];
    object.color[3] = 0;
    let scene = render(32, 32, &[object], &Camera::default(), None, None, false);
    assert!(scene.object_ids.iter().all(Option::is_none));
}

#[test]
fn camera_plane_crossing_is_clipped_and_geometry_behind_camera_is_rejected() {
    let mut object = SceneObject::new(8, Primitive::Cube, [30, 220, 150, 255]);
    object.transform.position[2] = 6.7;
    let crossing = render(
        64,
        64,
        std::slice::from_ref(&object),
        &front_camera(),
        None,
        None,
        false,
    );
    assert!(crossing.object_ids.contains(&Some(8)));
    object.transform.position[2] = 8.0;
    let behind = render(64, 64, &[object], &front_camera(), None, None, false);
    assert!(behind.object_ids.iter().all(Option::is_none));
}

#[test]
fn background_is_resized_in_screen_space_and_selection_preserves_picking() {
    let mut background = Raster::new(2, 1, [70, 80, 90, 255]);
    background.pixels[4..8].copy_from_slice(&[140, 150, 160, 255]);
    let blank = render(
        4,
        2,
        &[],
        &Camera::default(),
        Some(&background),
        None,
        false,
    );
    assert_eq!(&blank.raster.pixels[0..4], &[70, 80, 90, 255]);
    assert_eq!(&blank.raster.pixels[8..12], &[140, 150, 160, 255]);
    let object = SceneObject::new(9, Primitive::Cube, [180, 80, 140, 255]);
    let normal = render(
        80,
        80,
        std::slice::from_ref(&object),
        &front_camera(),
        None,
        None,
        false,
    );
    let selected = render(80, 80, &[object], &front_camera(), None, Some(9), false);
    assert_eq!(normal.object_ids, selected.object_ids);
    assert_ne!(normal.raster.pixels, selected.raster.pixels);
}

#[test]
fn wireframe_has_fewer_pickable_pixels_and_hides_rear_edges() {
    let front = SceneObject::new(11, Primitive::Cube, [220, 160, 80, 255]);
    let mut back = SceneObject::new(22, Primitive::Cube, [60, 110, 220, 255]);
    // A very close rear surface must not leak through a broad depth bias.
    // Submit it last so an order-dependent line pass cannot hide the error.
    back.transform.position[2] = -0.000001;
    let solid = render(
        128,
        128,
        std::slice::from_ref(&front),
        &front_camera(),
        None,
        None,
        false,
    );
    let edges = render(128, 128, &[front, back], &front_camera(), None, None, true);
    let filled_count = solid.object_ids.iter().filter(|id| id.is_some()).count();
    let edge_count = edges.object_ids.iter().filter(|id| id.is_some()).count();
    assert!(edge_count > 0 && edge_count < filled_count);
    assert!(
        !edges.object_ids.contains(&Some(22)),
        "occluded rear wireframe leaked through front surface"
    );
}

#[test]
fn zero_dimensions_are_bounded_and_invalid_camera_values_fall_back() {
    let camera = Camera {
        yaw: f32::NAN,
        pitch: f32::INFINITY,
        distance: f32::NEG_INFINITY,
    };
    let result = render(0, 0, &[], &camera, None, None, false);
    assert_eq!((result.raster.width, result.raster.height), (1, 1));
    assert_eq!(result.raster.pixels, [0u8; 4]);
    let result = render(2050, 1, &[], &camera, None, None, false);
    assert_eq!((result.raster.width, result.raster.height), (2048, 1));
}

#[test]
fn obj_export_bakes_transforms_and_uses_valid_face_indices() {
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path =
        std::env::temp_dir().join(format!("forma_scene_{}_{}.obj", std::process::id(), suffix));
    let mut object = SceneObject::new(1, Primitive::Cube, [255; 4]);
    object.name = "Cube\ninvalid OBJ line".into();
    object.transform.position = [2.0, 0.0, 0.0];
    object.transform.scale = [2.0, 1.0, 1.0];
    export_obj(&path, std::slice::from_ref(&object)).unwrap();
    let mut text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("Ada3D Studio"));
    assert!(text.contains("o Cube_invalid_OBJ_line_1"));
    assert!(text.contains("v 0.50000000 -0.75000000 -0.75000000"));
    assert_eq!(
        text.lines().filter(|line| line.starts_with("v ")).count(),
        8
    );
    assert_eq!(
        text.lines().filter(|line| line.starts_with("vn ")).count(),
        12
    );
    assert_eq!(
        text.lines().filter(|line| line.starts_with("f ")).count(),
        12
    );
    for line in text.lines().filter(|line| line.starts_with("f ")) {
        for reference in line.split_whitespace().skip(1) {
            let (vertex, normal) = reference.split_once("//").unwrap();
            assert!((1..=8).contains(&vertex.parse::<usize>().unwrap()));
            assert!((1..=12).contains(&normal.parse::<usize>().unwrap()));
        }
    }
    object.transform.position[0] = 3.0;
    export_obj(&path, std::slice::from_ref(&object)).unwrap();
    let replacement = std::fs::read_to_string(&path).unwrap();
    assert_ne!(replacement, text, "an existing OBJ must be replaceable");
    text = replacement;
    object.transform.scale[0] = 0.0;
    assert!(export_obj(&path, &[object]).is_err());
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        text,
        "failed validation must not truncate an existing export"
    );
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn valid_extreme_camera_and_transform_values_remain_bounded() {
    let mut camera = Camera {
        yaw: f32::MAX,
        pitch: f32::MAX,
        distance: f32::MAX,
    };
    let mut object = SceneObject::new(15, Primitive::Cone, [120, 170, 220, 255]);
    object.transform.position = [f32::MAX, -f32::MAX, f32::MAX];
    object.transform.rotation = [f32::MAX; 3];
    object.transform.scale = [f32::MAX; 3];
    assert!(object.validate().is_ok());
    let result = render(
        32,
        24,
        std::slice::from_ref(&object),
        &camera,
        None,
        None,
        false,
    );
    assert_eq!(result.raster.pixels.len(), 32 * 24 * 4);
    assert_eq!(result.object_ids.len(), 32 * 24);
    assert!(
        result
            .object_ids
            .iter()
            .all(|id| id.is_none() || *id == Some(15))
    );
    object.transform.position = [0.0; 3];
    object.transform.scale = [f32::MIN_POSITIVE; 3];
    let tiny = render(32, 24, &[object], &camera, None, None, false);
    assert!(tiny.object_ids.iter().all(Option::is_none));
    camera.reset();
    assert_eq!(camera, Camera::default());
}

#[test]
fn obj_export_rejects_unbounded_public_inputs_before_creating_a_file() {
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "forma_obj_limits_{}_{}.obj",
        std::process::id(),
        suffix
    ));
    let mut object = SceneObject::new(1, Primitive::Cube, [255; 4]);
    object.name = "x".repeat(adars::scene::MAX_OBJECT_NAME_BYTES + 1);
    assert!(object.validate().is_err());
    assert!(export_obj(&path, &[object]).is_err());
    let objects =
        vec![SceneObject::new(1, Primitive::Cube, [255; 4]); adars::document::MAX_OBJECTS + 1];
    assert!(export_obj(&path, &objects).is_err());
    assert!(
        !path.exists(),
        "input validation must precede file creation"
    );
}
