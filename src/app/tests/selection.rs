use super::*;

#[test]
fn eight_handles_resize_independently_and_each_drag_is_one_undo_step() {
    for handle in [
        [-1.0, -1.0],
        [1.0, -1.0],
        [1.0, 1.0],
        [-1.0, 1.0],
        [0.0, -1.0],
        [1.0, 0.0],
        [0.0, 1.0],
        [-1.0, 0.0],
    ] {
        let ctx = egui::Context::default();
        let mut app = studio(32, 32);
        app.tool = Tool::Select;
        app.scene_view = false;
        app.doc.canvas.clear([0; 4]);
        app.doc
            .canvas
            .stamp(14.0, 14.0, 3.0, [20, 160, 220, 255], 1.0);
        app.selection = adars::selection::Selection::new(&app.doc.canvas, [8.0, 8.0], [20.0, 20.0]);
        let before = app.doc.clone();
        let origin = Pos2::new(140.0 + handle[0] * 60.0, 140.0 + handle[1] * 60.0);
        let target = origin + Vec2::new(handle[0] * 40.0, handle[1] * 40.0);
        for (point, pressed) in [(origin, Some(true)), (target, None), (target, Some(false))] {
            let mut events = vec![egui::Event::PointerMoved(point)];
            if let Some(pressed) = pressed {
                events.push(egui::Event::PointerButton {
                    pos: point,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Default::default(),
                });
            }
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    let painter = ctx.layer_painter(egui::LayerId::background());
                    app.selection_overlay(
                        ctx,
                        &painter,
                        Rect::from_min_size(Pos2::ZERO, Vec2::splat(320.0)),
                        10.0,
                        Some(point),
                    );
                },
            );
        }
        let selection = app.selection.as_ref().unwrap();
        for (axis, component) in handle.into_iter().enumerate() {
            assert_eq!(
                selection.size[axis],
                if component == 0.0 { 12.0 } else { 16.0 }
            );
            assert_eq!(selection.center[axis], 14.0 + component * 2.0);
        }
        assert!(app.selection_drag.is_none());
        assert_ne!(app.doc, before);
        app.undo(false);
        assert_eq!(app.doc, before);
        assert!(!app.history.can_undo());
    }
}
