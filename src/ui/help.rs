//! Nonmodal native guide window with screenshot annotations.
mod topics;
use crate::{appearance, gallery, icons, native_window};
use eframe::egui::{self, Color32, Vec2};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
pub const TITLE: &str = "Ada3D — User guide";
#[derive(Default)]
pub struct GuideState {
    pub topic: usize,
    capture: Option<std::path::PathBuf>,
    capture_started: Option<std::time::Instant>,
    capture_requested: bool,
}
#[derive(Clone, Default)]
pub struct Guide {
    state: Arc<Mutex<GuideState>>,
    closed: Arc<AtomicBool>,
}
impl Guide {
    pub fn set_topic(&self, topic: usize) {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).topic = topic.min(7);
    }
    pub fn enable_capture(&self, path: std::path::PathBuf) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.capture = Some(path);
        state.capture_started = None;
        state.capture_requested = false;
    }

    pub fn reset(&self) {
        self.closed.store(false, Ordering::Relaxed);
    }
    pub fn was_closed(&self) -> bool {
        self.closed.swap(false, Ordering::Relaxed)
    }
    pub fn show(&self, ctx: &egui::Context, prefs: &adars::settings::Preferences) {
        let prefs = prefs.clone();
        let state = self.state.clone();
        let closed = self.closed.clone();
        ctx.show_viewport_deferred(
            egui::ViewportId::from_hash_of("ada-guide"),
            egui::ViewportBuilder::default()
                .with_title(TITLE)
                .with_inner_size([900.0, 720.0])
                .with_min_inner_size([740.0, 560.0])
                .with_icon(icons::window_icon())
                .with_transparent(true),
            move |ctx, _class| {
                if ctx.input(|i| i.viewport().close_requested()) {
                    closed.store(true, Ordering::Relaxed);
                    ctx.request_repaint_of(egui::ViewportId::ROOT);
                    return;
                }
                if let Some(handle) = native_window::owned_window_key(TITLE) {
                    let key = (
                        handle,
                        prefs.glass_effects,
                        prefs.backdrop,
                        ctx.theme() == egui::Theme::Dark,
                    );
                    let id = egui::Id::new("guide-native-material");
                    if ctx
                        .data(|d| d.get_temp::<(usize, bool, adars::settings::Backdrop, bool)>(id))
                        != Some(key)
                        && native_window::configure_owned(TITLE, key.1, key.2, key.3).is_ok()
                    {
                        ctx.data_mut(|d| d.insert_temp(id, key));
                    }
                }
                let mut state = state.lock().unwrap_or_else(|e| e.into_inner());
                draw(ctx, &mut state);
                capture(ctx, &mut state);
            },
        );
    }
}
pub fn draw(ctx: &egui::Context, state: &mut GuideState) {
    let colors = appearance::palette(ctx);
    egui::TopBottomPanel::top("help-heading")
        .frame(egui::Frame::new().fill(colors.panel).inner_margin(18))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                icons::logo(ui, 32.0);
                ui.vertical(|ui| {
                    ui.heading("Ada3D Help");
                    ui.label(egui::RichText::new("R3F Studio · Version 1.0.0").color(colors.muted));
                });
            });
        });
    egui::SidePanel::left("help-topics")
        .exact_width(168.0)
        .frame(egui::Frame::new().fill(colors.panel).inner_margin(12))
        .show(ctx, |ui| {
            ui.add_space(10.0);
            for (index, title) in topics::NAMES.iter().enumerate() {
                if ui
                    .add_sized(
                        [144.0, 36.0],
                        egui::Button::new(*title).selected(state.topic == index),
                    )
                    .clicked()
                {
                    state.topic = index;
                }
            }
        });
    egui::CentralPanel::default().frame(egui::Frame::new().fill(colors.panel).inner_margin(20)).show(ctx, |ui| {
        ui.style_mut().text_styles.insert(egui::TextStyle::Body, egui::FontId::proportional(14.0));
        egui::ScrollArea::vertical().show(ui, |ui| {
            if state.topic == 7 {
                ui.heading("Keyboard and mouse shortcuts"); ui.add_space(16.0);
                egui::Grid::new("help-shortcuts").striped(true).min_col_width(150.0).show(ui, |ui| {
                    for (keys, action) in [("B / E / F / I", "Brush / Eraser / Bucket / Picker"), ("V / Delete", "3D selection / Delete object"), ("Ctrl+N / O / S", "New / Open / Save"), ("Ctrl+Shift+S", "Save as"), ("Ctrl+Z / Ctrl+Shift+Z", "Undo / Redo"), ("Scroll / middle-drag", "Zoom / Pan"), ("Right-drag", "Orbit in 3D"), ("Escape", "Dismiss a menu or clear selection")] { ui.strong(keys); ui.label(action); ui.end_row(); }
                });
                ui.add_space(16.0); ui.label("On macOS, use Command for document shortcuts. Text fields keep normal editing shortcuts."); return;
            }
            let topic = &topics::TOPICS[state.topic.min(6)];
            ui.heading(topic.title); ui.add_space(14.0);
            let texture = gallery::photo(ctx, topic.image);
            let width = ui.available_width().min(720.0);
            let ratio = texture.size()[1] as f32 / texture.size()[0] as f32;
            let (rect, _) = ui.allocate_exact_size(Vec2::new(width, width * ratio), egui::Sense::hover());
            gallery::paint(ctx, ui.painter(), rect, topic.image, 1.0);
            for (index, point) in topic.points.iter().enumerate() { gallery::callout(ui.painter(), rect, *point, 0.035, index + 1); }
            ui.add_space(16.0);
            for (index, step) in topic.steps.iter().enumerate() {
                egui::Frame::new().fill(ui.visuals().faint_bg_color).corner_radius(7).inner_margin(12).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal_wrapped(|ui| { ui.label(egui::RichText::new(format!("{}.", index + 1)).color(Color32::from_rgb(255,195,94)).strong()); ui.label(*step); });
                }); ui.add_space(6.0);
            }
        });
    });
}

fn capture(ctx: &egui::Context, state: &mut GuideState) {
    let Some(path) = state.capture.clone() else {
        return;
    };
    let image = ctx.input(|i| {
        i.events.iter().find_map(|event| {
            if let egui::Event::Screenshot { image, .. } = event {
                Some(image.clone())
            } else {
                None
            }
        })
    });
    if let Some(image) = image {
        let pixels: Vec<u8> = image
            .pixels
            .iter()
            .flat_map(|pixel| pixel.to_array())
            .collect();
        let result = image::save_buffer_with_format(
            &path,
            &pixels,
            image.size[0] as u32,
            image.size[1] as u32,
            image::ColorType::Rgba8,
            image::ImageFormat::Png,
        );
        let record = serde_json::json!({"success":result.is_ok(),"source":"Ada3D Help framebuffer","width":image.size[0],"height":image.size[1],"topic":state.topic});
        let _ = std::fs::write(path.with_extension("json"), record.to_string());
        state.capture = None;
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Close);
        ctx.request_repaint_of(egui::ViewportId::ROOT);
    } else {
        let started = state
            .capture_started
            .get_or_insert_with(std::time::Instant::now);
        if started.elapsed().as_secs_f32() > 0.8 && !state.capture_requested {
            state.capture_requested = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(16));
    }
}
