use super::*;
fn ready(app: &mut Studio) {
    app.rendered = Some(scene::render(
        app.doc.canvas.width,
        app.doc.canvas.height,
        &app.doc.objects,
        &app.camera,
        None,
        None,
        false,
    ));
    app.render_dirty = false;
    app.display_generation = Some(app.render_generation);
}
#[test]
fn surface_brush_is_local_undoable_and_leaves_canvas_untouched() {
    let mut app = studio(256, 256);
    app.place_scene_object(Primitive::Cube, [0.0; 3]);
    app.history.clear();
    app.set_scene_paint_tool(Tool::Brush);
    app.color = [240, 20, 30, 255];
    app.radius = 12.0;
    ready(&mut app);
    let before = app.doc.clone();
    app.start_scene_paint([128.0, 128.0]);
    app.continue_scene_paint([145.0, 128.0]);
    app.finish_edit();
    assert_eq!(app.doc.canvas, before.canvas);
    let paint = app.doc.objects[0].paint.as_ref().unwrap();
    let changed = paint
        .pixels
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| **p != before.objects[0].color)
        .count();
    assert!(changed > 10 && changed < 4096, "localized brush: {changed}");
    let painted = app.doc.clone();
    app.undo(false);
    assert_eq!(app.doc, before);
    assert!(!app.history.can_undo());
    app.undo(true);
    assert_eq!(app.doc, painted);
}
#[test]
fn surface_bucket_sampler_and_transparency_preserve_geometry() {
    let mut app = studio(256, 256);
    app.place_scene_object(Primitive::Sphere, [0.0; 3]);
    app.set_scene_paint_tool(Tool::Fill);
    app.color = [17, 111, 224, 255];
    ready(&mut app);
    let transform = app.doc.objects[0].transform.clone();
    app.start_scene_paint([128.0, 128.0]);
    assert!(
        app.doc.objects[0]
            .paint
            .as_ref()
            .unwrap()
            .pixels
            .as_chunks::<4>()
            .0
            .iter()
            .all(|p| *p == app.color)
    );
    ready(&mut app);
    app.sample_color([128.0, 128.0]);
    assert_eq!(app.color, [17, 111, 224, 255]);
    app.set_scene_paint_tool(Tool::Eraser);
    app.radius = 12.0;
    ready(&mut app);
    app.start_scene_paint([128.0, 128.0]);
    app.finish_edit();
    assert!(
        app.doc.objects[0]
            .paint
            .as_ref()
            .unwrap()
            .pixels
            .as_chunks::<4>()
            .0
            .iter()
            .any(|p| p[3] == 0)
    );
    app.set_scene_paint_tool(Tool::Fill);
    app.transparent_paint = true;
    app.opacity = 1.0;
    ready(&mut app);
    app.start_scene_paint([128.0, 128.0]);
    assert!(
        app.doc.objects[0]
            .paint
            .as_ref()
            .unwrap()
            .pixels
            .as_chunks::<4>()
            .0
            .iter()
            .all(|p| p[3] == 0)
    );
    assert_eq!(app.doc.objects[0].transform, transform);
}
#[test]
fn scene_marker_does_not_accumulate_from_idle_frames() {
    let mut app = studio(256, 256);
    app.place_scene_object(Primitive::Cube, [0.0; 3]);
    app.set_scene_paint_tool(Tool::Brush);
    app.opacity = 0.2;
    ready(&mut app);
    app.start_scene_paint([128.0, 128.0]);
    let first = app.doc.clone();
    for _ in 0..20 {
        app.continue_scene_paint([128.0, 128.0]);
    }
    assert_eq!(app.doc, first);
}
#[test]
fn camera_relative_placement_is_undoable_and_keeps_paint_mode_in_3d() {
    let mut app = studio(256, 256);
    let position = scene::placement(&app.camera, [0.7, 0.3], 256, 256);
    assert!(position.iter().any(|v| v.abs() > 0.2));
    app.place_scene_object(Primitive::Torus, position);
    assert_eq!(app.doc.objects[0].transform.position, position);
    app.undo(false);
    assert!(app.doc.objects.is_empty());
    app.undo(true);
    assert_eq!(app.doc.objects.len(), 1);
    app.set_scene_paint_tool(Tool::Eraser);
    assert!(app.scene_view);
    assert!(app.tab == Tab::Objects);
    assert!(app.scene_paint_tools);
}

#[test]
fn sidebar_click_and_drag_place_real_shapes_in_the_scene() {
    let ctx = egui::Context::default();
    let mut app = studio(256, 256);
    app.scene_view = true;
    app.tab = Tab::Objects;
    app.tool = Tool::Select;
    let mut time = 0.0;
    let mut frame = |app: &mut Studio, events: Vec<egui::Event>| {
        time += 0.03;
        ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 700.0))),
                time: Some(time),
                events,
                ..Default::default()
            },
            |ctx| {
                app.inspector(ctx);
                app.canvas(ctx);
            },
        )
    };
    let output = frame(&mut app, vec![]);
    let label_point = |output: &egui::FullOutput, label: &str| {
        output
            .shapes
            .iter()
            .find_map(|shape| {
                if let egui::Shape::Text(text) = &shape.shape
                    && text.galley.text() == label
                {
                    return Some(text.pos + text.galley.size() * 0.5);
                }
                None
            })
            .expect("shape thumbnail label")
    };
    let cube = label_point(&output, "Cube");
    let button = |point, pressed| egui::Event::PointerButton {
        pos: point,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Default::default(),
    };
    frame(
        &mut app,
        vec![egui::Event::PointerMoved(cube), button(cube, true)],
    );
    let output = frame(&mut app, vec![button(cube, false)]);
    assert_eq!(app.doc.objects.len(), 1);
    assert_eq!(app.doc.objects[0].primitive, Primitive::Cube);
    let prism = label_point(&output, "Prism");
    let target = app.canvas_rect.center() + Vec2::new(55.0, -35.0);
    frame(
        &mut app,
        vec![egui::Event::PointerMoved(prism), button(prism, true)],
    );
    frame(&mut app, vec![egui::Event::PointerMoved(target)]);
    frame(&mut app, vec![button(target, false)]);
    assert_eq!(app.doc.objects.len(), 2, "dropping creates one shape");
    assert_eq!(app.doc.objects[1].primitive, Primitive::Prism);
    assert!(
        app.doc.objects[1]
            .transform
            .position
            .iter()
            .any(|n| n.abs() > 0.1)
    );
    app.undo(false);
    assert_eq!(app.doc.objects.len(), 1);
}
