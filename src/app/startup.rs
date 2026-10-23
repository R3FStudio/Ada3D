//! Startup readiness, welcome screen and window material.
use super::*;

impl Studio {
    pub fn configure_startup(&mut self, splash: bool) {
        self.show_splash = splash;
    }

    pub(super) fn apply_backdrop(&mut self, ctx: &egui::Context, frame: &eframe::Frame) {
        let key = (
            self.preferences.glass_effects,
            self.preferences.backdrop,
            ctx.theme() == egui::Theme::Dark,
        );
        if self.backdrop_applied == Some(key) {
            ctx.data_mut(|d| d.insert_temp(egui::Id::new("glass"), self.backdrop_active));
            return;
        }
        let result = native_window::configure(frame, key.0, key.1, key.2).and_then(|()| {
            if self.show_splash {
                native_window::splash_frame(frame)?;
            }
            Ok(())
        });
        self.backdrop_active = result.is_ok() && key.0;
        ctx.data_mut(|d| d.insert_temp(egui::Id::new("glass"), self.backdrop_active));
        self.backdrop_notice = match result {
            Ok(()) if key.0 => format!("{:?} · native title bar · full-window backdrop", key.1),
            Ok(()) => "Solid background · native title bar".into(),
            Err(e) => format!("Native backdrop unavailable: {e}"),
        };
        self.backdrop_applied = Some(key);
    }

    pub(super) fn splash(&mut self, ctx: &egui::Context, frame: &eframe::Frame) {
        if !self.startup_update_checked {
            self.startup_update_checked = true;
            if self.capture_path.is_none()
                && !cfg!(test)
                && self.preferences.check_updates_on_startup
            {
                self.check_updates(ctx, true);
            }
        }
        let update = self.updater.snapshot();
        if update.phase == adars::updater::Phase::Ready && !self.startup_update_installing {
            self.startup_update_installing = true;
            self.install_update(ctx);
        }
        self.refresh_texture(ctx);
        let started = *self
            .startup_started
            .get_or_insert_with(std::time::Instant::now);
        let elapsed = started.elapsed().as_secs_f32();
        let reduced = self.preferences.reduce_motion;
        let target = if self.texture.is_some() { 1.0 } else { 0.35 };
        let dt = ctx.input(|i| i.stable_dt).clamp(0.0, 0.05);
        self.startup_progress = if reduced {
            target
        } else {
            (self.startup_progress + dt * 1.8).min(target)
        };
        if !update.busy()
            && update.phase != adars::updater::Phase::Ready
            && self.texture.is_some()
            && self.startup_progress >= 1.0
            && (reduced || elapsed >= 0.9)
            && !self.splash_capture
        {
            self.splash_exit_started
                .get_or_insert_with(std::time::Instant::now);
        }
        let exit = self.splash_exit_started.map(|s| s.elapsed().as_secs_f32());
        let alpha = crate::ui::startup::splash_opacity(elapsed, exit, reduced);
        let (progress, status) = if update.busy() {
            (update.progress.unwrap_or(0.0), update.message.as_str())
        } else if update.phase == adars::updater::Phase::Ready {
            (1.0, "Updating - restarting Ada3D...")
        } else {
            (self.startup_progress, "")
        };
        crate::ui::startup::paint(ctx, progress, alpha, elapsed, reduced, status);
        if exit.is_some_and(|t| reduced || t >= crate::ui::startup::FADE_SECONDS) {
            self.show_splash = false;
            self.workspace_started = Some(std::time::Instant::now());
            ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Resizable(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::EnableButtons {
                close: true,
                minimized: true,
                maximize: true,
            });
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(1160.0, 740.0)));
            ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(Vec2::new(840.0, 560.0)));
            self.center_pending = true;
        }
        self.capture_frame(ctx, frame);
        ctx.request_repaint_after(std::time::Duration::from_millis(16));
    }

    pub(super) fn welcome_screen(&mut self, ctx: &egui::Context) {
        let mut action = 0;
        let mut recent = None;
        let recent_count = self.preferences.recent_projects.len();
        let before = self.preferences.show_welcome;
        egui::Modal::new(egui::Id::new("welcome"))
            .frame(
                egui::Frame::new()
                    .fill(ctx.style().visuals.window_fill)
                    .corner_radius(16)
                    .inner_margin(22)
                    .stroke(Stroke::new(1.0_f32, appearance::palette(ctx).border)),
            )
            .show(ctx, |ui| {
                ui.set_width(488.0);
                ui.vertical_centered(|ui| {
                    icons::logo(ui, 40.0);
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new("Welcome").size(24.0));
                    ui.label(
                        egui::RichText::new("A blank canvas. A new dimension.")
                            .color(appearance::palette(ctx).muted),
                    );
                });
                ui.add_space(18.0);
                ui.horizontal(|ui| {
                    for (id, icon, label, hint) in [
                        (1, Icon::New, "New", "Start a fresh canvas"),
                        (2, Icon::Open, "Open", "Continue a project"),
                        (3, Icon::Canvas, "Import image", "Bring in a PNG or JPEG"),
                    ] {
                        let (rect, response) =
                            ui.allocate_exact_size(Vec2::new(157.0, 100.0), Sense::click());
                        ui.painter().rect_filled(
                            rect,
                            14.0,
                            if response.hovered() {
                                ui.visuals().widgets.hovered.bg_fill
                            } else {
                                ui.visuals().faint_bg_color
                            },
                        );
                        if id == 1 {
                            appearance::rounded_gradient(
                                ui.painter(),
                                rect,
                                14.0,
                                appearance::accent(self.preferences.accent),
                                Color32::from_rgb(118, 73, 218),
                            );
                        }
                        ui.painter().rect_stroke(
                            rect,
                            14.0,
                            Stroke::new(
                                1.0_f32,
                                if response.hovered() {
                                    appearance::accent(self.preferences.accent)
                                } else {
                                    appearance::palette(ctx).border
                                },
                            ),
                            egui::StrokeKind::Inside,
                        );
                        icons::draw(
                            ui.painter(),
                            icon,
                            Rect::from_center_size(
                                rect.center_top() + Vec2::new(0.0, 28.0),
                                Vec2::splat(26.0),
                            ),
                            if id == 1 {
                                Color32::WHITE
                            } else {
                                appearance::accent(self.preferences.accent)
                            },
                        );
                        ui.painter().text(
                            rect.center_top() + Vec2::new(0.0, 60.0),
                            egui::Align2::CENTER_CENTER,
                            label,
                            egui::FontId::proportional(15.0),
                            if id == 1 {
                                Color32::WHITE
                            } else {
                                ui.visuals().text_color()
                            },
                        );
                        ui.painter().text(
                            rect.center_top() + Vec2::new(0.0, 82.0),
                            egui::Align2::CENTER_CENTER,
                            hint,
                            egui::FontId::proportional(12.0),
                            if id == 1 {
                                Color32::from_rgb(225, 231, 255)
                            } else {
                                appearance::palette(ctx).muted
                            },
                        );
                        if response.clicked() {
                            action = id;
                        }
                    }
                });
                ui.add_space(18.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Recent projects").strong());
                    if !self.preferences.recent_projects.is_empty()
                        && ui.small_button("Clear list").clicked()
                    {
                        self.preferences.recent_projects.clear();
                    }
                });
                if self.preferences.recent_projects.is_empty() {
                    ui.label(
                        egui::RichText::new("Projects you save or open will appear here.")
                            .color(appearance::palette(ctx).muted),
                    );
                } else {
                    egui::ScrollArea::vertical()
                        .max_height(210.0)
                        .show(ui, |ui| {
                            for path in &self.preferences.recent_projects {
                                let (rect, response) = ui.allocate_exact_size(
                                    Vec2::new(ui.available_width(), 52.0),
                                    Sense::click(),
                                );
                                if response.hovered() {
                                    ui.painter().rect_filled(
                                        rect,
                                        8.0,
                                        ui.visuals().widgets.hovered.bg_fill,
                                    );
                                }
                                let logo = icons::logo_texture(ctx);
                                ui.painter().image(
                                    logo.id(),
                                    Rect::from_center_size(
                                        rect.left_center() + Vec2::new(21.0, 0.0),
                                        Vec2::splat(26.0),
                                    ),
                                    Rect::from_min_max(Pos2::ZERO, egui::pos2(1.0, 1.0)),
                                    Color32::WHITE,
                                );
                                ui.painter().text(
                                    rect.left_top() + Vec2::new(45.0, 16.0),
                                    egui::Align2::LEFT_CENTER,
                                    path.file_stem().unwrap_or_default().to_string_lossy(),
                                    egui::FontId::proportional(13.0),
                                    ui.visuals().text_color(),
                                );
                                let text = ui.painter().layout_no_wrap(
                                    path.to_string_lossy().into_owned(),
                                    egui::FontId::proportional(10.0),
                                    appearance::palette(ctx).muted,
                                );
                                ui.painter().with_clip_rect(rect).galley(
                                    rect.left_top() + Vec2::new(45.0, 31.0),
                                    text,
                                    appearance::palette(ctx).muted,
                                );
                                if response.clicked() {
                                    recent = Some(path.clone());
                                }
                            }
                        });
                }
                ui.add_space(18.0);
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.preferences.show_welcome, "Show on startup");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Continue to canvas").clicked() {
                            action = 4;
                        }
                    });
                });
            });
        if before != self.preferences.show_welcome
            || recent_count != self.preferences.recent_projects.len()
        {
            self.persist_preferences(ctx);
        }
        if let Some(path) = recent {
            self.request(Pending::OpenPath(path), ctx);
        }
        match action {
            1 => self.request(Pending::New, ctx),
            2 => self.request(Pending::Open, ctx),
            3 => {
                self.import();
            }
            4 => self.show_welcome = false,
            _ => {}
        }
    }
}
