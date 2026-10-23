#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod app;
mod platform;
mod ui;
use adars::settings::{self, Backdrop, ThemeMode};
use platform::native_window;
use ui::{appearance, gallery, help, icons, settings as settings_ui};
fn main() -> eframe::Result {
    let mut args = std::env::args().skip(1);
    let mut capture = None;
    let mut capture_delay = None;
    let mut ui_scale = None;
    let mut demo = false;
    let mut blank = false;
    let mut settings_page = false;
    let mut welcome = false;
    let mut splash_preview = false;
    let mut theme = None;
    let mut backdrop = None;
    let mut transparency_preview = false;
    let mut startup_check = false;
    let mut guide = false;
    let mut selection_preview = false;
    let mut color_preview = false;
    let mut tooltip_preview = false;
    let mut menu_preview = false;
    let mut settings_tab = None;
    let mut context_preview = false;
    let mut paint_study = false;
    let mut export_preview = None;
    let mut export_check = false;
    let mut recent_preview = false;
    let mut pixel_preview = false;
    let mut scene_preview = None;
    let mut guide_topic = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--demo" => demo = true,
            "--scene-library" => scene_preview = Some(false),
            "--scene-paint" => scene_preview = Some(true),
            "--guide-topic" => guide_topic = args.next().and_then(|s| s.parse::<usize>().ok()),
            "--blank" => blank = true,
            "--capture" => capture = args.next().map(std::path::PathBuf::from),
            "--capture-delay" => capture_delay = args.next().and_then(|s| s.parse::<f64>().ok()),
            "--ui-scale" => ui_scale = args.next().and_then(|s| s.parse::<f32>().ok()),
            "--settings" => settings_page = true,
            "--welcome" => welcome = true,
            "--splash" => splash_preview = true,
            "--transparent" => transparency_preview = true,
            "--startup" => startup_check = true,
            "--guide" => guide = true,
            "--selection" => selection_preview = true,
            "--color" => color_preview = true,
            "--tooltip" => tooltip_preview = true,
            "--menu" => menu_preview = true,
            "--settings-tab" => {
                settings_tab = args.next().and_then(|value| match value.as_str() {
                    "appearance" => Some(0),
                    "workspace" => Some(1),
                    "about" => Some(2),
                    _ => None,
                });
            }
            "--context" => context_preview = true,
            "--paint-study" => paint_study = true,
            "--recent-projects" => recent_preview = true,
            "--pixel-art" => pixel_preview = true,
            "--export-check" => export_check = true,
            "--export-window" => {
                export_preview = args.next().map(|s| match s.as_str() {
                    "ico" => adars::export::Format::Ico,
                    "jpeg" => adars::export::Format::Jpeg,
                    "obj" => adars::export::Format::Obj,
                    _ => adars::export::Format::Png,
                });
            }
            "--backdrop" => backdrop = args.next(),
            "--theme" => {
                theme = args.next().and_then(|value| match value.as_str() {
                    "light" => Some(ThemeMode::Light),
                    "dark" => Some(ThemeMode::Dark),
                    "system" => Some(ThemeMode::System),
                    _ => None,
                })
            }
            _ => {}
        }
    }
    let startup = capture.is_none() || splash_preview || startup_check;
    let options = eframe::NativeOptions {
        centered: true,
        persist_window: false,
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Ada3D")
            .with_inner_size(if startup {
                [640.0, 460.0]
            } else {
                [1160.0, 740.0]
            })
            .with_min_inner_size(if startup {
                [520.0, 400.0]
            } else {
                [840.0, 560.0]
            })
            .with_icon(icons::window_icon())
            .with_app_id("Ada3D")
            .with_transparent(true)
            .with_decorations(!startup)
            .with_resizable(!startup)
            .with_close_button(!startup)
            .with_minimize_button(!startup)
            .with_maximize_button(!startup),
        ..Default::default()
    };
    let preference_path = if capture.is_some() {
        None
    } else {
        settings::default_path()
    };
    eframe::run_native(
        "Ada3D",
        options,
        Box::new(move |cc| {
            if let Some(scale) = ui_scale.filter(|s| s.is_finite()) {
                cc.egui_ctx.set_zoom_factor(scale.clamp(0.75, 2.0));
            }
            let mut studio = app::Studio::new(cc, preference_path);
            studio.configure_startup(startup);
            if demo || capture.is_some() && !settings_page && !welcome && !startup_check {
                studio.load_study();
            }
            if let Some(theme) = theme {
                studio.configure_theme(&cc.egui_ctx, theme);
            }
            if let Some(backdrop) = backdrop {
                match backdrop.as_str() {
                    "acrylic" => studio.configure_backdrop(&cc.egui_ctx, true, Backdrop::Acrylic),
                    "mica" => studio.configure_backdrop(&cc.egui_ctx, true, Backdrop::Mica),
                    _ => {}
                }
            }
            if let Some(path) = capture {
                studio.enable_capture(path);
            }
            if let Some(delay) = capture_delay {
                studio.configure_capture_delay(delay);
            }
            if blank {
                studio.open_blank_capture();
            }
            if welcome {
                studio.open_welcome_capture();
            }
            if transparency_preview {
                studio.open_transparency_capture();
            }
            if startup_check {
                studio.open_startup_capture();
            }
            if splash_preview {
                studio.open_splash_capture();
            }
            if paint_study {
                studio.open_paint_study_capture();
            }
            if recent_preview {
                studio.open_recent_capture();
            }
            if pixel_preview {
                studio.open_pixel_capture();
            }
            if let Some(format) = export_preview {
                studio.open_export_capture(format, export_check);
            }
            if context_preview {
                studio.open_context_capture();
            }
            if selection_preview {
                studio.open_selection_capture();
            }
            if color_preview {
                studio.open_color_capture(&cc.egui_ctx);
            }
            if tooltip_preview {
                studio.open_tooltip_capture();
            }
            if menu_preview {
                studio.open_menu_capture();
            }
            if guide {
                studio.open_guide_capture();
            }
            if settings_page {
                studio.open_settings_capture();
            }
            if let Some(paint) = scene_preview {
                studio.open_scene_capture(paint);
            }
            if let Some(topic) = guide_topic {
                studio.configure_guide_capture_topic(topic);
            }
            if let Some(tab) = settings_tab {
                studio.configure_settings_capture_tab(tab);
            }
            Ok(Box::new(studio))
        }),
    )
}
