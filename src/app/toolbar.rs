//! Title-bar commands and tool navigation.
use super::*;

impl Studio {
    pub(super) fn header(&mut self, ctx: &egui::Context, frame: &eframe::Frame) {
        let colors = appearance::palette(ctx);
        let top_inset = native_window::header_top_inset(frame, ctx.pixels_per_point());
        let mut drag_regions = Vec::new();
        egui::TopBottomPanel::top("integrated-title")
            .exact_height(38.0 + top_inset)
            .frame(
                egui::Frame::new()
                    .fill(Color32::TRANSPARENT)
                    .inner_margin(egui::Margin {
                        left: 8,
                        right: 8,
                        top: (4.0 + top_inset).round() as i8,
                        bottom: 4,
                    }),
            )
            .show(ctx, |ui| {
                ui.set_clip_rect(
                    ui.clip_rect()
                        .intersect(native_window::header_content_rect(ctx, frame)),
                );
                native_window::paint_header_background(
                    ctx,
                    frame,
                    ui.painter(),
                    colors.panel,
                    self.backdrop_active && self.preferences.backdrop == Backdrop::Acrylic,
                );
                if self.loading.is_some()
                    || self.pending.is_some()
                    || self.error.is_some()
                    || self.show_welcome
                {
                    ui.disable();
                }
                ui.scope(|ui| {
                    ui.set_max_width(
                        (native_window::header_content_rect(ctx, frame).right()
                            - ui.cursor().left()
                            - 8.0)
                            .max(0.0),
                    );
                    let compact = ui.available_width() < 730.0;
                    ui.horizontal(|ui| {
                        icons::title_logo(ui);
                        if icons::action(ui, Icon::New, "")
                            .on_hover_text("New · Ctrl+N")
                            .clicked()
                        {
                            self.request(Pending::New, ctx);
                        }
                        let open = icons::action(ui, Icon::Open, "").on_hover_text("Open · Ctrl+O");
                        self.open_rect = open.rect;
                        if open.clicked() {
                            self.request(Pending::Open, ctx);
                        }
                        if icons::action(ui, Icon::Import, "")
                            .on_hover_text("Import image")
                            .clicked()
                        {
                            self.import();
                        }
                        let menu = icons::action(ui, Icon::Menu, "Menu");
                        let popup = egui::Popup::menu(&menu);
                        if self.capture_menu {
                            popup.open(true).show(|ui| self.project_menu(ui, ctx));
                        } else {
                            popup.show(|ui| self.project_menu(ui, ctx));
                        }
                        if ui
                            .add_enabled_ui(self.history.can_undo(), |ui| {
                                icons::action(ui, Icon::Undo, "")
                            })
                            .inner
                            .on_hover_text("Ctrl+Z")
                            .clicked()
                        {
                            self.undo(false);
                        }
                        if ui
                            .add_enabled_ui(self.history.can_redo(), |ui| {
                                icons::action(ui, Icon::Redo, "")
                            })
                            .inner
                            .on_hover_text("Ctrl+Shift+Z")
                            .clicked()
                        {
                            self.undo(true);
                        }
                        ui.separator();
                        let title = self
                            .path
                            .as_ref()
                            .and_then(|p| p.file_stem())
                            .map(|s| s.to_string_lossy().into_owned())
                            .unwrap_or_else(|| self.doc.title.clone());
                        let labels = if compact {
                            ["", "", "Export", ""]
                        } else {
                            ["Settings", "Help", "Export", "Save"]
                        };
                        let action_width: f32 = labels
                            .iter()
                            .map(|label| icons::action_width(ui, label))
                            .sum();
                        let title_width = (ui.available_width()
                            - action_width
                            - ui.spacing().item_spacing.x * 4.0)
                            .max(0.0);
                        let title_response = ui.add_sized(
                            [title_width, 26.0],
                            egui::Label::new(
                                egui::RichText::new(format!(
                                    "{}{}",
                                    title,
                                    if self.dirty { " •" } else { "" }
                                ))
                                .color(colors.muted),
                            )
                            .truncate(),
                        );
                        drag_regions.push([
                            title_response.rect.left(),
                            4.0 + top_inset,
                            title_response.rect.right(),
                            37.0 + top_inset,
                        ]);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let settings = icons::action(
                                ui,
                                Icon::Settings,
                                if compact { "" } else { "Settings" },
                            )
                            .on_hover_text("Settings");
                            self.settings_rect = settings.rect;
                            if settings.clicked() {
                                self.finish_edit();
                                self.show_settings = !self.show_settings;
                            }
                            if icons::action(ui, Icon::Help, if compact { "" } else { "Help" })
                                .on_hover_text("Help")
                                .clicked()
                            {
                                self.finish_edit();
                                self.guide.reset();
                                self.show_help = true;
                            }
                            if icons::action(ui, Icon::Export, "Export").clicked() {
                                self.export(self.scene_view);
                            }
                            if icons::action(ui, Icon::Save, if compact { "" } else { "Save" })
                                .on_hover_text("Save · Ctrl+S")
                                .clicked()
                            {
                                self.save(false);
                            }
                        });
                    });
                });
            });
        native_window::header_regions(frame, ctx.pixels_per_point(), drag_regions);
        egui::TopBottomPanel::top("compact-tools")
            .exact_height(32.0)
            .frame(
                egui::Frame::new()
                    .fill(colors.panel)
                    .inner_margin(egui::Margin::symmetric(8, 2)),
            )
            .show(ctx, |ui| {
                if self.loading.is_some()
                    || self.pending.is_some()
                    || self.error.is_some()
                    || self.show_welcome
                {
                    ui.disable();
                }
                ui.horizontal(|ui| {
                    for (tab, icon, label) in [
                        (Tab::Paint, Icon::Brush, "Brushes"),
                        (Tab::Shapes, Icon::Shapes, "2D shapes"),
                        (Tab::Objects, Icon::Cube, "3D shapes"),
                        (Tab::Canvas, Icon::Canvas, "Canvas"),
                    ] {
                        let response =
                            icons::ribbon(ui, icon, label, !self.show_settings && self.tab == tab);
                        if tab == Tab::Paint {
                            self.paint_tab_rect = response.rect;
                        }
                        if tab == Tab::Objects {
                            self.objects_tab_rect = response.rect;
                        }
                        if response.clicked() {
                            self.finish_edit();
                            self.show_settings = false;
                            if tab == Tab::Paint && self.scene_view {
                                self.set_scene_paint_tool(Tool::Brush);
                                continue;
                            }
                            self.tab = tab;
                            match tab {
                                Tab::Paint => self.set_tool(Tool::Brush),
                                Tab::Shapes => self.set_tool(Tool::Rectangle),
                                Tab::Objects => {
                                    self.scene_paint_tools = false;
                                    self.set_tool(Tool::Select);
                                }
                                Tab::Canvas => {}
                            }
                        }
                    }
                    ui.separator();
                    if ui
                        .selectable_label(!self.scene_view, "2D")
                        .on_hover_text("2D canvas")
                        .clicked()
                    {
                        self.finish_edit();
                        self.scene_view = false;
                        if self.tab == Tab::Objects {
                            self.tab = Tab::Paint;
                            self.tool = Tool::Brush;
                        }
                        self.render_dirty = true;
                    }
                    if ui
                        .selectable_label(self.scene_view, "3D")
                        .on_hover_text("3D scene")
                        .clicked()
                    {
                        self.tab = Tab::Objects;
                        self.scene_paint_tools = false;
                        self.set_tool(Tool::Select);
                    }
                    if icons::action(ui, Icon::Select, "")
                        .on_hover_text("Select pixels or objects")
                        .clicked()
                    {
                        self.finish_edit();
                        self.tool = Tool::Select;
                        if self.scene_view {
                            self.scene_paint_tools = false;
                        }
                    }
                    if ui.small_button("Fit").clicked() {
                        self.zoom = 1.0;
                        self.pan = Vec2::ZERO;
                    }
                    if ctx.screen_rect().width() >= 750.0 {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(format!("{:.0}%", self.zoom * 100.0));
                            ui.add(
                                egui::Slider::new(&mut self.zoom, 0.15..=8.0)
                                    .logarithmic(true)
                                    .show_value(false),
                            )
                            .on_hover_text("Workspace zoom");
                        });
                    }
                });
            });
    }
}
