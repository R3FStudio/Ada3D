//! Settings pages share a single card and row layout.
mod controls;
use crate::{
    appearance,
    icons::{self, Icon},
};
use adars::{
    settings::{Accent, AlphaBackground, Backdrop, Preferences, ThemeMode},
    updater::{Phase, Snapshot},
};
use eframe::egui::{self, RichText, Vec2};
pub const TRANSITION: &str = "settings-transition";
#[derive(Default, PartialEq, Eq)]
pub enum UpdateAction {
    #[default]
    None,
    Check,
    Install,
}
#[allow(clippy::too_many_arguments)]
pub fn show(
    ctx: &egui::Context,
    p: &mut Preferences,
    tab: &mut usize,
    notice: &str,
    material: &str,
    visible: bool,
    update: &Snapshot,
    action: &mut UpdateAction,
) -> bool {
    let duration = if p.reduce_motion { 0.0 } else { 0.24 };
    let progress = ctx.animate_bool_with_time(egui::Id::new(TRANSITION), visible, duration);
    let colors = appearance::palette(ctx);
    let mut back = false;
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(colors.panel).inner_margin(24))
        .show(ctx, |ui| {
            ui.set_opacity(progress);
            if !visible {
                ui.disable();
            }
            ui.spacing_mut().interact_size = Vec2::new(40.0, 34.0);
            ui.spacing_mut().button_padding = Vec2::new(14.0, 8.0);
            ui.spacing_mut().item_spacing = Vec2::new(12.0, 10.0);
            for style in [egui::TextStyle::Body, egui::TextStyle::Button] {
                ui.style_mut()
                    .text_styles
                    .insert(style, egui::FontId::proportional(14.0));
            }
            ui.horizontal_top(|ui| {
                let height = ui.available_height();
                ui.allocate_ui_with_layout(
                    Vec2::new(196.0, height),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.horizontal(|ui| {
                            back = icons::action(ui, Icon::Back, "")
                                .on_hover_text("Return to canvas")
                                .clicked();
                            ui.label(RichText::new("Settings").size(20.0));
                        });
                        ui.add_space(30.0);
                        for (index, (icon, title)) in [
                            (Icon::Settings, "Appearance"),
                            (Icon::Canvas, "Workspace"),
                            (Icon::Help, "About"),
                        ]
                        .iter()
                        .enumerate()
                        {
                            if controls::navigation(ui, *icon, title, *tab == index).clicked() {
                                *tab = index;
                            }
                        }
                    },
                );
                ui.add_space(12.0 + (1.0 - progress) * 12.0);
                ui.allocate_ui_with_layout(
                    Vec2::new(ui.available_width().min(860.0), height),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        let tab_id = egui::Id::new("settings-page-fade");
                        let now = ctx.input(|i| i.time);
                        let (old, start) = ctx
                            .data(|d| d.get_temp::<(usize, f64)>(tab_id))
                            .unwrap_or((*tab, now));
                        let start = if old != *tab { now } else { start };
                        ctx.data_mut(|d| d.insert_temp(tab_id, (*tab, start)));
                        let fade = if p.reduce_motion {
                            1.0
                        } else {
                            ((now - start) / 0.18).clamp(0.0, 1.0) as f32
                        };
                        if fade < 1.0 {
                            ctx.request_repaint();
                        }
                        ui.set_opacity(progress * fade);
                        egui::ScrollArea::vertical()
                            .id_salt("settings-pages")
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width().max(320.0));
                                let title = ["Appearance", "Workspace", "About"][(*tab).min(2)];
                                ui.label(RichText::new(title).size(28.0).strong());
                                ui.add_space(12.0);
                                match *tab {
                                    0 => appearance_page(ui, p, material),
                                    1 => workspace_page(ui, p),
                                    _ => {
                                        about_page(ui, p, update, action);
                                    }
                                }
                                if *tab < 2 {
                                    ui.add_space(8.0);
                                    ui.horizontal(|ui| {
                                        if ui.button("Restore defaults").clicked() {
                                            let recent = std::mem::take(&mut p.recent_projects);
                                            *p = Preferences::default();
                                            p.recent_projects = recent;
                                        }
                                        ui.label(
                                            RichText::new(notice).size(12.0).color(colors.muted),
                                        );
                                    });
                                }
                            });
                    },
                );
            });
        });
    back
}
fn card(ui: &mut egui::Ui, title: &str, subtitle: &str, contents: impl FnOnce(&mut egui::Ui)) {
    let colors = appearance::palette(ui.ctx());
    let fill = if ui.visuals().dark_mode {
        egui::Color32::from_rgb(39, 39, 39)
    } else {
        egui::Color32::from_rgb(255, 255, 255)
    };
    egui::Frame::new()
        .fill(fill)
        .stroke(egui::Stroke::new(
            1.0_f32,
            colors.border.linear_multiply(0.6),
        ))
        .corner_radius(9)
        .inner_margin(16)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(title).size(16.0).strong());
            if !subtitle.is_empty() {
                ui.label(RichText::new(subtitle).size(12.0).color(colors.muted));
            }
            ui.add_space(8.0);
            contents(ui);
        });
    ui.add_space(0.0);
}
fn appearance_page(ui: &mut egui::Ui, p: &mut Preferences, material: &str) {
    card(
        ui,
        "Theme",
        "Use your device theme or choose a light or dark workspace.",
        |ui| {
            let width = (ui.available_width() - 24.0) / 3.0;
            ui.horizontal(|ui| {
                for (value, title, description, preview) in [
                    (ThemeMode::System, "System", "Follow your device", None),
                    (ThemeMode::Light, "Light", "Light surfaces", Some(false)),
                    (ThemeMode::Dark, "Dark", "Dark surfaces", Some(true)),
                ] {
                    if controls::choice(
                        ui,
                        width,
                        title,
                        description,
                        p.theme == value,
                        Some(preview),
                    )
                    .clicked()
                    {
                        p.theme = value;
                    }
                }
            });
        },
    );
    card(ui, "Window material", "", |ui| {
        if cfg!(target_os = "windows") {
            let width = (ui.available_width() - 12.0) / 2.0;
            ui.horizontal(|ui| {
                for (value, title, description) in [
                    (Backdrop::Mica, "Mica", "Subtle wallpaper tint"),
                    (Backdrop::Acrylic, "Acrylic", "Blurred desktop colors"),
                ] {
                    if controls::choice(ui, width, title, description, p.backdrop == value, None)
                        .clicked()
                    {
                        p.backdrop = value;
                    }
                }
            });
        } else {
            ui.label(if cfg!(target_os = "macos") {
                "Frosted glass uses your Mac’s native window material."
            } else {
                "Acrylic · blurred desktop colors"
            });
        }
        if material.contains("unavailable") {
            ui.label(RichText::new("Your desktop doesn’t support background blur.").small());
        }
    });
    card(
        ui,
        "Accent color",
        "Used for selected tools, highlights and controls.",
        |ui| {
            let width = (ui.available_width() - 24.0) / 3.0;
            ui.horizontal(|ui| {
                for (value, title) in [
                    (Accent::Blue, "Blue"),
                    (Accent::Violet, "Violet"),
                    (Accent::Teal, "Teal"),
                ] {
                    let label = title;
                    if ui
                        .add_sized(
                            [width, 36.0],
                            egui::Button::new(RichText::new(label).color(egui::Color32::WHITE))
                                .fill(appearance::accent(value))
                                .stroke(egui::Stroke::new(
                                    if p.accent == value { 2.0_f32 } else { 0.0_f32 },
                                    ui.visuals().text_color(),
                                )),
                        )
                        .clicked()
                    {
                        p.accent = value;
                    }
                }
            });
        },
    );
}
fn workspace_page(ui: &mut egui::Ui, p: &mut Preferences) {
    let duration = if p.reduce_motion { 0.0 } else { 0.22 };
    card(ui, "Canvas display", "", |ui| {
        ui.label("Show transparent pixels as");
        let width = (ui.available_width() - 12.0) / 2.0;
        ui.horizontal(|ui| {
            for (value, title, description) in [
                (
                    AlphaBackground::Workspace,
                    "Workspace",
                    "See the app background",
                ),
                (
                    AlphaBackground::Checkerboard,
                    "Checkerboard",
                    "Show a transparency grid",
                ),
            ] {
                if controls::choice(
                    ui,
                    width,
                    title,
                    description,
                    p.alpha_background == value,
                    None,
                )
                .clicked()
                {
                    p.alpha_background = value;
                }
            }
        });
        ui.add_space(4.0);
        controls::toggle(
            ui,
            &mut p.workspace_grid,
            "Workspace grid",
            "Show dots behind the canvas.",
            duration,
        );
        ui.separator();
        controls::toggle(
            ui,
            &mut p.brush_cursor,
            "Brush-size cursor",
            "Preview the diameter of your stroke.",
            duration,
        );
    });
    card(ui, "Startup and accessibility", "", |ui| {
        controls::toggle(
            ui,
            &mut p.show_welcome,
            "Show Welcome",
            "Choose a new canvas or open a recent project.",
            duration,
        );
        ui.separator();
        controls::toggle(
            ui,
            &mut p.reduce_motion,
            "Reduce motion",
            "Turn off interface transitions.",
            duration,
        );
    });
}
fn updates_page(
    ui: &mut egui::Ui,
    p: &mut Preferences,
    update: &Snapshot,
    action: &mut UpdateAction,
) {
    card(ui, "Software updates", "Keep Ada3D up to date.", |ui| {
        ui.label(match update.phase {
            Phase::Idle => "Check for a newer release of Ada3D.",
            Phase::NotConfigured => "Updates are unavailable right now. Please try again later.",
            Phase::Error => "Couldn’t finish the update. Please try again later.",
            _ => &update.message,
        });
        if update.busy() {
            ui.add(
                egui::ProgressBar::new(update.progress.unwrap_or(0.0))
                    .animate(true)
                    .show_percentage(),
            );
        }
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui
                .add_enabled(!update.busy(), egui::Button::new("Check for updates"))
                .clicked()
            {
                *action = UpdateAction::Check;
            }
            if update.phase == Phase::Ready && ui.button("Restart and install").clicked() {
                *action = UpdateAction::Install;
            }
        });
        ui.add_space(8.0);
        ui.separator();
        let duration = if p.reduce_motion { 0.0 } else { 0.22 };
        controls::toggle(
            ui,
            &mut p.check_updates_on_startup,
            "Check when Ada3D starts",
            "Look for a new version when you open the app.",
            duration,
        );
    });
}

fn about_page(
    ui: &mut egui::Ui,
    p: &mut Preferences,
    update: &Snapshot,
    action: &mut UpdateAction,
) {
    card(ui, "Ada3D", "", |ui| {
        ui.horizontal(|ui| {
            icons::logo(ui, 56.0);
            ui.vertical(|ui| {
                ui.strong("R3F Studio");
                ui.label(concat!("Version ", env!("CARGO_PKG_VERSION")));
                ui.label("© 2026 R3F Studio");
                ui.hyperlink_to(
                    "Visit Ada3D on GitHub",
                    "https://github.com/R3FStudio/Ada3D",
                );
            });
        });
    });
    updates_page(ui, p, update, action);
    card(
        ui,
        "License",
        "GNU General Public License, version 3",
        |ui| {
            ui.label("Free software. You may use, study, modify and redistribute Ada3D under GPLv3. Provided without warranty.");
            ui.hyperlink_to("Read GPLv3", "https://www.gnu.org/licenses/gpl-3.0.html");
        },
    );
}
