use super::*;
#[test]
fn shape_palette_chooses_and_drops_only_in_the_2d_workspace() {
    let ctx = egui::Context::default();
    let mut app = studio(256, 256);
    app.scene_view = false;
    app.tab = Tab::Shapes;
    app.tool = Tool::Shape(ShapeKind::Rectangle);
    app.filled = true;
    let before = app.doc.clone();
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
    let cube = label_point(&output, "Star");
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
    assert!(app.tool == Tool::Shape(ShapeKind::Star));
    assert_eq!(app.doc, before, "choosing a shape does not edit artwork");
    let prism = label_point(&output, "Heart");
    let target = app.canvas_rect.center() + Vec2::new(55.0, -35.0);
    frame(
        &mut app,
        vec![egui::Event::PointerMoved(prism), button(prism, true)],
    );
    frame(&mut app, vec![egui::Event::PointerMoved(target)]);
    frame(&mut app, vec![button(target, false)]);
    assert!(app.doc.objects.is_empty());
    assert!(!app.scene_view);
    assert_ne!(app.doc.canvas, before.canvas, "dropping paints the shape");
    assert_eq!(app.history.state_count(), 1);
    app.undo(false);
    assert_eq!(app.doc, before);
    app.tab = Tab::Paint;
    let output = frame(&mut app, vec![]);
    assert!(
        !output
            .shapes
            .iter()
            .any(|shape| matches!(&shape.shape,egui::Shape::Text(t) if t.galley.text()=="Heart")),
        "brush panel stays separate"
    );
}
