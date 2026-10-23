use super::*;

struct Editor {
    ctx: egui::Context,
    app: Studio,
    button: Pos2,
}

impl Editor {
    fn new() -> Self {
        let mut editor = Self {
            ctx: egui::Context::default(),
            app: studio(32, 32),
            button: Pos2::ZERO,
        };
        editor.app.preferences.reduce_motion = true;
        editor.frame(vec![]);
        editor.frame(vec![]);
        editor
    }

    fn frame(&mut self, events: Vec<egui::Event>) {
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100.0, 740.0))),
            events,
            ..Default::default()
        };
        let _ = self.ctx.run(input, |ctx| {
            self.app.menu_input_blocked = egui::Popup::is_any_open(ctx);
            self.app.shortcuts(ctx);
            egui::TopBottomPanel::top("test-command").show(ctx, |ui| {
                let button = ui.button("Background command");
                self.button = button.rect.center();
                if button.clicked() {
                    self.app.opacity = 0.25;
                }
            });
            self.app.inspector(ctx);
            self.app.canvas(ctx);
        });
    }

    fn pointer(&mut self, point: Pos2, button: egui::PointerButton, pressed: bool) {
        self.frame(vec![
            egui::Event::PointerMoved(point),
            egui::Event::PointerButton {
                pos: point,
                button,
                pressed,
                modifiers: Default::default(),
            },
        ]);
    }

    fn open(&mut self) {
        let point = Pos2::new(300.0, 200.0);
        self.pointer(point, egui::PointerButton::Secondary, true);
        self.pointer(point, egui::PointerButton::Secondary, false);
        self.frame(vec![]);
        assert!(egui::Popup::is_any_open(&self.ctx));
    }
}

fn key(key: egui::Key) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Default::default(),
    }
}

#[test]
fn context_menu_blocks_paint_zoom_and_shortcuts_and_consumes_dismissal_click() {
    let mut editor = Editor::new();
    editor.open();
    let before = editor.app.doc.clone();
    let zoom = editor.app.zoom;
    let point = Pos2::new(650.0, 600.0);
    editor.frame(vec![
        egui::Event::PointerMoved(point),
        egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: Vec2::new(0.0, 120.0),
            modifiers: Default::default(),
        },
        key(egui::Key::E),
    ]);
    assert!(editor.app.tool == Tool::Brush);
    assert_eq!(editor.app.zoom, zoom);
    editor.pointer(point, egui::PointerButton::Primary, true);
    editor.pointer(point, egui::PointerButton::Primary, false);
    editor.frame(vec![]);
    assert!(!egui::Popup::is_any_open(&editor.ctx));
    assert_eq!(editor.app.doc, before);
    assert!(!editor.app.history.can_undo());
    editor.pointer(point, egui::PointerButton::Primary, true);
    editor.pointer(point, egui::PointerButton::Primary, false);
    assert_ne!(editor.app.doc, before);
}

#[test]
fn context_menu_blocks_background_buttons_until_a_second_click() {
    let mut editor = Editor::new();
    editor.open();
    let point = editor.button;
    editor.pointer(point, egui::PointerButton::Primary, true);
    editor.pointer(point, egui::PointerButton::Primary, false);
    editor.frame(vec![]);
    assert!(!egui::Popup::is_any_open(&editor.ctx));
    assert_eq!(editor.app.opacity, 1.0);
    editor.pointer(point, egui::PointerButton::Primary, true);
    editor.pointer(point, egui::PointerButton::Primary, false);
    assert_eq!(editor.app.opacity, 0.25);
}

#[test]
fn context_menu_commands_still_run_and_close_the_menu() {
    let mut editor = Editor::new();
    editor.open();
    let point = Pos2::new(350.0, 350.0);
    editor.pointer(point, egui::PointerButton::Primary, true);
    editor.pointer(point, egui::PointerButton::Primary, false);
    assert!(editor.app.tool == Tool::Dropper);
    assert!(!egui::Popup::is_any_open(&editor.ctx));
}

#[test]
fn selection_action_bar_rotates_without_starting_a_marquee_and_done_preserves_pixels() {
    let mut editor = Editor::new();
    editor.app.tool = Tool::Select;
    editor.app.scene_view = false;
    editor.app.doc.canvas.clear([0; 4]);
    editor
        .app
        .doc
        .canvas
        .stamp(10.0, 12.0, 2.0, [255, 50, 20, 255], 1.0);
    editor.app.selection =
        adars::selection::Selection::new(&editor.app.doc.canvas, [5.0, 5.0], [20.0, 20.0]);
    let before = editor.app.doc.clone();
    editor.frame(vec![]);
    editor.frame(vec![]);
    let bar = editor.app.selection_actions_rect;
    let point = Pos2::new(bar.left() + 45.0, bar.center().y);
    editor.pointer(point, egui::PointerButton::Primary, true);
    editor.pointer(point, egui::PointerButton::Primary, false);
    assert_eq!(
        editor.app.selection.as_ref().unwrap().angle,
        -std::f32::consts::FRAC_PI_2,
        "bar={bar:?}, point={point:?}, selection_drag={}, marquee={:?}",
        editor.app.selection_drag.is_some(),
        editor.app.select_start
    );
    assert!(editor.app.select_start.is_none());
    assert!(!egui::Popup::is_any_open(&editor.ctx));
    let transformed = editor.app.doc.clone();
    let point = Pos2::new(bar.right() - 30.0, bar.center().y);
    editor.pointer(point, egui::PointerButton::Primary, true);
    editor.pointer(point, egui::PointerButton::Primary, false);
    assert!(editor.app.selection.is_none());
    assert_eq!(editor.app.doc, transformed);
    editor.app.undo(false);
    assert_eq!(editor.app.doc, before);
    assert!(!editor.app.history.can_undo());
}

#[test]
fn selected_area_keeps_its_context_actions_and_rotation_is_undoable() {
    let mut editor = Editor::new();
    editor.app.tool = Tool::Select;
    editor.app.scene_view = false;
    editor.app.selection =
        adars::selection::Selection::new(&editor.app.doc.canvas, [5.0, 5.0], [20.0, 20.0]);
    editor.open();
    assert!(editor.app.selection.is_some());
    let point = Pos2::new(350.0, 315.0);
    editor.pointer(point, egui::PointerButton::Primary, true);
    editor.pointer(point, egui::PointerButton::Primary, false);
    assert_eq!(
        editor.app.selection.as_ref().unwrap().angle,
        -std::f32::consts::FRAC_PI_2
    );
    assert!(!egui::Popup::is_any_open(&editor.ctx));
    assert!(editor.app.select_start.is_none());
}

#[test]
fn context_menu_blocks_scene_navigation_and_escape_only_closes_menu() {
    let mut editor = Editor::new();
    editor.app.scene_view = true;
    editor.app.selected = Some(99);
    editor.open();
    let yaw = editor.app.camera.yaw;
    let pitch = editor.app.camera.pitch;
    let pan = editor.app.pan;
    let point = Pos2::new(650.0, 600.0);
    editor.pointer(point, egui::PointerButton::Middle, true);
    editor.frame(vec![egui::Event::PointerMoved(point + Vec2::splat(20.0))]);
    assert_eq!(editor.app.pan, pan);
    editor.pointer(point, egui::PointerButton::Middle, false);
    if !egui::Popup::is_any_open(&editor.ctx) {
        editor.open();
    }
    editor.pointer(point, egui::PointerButton::Secondary, true);
    editor.frame(vec![egui::Event::PointerMoved(point + Vec2::splat(30.0))]);
    assert_eq!(editor.app.camera.yaw, yaw);
    assert_eq!(editor.app.camera.pitch, pitch);
    editor.frame(vec![key(egui::Key::Escape)]);
    assert!(!egui::Popup::is_any_open(&editor.ctx));
    assert_eq!(editor.app.selected, Some(99));
}
