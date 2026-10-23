mod canvas;
mod capture;
mod dialogs;
mod document;
mod exporting;
mod inspector;
mod jobs;
mod menus;
mod preferences;
mod projects;
mod render;
mod scene_tools;
mod selection;
mod shape_tools;
mod startup;
mod toolbar;
mod updates;

use crate::icons::{self, Icon};
use adars::render_worker::{RenderRequest, RenderWorker};
use adars::{
    document::Document,
    history::History,
    persistence,
    raster::{Raster, ShapeKind},
    scene::{self, Camera, Primitive, RenderedScene},
};
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, Vec2};
use std::path::PathBuf;

use crate::appearance;
use crate::help;
use crate::native_window;
use adars::settings::{self, AlphaBackground, Backdrop, Preferences, ThemeMode};

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Paint,
    Shapes,
    Objects,
    Canvas,
}
#[derive(Clone, Copy, PartialEq)]
enum Tool {
    Brush,
    Eraser,
    Fill,
    Dropper,
    Shape(ShapeKind),
    Select,
}
#[derive(Clone)]
enum Pending {
    New,
    Open,
    OpenPath(PathBuf),
    Update,
    Close,
}
struct Gesture {
    start: [f32; 2],
    last: [f32; 2],
    base: Option<Raster>,
    tool: Tool,
    transparent: bool,
}

#[derive(Clone, Copy)]
enum SelectionMode {
    Move,
    Resize([f32; 2]),
    Rotate,
}

struct SelectionDrag {
    mode: SelectionMode,
    start: [f32; 2],
    selection: Option<adars::selection::Selection>,
    object: Option<scene::SceneObject>,
    size: [f32; 2],
}

pub struct Studio {
    doc: Document,
    saved: Document,
    history: History,
    path: Option<PathBuf>,
    dirty: bool,
    tab: Tab,
    tool: Tool,
    color: [u8; 4],
    radius: f32,
    opacity: f32,
    filled: bool,
    transparent_paint: bool,
    scene_view: bool,
    scene_paint_tools: bool,
    scene_gesture: Option<[f32; 2]>,
    camera: Camera,
    wireframe: bool,
    selected: Option<u64>,
    rendered: Option<RenderedScene>,
    texture: Option<egui::TextureHandle>,
    render_dirty: bool,
    worker: RenderWorker,
    render_pending: Option<RenderRequest>,
    render_generation: u64,
    display_generation: Option<u64>,
    render_floor: u64,
    worker_failed: bool,
    zoom: f32,
    pan: Vec2,
    gesture: Option<Gesture>,
    property_active: bool,
    selection: Option<adars::selection::Selection>,
    select_start: Option<[f32; 2]>,
    selection_drag: Option<SelectionDrag>,
    selection_drag_point: [f32; 2],
    selection_actions_rect: Rect,
    canvas_width: u32,
    canvas_height: u32,
    pending: Option<Pending>,
    allow_close: bool,
    show_help: bool,
    guide: help::Guide,
    native_header: bool,
    header_initialized: bool,
    error: Option<String>,
    status: String,
    preferences: Preferences,
    preferences_path: Option<PathBuf>,
    preferences_notice: String,
    show_settings: bool,
    settings_tab: usize,
    show_color: bool,
    menu_input_blocked: bool,
    sampler_restore: Option<(Tool, bool)>,
    show_splash: bool,
    splash_capture: bool,
    startup_started: Option<std::time::Instant>,
    splash_exit_started: Option<std::time::Instant>,
    workspace_started: Option<std::time::Instant>,
    startup_progress: f32,
    center_pending: bool,
    show_welcome: bool,
    brush_preset: u8,
    backdrop_applied: Option<(bool, Backdrop, bool)>,
    backdrop_notice: String,
    backdrop_active: bool,
    paint_tab_rect: Rect,
    objects_tab_rect: Rect,
    settings_rect: Rect,
    open_rect: Rect,
    window_title: String,
    canvas_rect: Rect,
    capture_path: Option<PathBuf>,
    capture_requested: bool,
    capture_frames: u16,
    capture_started: Option<f64>,
    capture_step: u8,
    capture_checks: [bool; 4],
    capture_visual_only: bool,
    capture_guide: bool,
    capture_context: u8,
    capture_tooltip: bool,
    capture_menu: bool,
    capture_delay: f64,
    loading: Option<(PathBuf, jobs::Job<Document>)>,
    export_dialog: Option<exporting::ExportDialog>,
    shutdown_requested: bool,
    updater: adars::updater::Updater,
    startup_update_checked: bool,
    startup_update_installing: bool,
}

impl Studio {
    pub fn new(cc: &eframe::CreationContext<'_>, preferences_path: Option<PathBuf>) -> Self {
        let loaded = preferences_path
            .as_ref()
            .map(|p| settings::load(p))
            .unwrap_or_else(|| Ok(Preferences::default()));
        let (mut preferences, preferences_notice) = match loaded {
            Ok(p) => (p, String::new()),
            Err(e) => (Preferences::default(), e),
        };
        // Materials are always enabled; retain the serialized flag only so
        // preferences from previous releases can still be read.
        preferences.glass_effects = true;
        #[cfg(target_os = "linux")]
        {
            preferences.backdrop = Backdrop::Acrylic;
        }
        appearance::install(&cc.egui_ctx, &preferences);
        let doc = Document::default();
        let repaint = cc.egui_ctx.clone();
        let worker = RenderWorker::new(move || repaint.request_repaint());
        Self {
            saved: doc.clone(),
            doc,
            history: History::new(),
            path: None,
            dirty: false,
            tab: Tab::Paint,
            tool: Tool::Brush,
            color: [0, 117, 112, 255],
            radius: 8.0,
            opacity: 1.0,
            filled: false,
            transparent_paint: false,
            scene_view: false,
            scene_paint_tools: false,
            scene_gesture: None,
            camera: Camera::default(),
            wireframe: false,
            selected: None,
            rendered: None,
            texture: None,
            render_dirty: true,
            worker,
            render_pending: None,
            render_generation: 0,
            display_generation: None,
            render_floor: 1,
            worker_failed: false,
            zoom: 1.0,
            pan: Vec2::ZERO,
            gesture: None,
            property_active: false,
            selection: None,
            select_start: None,
            selection_drag: None,
            selection_drag_point: [f32::NAN; 2],
            selection_actions_rect: Rect::NOTHING,
            canvas_width: 960,
            canvas_height: 640,
            pending: None,
            allow_close: false,
            show_help: false,
            guide: help::Guide::default(),
            native_header: false,
            header_initialized: false,
            error: None,
            status: "Ready".into(),
            show_welcome: preferences.show_welcome,
            brush_preset: 0,
            backdrop_applied: None,
            backdrop_notice: String::new(),
            backdrop_active: false,
            preferences,
            preferences_path,
            preferences_notice,
            show_settings: false,
            settings_tab: 0,
            show_color: false,
            menu_input_blocked: false,
            sampler_restore: None,
            show_splash: true,
            splash_capture: false,
            startup_started: None,
            splash_exit_started: None,
            workspace_started: None,
            startup_progress: 0.0,
            center_pending: true,
            paint_tab_rect: Rect::NOTHING,
            objects_tab_rect: Rect::NOTHING,
            settings_rect: Rect::NOTHING,
            open_rect: Rect::NOTHING,
            canvas_rect: Rect::NOTHING,
            window_title: String::new(),
            capture_path: None,
            capture_requested: false,
            capture_frames: 0,
            capture_started: None,
            capture_step: 0,
            capture_checks: [false; 4],
            capture_visual_only: false,
            capture_guide: false,
            capture_context: 0,
            capture_tooltip: false,
            capture_menu: false,
            capture_delay: 0.35,
            loading: None,
            export_dialog: None,
            shutdown_requested: false,
            updater: adars::updater::Updater::default(),
            startup_update_checked: false,
            startup_update_installing: false,
        }
    }

    // An original deterministic study for native smoke/visual verification.
}

impl eframe::App for Studio {
    fn raw_input_hook(&mut self, _ctx: &egui::Context, input: &mut egui::RawInput) {
        self.capture_input(input);
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0; 4]
    }
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        native_window::shutdown();
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if self.shutdown_requested {
            self.cancel_file_tasks();
            self.status = "Finishing file tasks…".into();
            if jobs::active_count() == 0 {
                self.allow_close = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }
        self.poll_load();
        if !self.show_splash && !self.header_initialized {
            self.header_initialized = true;
            #[cfg(target_os = "windows")]
            match native_window::install_header(frame) {
                Ok(()) => self.native_header = true,
                Err(error) => self.status = error,
            }
        }
        self.apply_backdrop(ctx, frame);
        if self.center_pending {
            if let Err(error) = native_window::center(frame) {
                self.status = format!("Window positioning: {error}");
            }
            self.center_pending = false;
        }
        if self.show_splash {
            self.splash(ctx, frame);
            return;
        }
        self.handle_close_request(ctx);
        let title = format!(
            "{}{} — Ada3D",
            self.path
                .as_ref()
                .and_then(|p| p.file_stem())
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| self.doc.title.clone()),
            if self.dirty { " •" } else { "" }
        );
        if title != self.window_title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.window_title = title;
        }
        self.menu_input_blocked = egui::Popup::is_any_open(ctx);
        self.shortcuts(ctx);
        self.header(ctx, frame);
        egui::TopBottomPanel::bottom("status")
            .frame(
                egui::Frame::new()
                    .fill(appearance::palette(ctx).panel)
                    .inner_margin(4),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(&self.status)
                            .small()
                            .color(appearance::palette(ctx).muted),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("Fit").clicked() {
                            self.zoom = 1.0;
                            self.pan = Vec2::ZERO;
                        }
                        ui.label(format!("{:.0}%", self.zoom * 100.0));
                        if ctx.screen_rect().width() < 750.0 {
                            ui.add(
                                egui::Slider::new(&mut self.zoom, 0.15..=8.0)
                                    .logarithmic(true)
                                    .show_value(false),
                            )
                            .on_hover_text("Workspace zoom");
                        }
                        ui.label(format!(
                            "{} × {} px",
                            self.doc.canvas.width, self.doc.canvas.height
                        ));
                        ui.label(format!("{} objects", self.doc.objects.len()));
                    });
                });
            });
        let settings_progress = ctx.animate_bool_with_time(
            egui::Id::new(crate::settings_ui::TRANSITION),
            self.show_settings,
            if self.preferences.reduce_motion {
                0.0
            } else {
                0.24
            },
        );
        if self.show_settings || settings_progress > 0.01 {
            self.settings_page(ctx);
        } else {
            self.inspector(ctx);
            self.canvas(ctx);
        }
        if self.show_welcome {
            self.welcome_screen(ctx);
        }
        if self.guide.was_closed() {
            self.show_help = false;
        }
        if self.show_help {
            self.guide.show(ctx, &self.preferences);
        }
        self.dialogs(ctx);
        self.loading_dialog(ctx);
        if let Some(dialog) = &self.export_dialog {
            dialog.show(ctx, &self.preferences);
        }
        if self.export_dialog.as_ref().is_some_and(|d| d.was_closed()) {
            self.export_dialog = None;
        }
        if let Some(started) = self.workspace_started
            && !self.preferences.reduce_motion
        {
            crate::ui::startup::reveal(ctx, frame, started.elapsed().as_secs_f32());
        }
        self.capture_frame(ctx, frame);
    }
}

fn canvas_point(point: Pos2, rect: Rect, width: u32, height: u32) -> [f32; 2] {
    [
        (point.x - rect.left()) / rect.width() * width as f32,
        (point.y - rect.top()) / rect.height() * height as f32,
    ]
}

fn pick_index(point: Pos2, rect: Rect, width: u32, height: u32) -> usize {
    let pixel = canvas_point(point, rect, width, height);
    let x = (pixel[0] as u32).min(width - 1);
    let y = (pixel[1] as u32).min(height - 1);
    (y * width + x) as usize
}

fn upload_texture(texture: &mut Option<egui::TextureHandle>, ctx: &egui::Context, raster: &Raster) {
    let image = egui::ColorImage::from_rgba_unmultiplied(
        [raster.width as usize, raster.height as usize],
        &raster.pixels,
    );
    if let Some(texture) = texture.as_mut() {
        texture.set(image, canvas_texture_options());
    } else {
        *texture = Some(ctx.load_texture("Ada3D canvas", image, canvas_texture_options()));
    }
}

fn canvas_texture_options() -> egui::TextureOptions {
    egui::TextureOptions {
        magnification: egui::TextureFilter::Nearest,
        minification: egui::TextureFilter::Linear,
        ..egui::TextureOptions::default()
    }
}

fn transform_row(
    ui: &mut egui::Ui,
    label: &str,
    values: &mut [f32; 3],
    speed: f64,
    range: std::ops::RangeInclusive<f32>,
    suffix: &str,
) {
    ui.label(egui::RichText::new(label).small());
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for (value, axis) in values.iter_mut().zip(["X ", "Y ", "Z "]) {
            ui.add_sized(
                [62.0, 24.0],
                egui::DragValue::new(value)
                    .speed(speed)
                    .range(range.clone())
                    .clamp_existing_to_range(false)
                    .update_while_editing(false)
                    .max_decimals(2)
                    .prefix(axis)
                    .suffix(suffix),
            );
        }
    });
}

#[cfg(test)]
mod tests;
