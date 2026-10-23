use super::*;

mod menu;
mod scene_paint;
mod selection;
mod shapes;

#[test]
fn background_project_load_applies_valid_results_and_preserves_artwork_on_failure() {
    let path =
        std::env::temp_dir().join(format!("ada-ui-load-{}.ada3dproject", std::process::id()));
    let doc = Document::new(32, 16);
    persistence::save_project(&path, &doc).unwrap();
    let ctx = egui::Context::default();
    let mut app = studio(8, 8);
    app.begin_load(path.clone(), &ctx);
    let until = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while app.loading.is_some() && std::time::Instant::now() < until {
        app.poll_load();
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert!(app.loading.is_none());
    assert_eq!(app.doc, doc);
    assert_eq!(app.preferences.recent_projects.first(), Some(&path));
    std::fs::remove_file(&path).unwrap();
    app.begin_load(path, &ctx);
    let until = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while app.loading.is_some() && std::time::Instant::now() < until {
        app.poll_load();
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert!(app.loading.is_none());
    assert_eq!(app.doc, doc);
    assert!(app.error.is_some());
}

#[test]
fn canvas_magnifies_with_exact_pixels_and_recent_open_preserves_unsaved_work() {
    assert_eq!(
        canvas_texture_options().magnification,
        egui::TextureFilter::Nearest
    );
    let mut app = studio(8, 8);
    let ctx = egui::Context::default();
    app.doc.canvas.fill(0, 0, [10, 20, 30, 255]);
    app.changed();
    let before = app.doc.clone();
    let path = PathBuf::from("saved.ada3dproject");
    app.request(Pending::OpenPath(path.clone()), &ctx);
    assert!(matches!(&app.pending,Some(Pending::OpenPath(p)) if *p==path));
    assert!(app.loading.is_none());
    assert_eq!(app.doc, before);
}

fn studio(width: u32, height: u32) -> Studio {
    let ctx = egui::Context::default();
    let cc = eframe::CreationContext::_new_kittest(ctx);
    let mut studio = Studio::new(&cc, None);
    studio.replace(Document::new(width, height), None);
    studio
}

#[test]
fn marquee_and_handle_drag_commit_one_undoable_transform() {
    let ctx = egui::Context::default();
    let mut app = studio(32, 32);
    app.tool = Tool::Select;
    app.scene_view = false;
    let before = app.doc.clone();
    let rect = Rect::from_min_size(Pos2::ZERO, Vec2::splat(320.0));
    let mut step = |point: Pos2, button: Option<bool>| {
        let mut raw = egui::RawInput {
            screen_rect: Some(rect),
            ..Default::default()
        };
        raw.events.push(egui::Event::PointerMoved(point));
        if let Some(pressed) = button {
            raw.events.push(egui::Event::PointerButton {
                pos: point,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            });
        }
        let _ = ctx.run(raw, |ctx| {
            let painter = ctx.layer_painter(egui::LayerId::background());
            app.selection_overlay(ctx, &painter, rect, 10.0, Some(point));
        });
    };
    step(Pos2::new(40.0, 40.0), Some(true));
    step(Pos2::new(120.0, 120.0), None);
    step(Pos2::new(120.0, 120.0), Some(false));
    step(Pos2::new(80.0, 80.0), Some(true));
    step(Pos2::new(100.0, 100.0), None);
    step(Pos2::new(100.0, 100.0), Some(false));
    assert!(app.history.can_undo());
    assert_ne!(app.doc, before);
    let transformed = app.doc.clone();
    app.undo(false);
    assert_eq!(app.doc, before);
    assert!(!app.history.can_undo());
    app.undo(true);
    assert_eq!(app.doc, transformed);
}
#[test]
fn picker_samples_all_channels_without_editing_document() {
    let mut app = studio(32, 32);
    app.doc.canvas = Raster::new(32, 32, [17, 89, 204, 123]);
    let before = app.doc.clone();
    app.tool = Tool::Dropper;
    app.start_gesture([7.0, 9.0]);
    assert_eq!(app.color, [17, 89, 204, 123]);
    assert_eq!(app.doc, before);
    assert!(!app.history.can_undo());
}
#[test]
fn dialog_sampler_returns_with_exact_color_and_previous_tool() {
    let mut app = studio(32, 32);
    app.doc.canvas = Raster::new(32, 32, [33, 76, 191, 149]);
    let before = app.doc.clone();
    app.show_color = true;
    app.tool = Tool::Eraser;
    app.begin_sampler();
    assert!(!app.show_color);
    assert!(app.tool == Tool::Dropper);
    app.start_gesture([3.0, 4.0]);
    assert_eq!(app.color, [33, 76, 191, 149]);
    assert!(app.show_color);
    assert!(app.tool == Tool::Eraser);
    assert_eq!(app.doc, before);
    assert!(!app.history.can_undo());
}
#[test]
fn cancel_sampler_keeps_color_and_reopens_dialog() {
    let mut app = studio(32, 32);
    let color = app.color;
    app.begin_sampler();
    app.finish_sampler();
    assert_eq!(app.color, color);
    assert!(app.show_color);
    assert!(app.tool == Tool::Brush);
}
#[test]
fn stationary_brush_does_not_accumulate_opacity_from_repaints() {
    let mut s = studio(3, 3);
    s.doc.canvas.clear([0; 4]);
    s.saved = s.doc.clone();
    s.radius = 0.5;
    s.opacity = 0.5;
    s.color = [255, 0, 0, 255];
    s.start_gesture([1.5, 1.5]);
    let once = s.doc.canvas.clone();
    for _ in 0..20 {
        s.draw_gesture([1.5, 1.5]);
    }
    assert_eq!(s.doc.canvas, once);
    assert_eq!(s.doc.canvas.pixel(1, 1), [255, 0, 0, 128]);
    s.finish_edit();
    assert_eq!(s.history.state_count(), 1);
    s.undo(false);
    assert_eq!(s.doc.canvas.pixel(1, 1), [0; 4]);
    assert!(!s.dirty);
}

#[test]
fn shape_preview_commits_one_edit_and_undo_restores_saved_state() {
    let mut s = studio(32, 32);
    let before = s.doc.clone();
    s.set_tool(Tool::Shape(ShapeKind::Rectangle));
    s.filled = true;
    s.start_gesture([2.0, 2.0]);
    s.draw_gesture([10.0, 10.0]);
    s.draw_gesture([20.0, 20.0]);
    s.finish_edit();
    let after = s.doc.clone();
    assert!(s.dirty);
    assert_eq!(s.history.state_count(), 1);
    s.undo(false);
    assert_eq!(s.doc, before);
    assert!(!s.dirty);
    s.undo(true);
    assert_eq!(s.doc, after);
    assert!(s.dirty);
}

#[test]
fn selection_tool_enters_scene_and_brush_returns_to_canvas() {
    let mut s = studio(8, 8);
    s.set_tool(Tool::Select);
    assert!(s.scene_view);
    s.set_tool(Tool::Brush);
    assert!(!s.scene_view);
}

#[test]
fn pixel_mapping_tracks_canvas_pan_and_zoom_and_clamps_picking_edges() {
    let rect = Rect::from_min_size(Pos2::new(150.0, 75.0), Vec2::new(600.0, 400.0));
    assert_eq!(canvas_point(rect.center(), rect, 960, 640), [480.0, 320.0]);
    assert_eq!(pick_index(rect.right_bottom(), rect, 48, 32), 48 * 32 - 1);
    assert_eq!(pick_index(rect.left_top(), rect, 48, 32), 0);
    let moved = rect.translate(Vec2::new(-50.0, 100.0));
    assert_eq!(
        canvas_point(moved.center(), moved, 960, 640),
        [480.0, 320.0]
    );
}
#[test]
fn system_theme_changes_live_and_manual_override_is_respected() {
    let ctx = egui::Context::default();
    let mut prefs = Preferences::default();
    appearance::install(&ctx, &prefs);
    let _ = ctx.run(
        egui::RawInput {
            system_theme: Some(egui::Theme::Dark),
            ..Default::default()
        },
        |_| {},
    );
    assert_eq!(ctx.theme(), egui::Theme::Dark);
    let _ = ctx.run(
        egui::RawInput {
            system_theme: Some(egui::Theme::Light),
            ..Default::default()
        },
        |_| {},
    );
    assert_eq!(ctx.theme(), egui::Theme::Light);
    prefs.theme = ThemeMode::Dark;
    appearance::install(&ctx, &prefs);
    let _ = ctx.run(
        egui::RawInput {
            system_theme: Some(egui::Theme::Light),
            ..Default::default()
        },
        |_| {},
    );
    assert_eq!(ctx.theme(), egui::Theme::Dark);
    prefs.theme = ThemeMode::System;
    appearance::install(&ctx, &prefs);
    assert_eq!(ctx.theme(), egui::Theme::Light);
}
#[test]
fn settings_page_and_theme_changes_preserve_artwork_and_history() {
    let mut s = studio(16, 16);
    s.start_gesture([4.0, 4.0]);
    s.finish_edit();
    let before = s.doc.clone();
    let saved = s.saved.clone();
    let count = s.history.state_count();
    let dirty = s.dirty;
    let ctx = egui::Context::default();
    s.show_settings = true;
    s.configure_theme(&ctx, ThemeMode::Dark);
    let _ = ctx.run(egui::RawInput::default(), |ctx| s.settings_page(ctx));
    assert_eq!(s.doc, before);
    assert_eq!(s.saved, saved);
    assert_eq!(s.history.state_count(), count);
    assert_eq!(s.dirty, dirty);
}
#[test]
fn welcome_new_and_dirty_close_preserve_document_policy() {
    let ctx = egui::Context::default();
    let mut app = studio(64, 64);
    app.show_welcome = true;
    app.execute(Pending::New, &ctx);
    assert!(!app.show_welcome);
    app.doc.canvas.stamp(12.0, 12.0, 4.0, [255, 0, 0, 255], 1.0);
    app.changed();
    let artwork = app.doc.clone();
    app.show_welcome = true;
    app.request(Pending::Close, &ctx);
    assert!(matches!(app.pending, Some(Pending::Close)));
    assert!(!app.allow_close);
    assert!(!app.show_welcome);
    assert_eq!(app.doc, artwork);
}
#[test]
fn brush_presets_change_stroke_controls_without_editing_artwork() {
    let mut app = studio(64, 64);
    let before = app.doc.clone();
    app.select_brush_preset(1);
    assert_eq!((app.radius, app.opacity), (1.5, 1.0));
    app.select_brush_preset(2);
    assert_eq!((app.radius, app.opacity), (14.0, 0.4));
    assert_eq!(app.doc, before);
    assert!(!app.dirty);
    assert!(!app.history.can_undo());
}
#[test]
fn clicking_welcome_new_enters_editor() {
    let ctx = egui::Context::default();
    let mut app = studio(64, 64);
    app.show_welcome = true;
    let input = || egui::RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 820.0))),
        ..Default::default()
    };
    let _ = ctx.run(input(), |ctx| app.welcome_screen(ctx));
    let output = ctx.run(input(), |ctx| app.welcome_screen(ctx));
    let target = output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Rect(r)
                if (r.rect.width() - 157.0).abs() < 0.1
                    && (r.rect.height() - 100.0).abs() < 0.1 =>
            {
                Some(r.rect.center())
            }
            _ => None,
        })
        .min_by(|a, b| a.x.total_cmp(&b.x))
        .expect("New action card");
    for pressed in [true, false] {
        let mut raw = input();
        raw.events = vec![
            egui::Event::PointerMoved(target),
            egui::Event::PointerButton {
                pos: target,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            },
        ];
        let _ = ctx.run(raw, |ctx| app.welcome_screen(ctx));
    }
    assert!(!app.show_welcome);
    assert!(!app.dirty);
}
#[test]
fn transparent_brush_and_bucket_have_undoable_alpha_edits() {
    let mut app = studio(32, 32);
    let before = app.doc.clone();
    app.transparent_paint = true;
    app.radius = 2.0;
    app.opacity = 1.0;
    app.start_gesture([5.0, 5.0]);
    app.draw_gesture([12.0, 5.0]);
    app.finish_edit();
    assert_eq!(app.doc.canvas.pixel(8, 5), [0; 4]);
    assert_eq!(app.doc.canvas.pixel(25, 25), before.canvas.pixel(25, 25));
    let erased = app.doc.clone();
    app.undo(false);
    assert_eq!(app.doc, before);
    app.undo(true);
    assert_eq!(app.doc, erased);
    app.set_tool(Tool::Fill);
    app.start_gesture([25.0, 25.0]);
    assert_eq!(app.doc.canvas.pixel(25, 25), [0; 4]);
    app.undo(false);
    assert_eq!(app.doc, erased);
}
#[test]
fn native_close_with_dirty_work_exposes_confirmation_above_welcome() {
    let ctx = egui::Context::default();
    let mut app = studio(32, 32);
    app.start_gesture([8.0, 8.0]);
    app.finish_edit();
    let before = app.doc.clone();
    app.show_welcome = true;
    let mut raw = egui::RawInput::default();
    raw.viewports
        .get_mut(&egui::ViewportId::ROOT)
        .unwrap()
        .events
        .push(egui::ViewportEvent::Close);
    let out = ctx.run(raw, |ctx| app.handle_close_request(ctx));
    assert!(!app.show_welcome);
    assert!(!app.allow_close);
    assert!(matches!(app.pending, Some(Pending::Close)));
    assert_eq!(app.doc, before);
    assert!(out.viewport_output.values().any(|v| {
        v.commands
            .iter()
            .any(|c| matches!(c, egui::ViewportCommand::CancelClose))
    }));
}

#[test]
fn update_restart_preserves_unsaved_work_until_confirmed() {
    let mut app = studio(16, 16);
    app.doc.canvas.stamp(5.0, 5.0, 2.0, [10, 20, 30, 255], 1.0);
    app.changed();
    let before = app.doc.clone();
    app.request(Pending::Update, &egui::Context::default());
    assert!(matches!(app.pending, Some(Pending::Update)));
    assert_eq!(app.doc, before);
}

#[test]
fn light_theme_headings_keep_dark_foreground_colors() {
    let ctx = egui::Context::default();
    appearance::install(&ctx, &Preferences::default());
    let style = ctx.style_of(egui::Theme::Light);
    assert_eq!(
        style.visuals.strong_text_color(),
        Color32::from_rgb(37, 46, 65)
    );
}
