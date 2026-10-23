//! Brush, palette, scene and canvas property panels.
use super::*;

const SWATCH_SIZE: Vec2 = Vec2::new(38.0, 28.0);
const SWATCH_SPACING: f32 = 4.0;

impl Studio {
    pub(super) fn palette(&mut self, ui: &mut egui::Ui) {
        let palette_width = 6.0 * SWATCH_SIZE.x + 5.0 * SWATCH_SPACING;
        let inset = ((ui.available_width() - palette_width) * 0.5).max(0.0);
        ui.horizontal(|ui| {
            ui.add_space(inset);
            ui.allocate_ui_with_layout(
                Vec2::new(palette_width, 0.0),
                egui::Layout::top_down(egui::Align::Min),
                |ui| self.palette_controls(ui),
            );
        });
    }

    fn palette_controls(&mut self, ui: &mut egui::Ui) {
        ui.style_mut().override_font_id = Some(egui::FontId::proportional(13.0));
        ui.label("Color");
        ui.horizontal(|ui| {
            if ui
                .add(
                    egui::Button::new(egui::RichText::new("Choose color").color(
                        if self.color[0] as u32 * 299
                            + self.color[1] as u32 * 587
                            + self.color[2] as u32 * 114
                            > 140000
                        {
                            Color32::BLACK
                        } else {
                            Color32::WHITE
                        },
                    ))
                    .min_size(Vec2::new(96.0, 30.0))
                    .fill(Color32::from_rgb(
                        self.color[0],
                        self.color[1],
                        self.color[2],
                    )),
                )
                .clicked()
            {
                self.show_color = true;
            }
            ui.label(format!(
                "#{:02X}{:02X}{:02X}",
                self.color[0], self.color[1], self.color[2]
            ));
        });
        let colors = [
            [255, 255, 255],
            [197, 200, 209],
            [103, 109, 122],
            [30, 34, 44],
            [157, 22, 58],
            [236, 54, 73],
            [255, 137, 39],
            [253, 191, 45],
            [255, 228, 100],
            [135, 194, 47],
            [43, 156, 105],
            [0, 143, 144],
            [109, 226, 228],
            [44, 183, 235],
            [45, 104, 238],
            [95, 75, 211],
            [164, 61, 199],
            [235, 120, 172],
            [248, 198, 177],
            [189, 121, 96],
            [116, 74, 57],
            [196, 158, 115],
            [172, 193, 166],
            [106, 137, 173],
        ];
        ui.add_space(6.0);
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing.y = SWATCH_SPACING;
            for row in colors.chunks(6) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = SWATCH_SPACING;
                    for &[r, g, b] in row {
                        let (rect, response) = ui.allocate_exact_size(SWATCH_SIZE, Sense::click());
                        ui.painter()
                            .rect_filled(rect.shrink(1.0), 2.0, Color32::from_rgb(r, g, b));
                        if self.color == [r, g, b, 255] {
                            ui.painter().rect_stroke(
                                rect.shrink(2.0),
                                2.0,
                                Stroke::new(2.0_f32, ui.visuals().text_color()),
                                egui::StrokeKind::Inside,
                            );
                        }
                        if response.clicked() {
                            self.color = [r, g, b, 255];
                        }
                    }
                });
            }
        });
    }

    pub(super) fn inspector(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("inspector")
            .exact_width(if self.scene_view && self.tab == Tab::Objects {
                302.0
            } else {
                286.0
            })
            .frame(
                egui::Frame::new()
                    .fill(appearance::palette(ctx).panel)
                    .inner_margin(14),
            )
            .show(ctx, |ui| {
                if self.loading.is_some() || self.pending.is_some() || self.error.is_some() {
                    ui.disable();
                }
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.add_space(6.0);
                    ui.heading(self.inspector_title());
                    ui.label(
                        egui::RichText::new(match self.tab {
                            Tab::Paint => "Draw on the canvas",
                            Tab::Shapes => "Drag to draw a shape",
                            Tab::Objects if self.scene_paint_tools => {
                                "Paint directly on object surfaces"
                            }
                            Tab::Objects => "Click a shape or drag it into your scene",
                            Tab::Canvas => "Size and transparency",
                        })
                        .size(10.0)
                        .color(appearance::palette(ctx).muted),
                    );
                    ui.add_space(8.0);
                    match self.tab {
                        Tab::Paint => self.paint_inspector(ui),
                        Tab::Shapes => self.shapes_inspector(ui),
                        Tab::Objects => self.scene_inspector(ui, ctx),
                        Tab::Canvas => self.canvas_inspector(ui),
                    }
                });
            });
    }
    fn inspector_title(&self) -> &'static str {
        match self.tab {
            Tab::Paint => match self.tool {
                Tool::Eraser => "Eraser",
                Tool::Fill => "Fill",
                Tool::Dropper => "Color picker",
                Tool::Select => "Selection",
                _ => match self.brush_preset {
                    1 => "Pencil",
                    2 => "Marker",
                    _ => "Brush",
                },
            },
            Tab::Shapes => "2D shapes",
            Tab::Objects if self.scene_paint_tools => "3D paint",
            Tab::Objects => "3D shapes",
            Tab::Canvas => "Canvas",
        }
    }
    fn shapes_inspector(&mut self, ui: &mut egui::Ui) {
        for (tool, label) in [
            (Tool::Line, "Line"),
            (Tool::Rectangle, "Rectangle"),
            (Tool::Ellipse, "Ellipse"),
        ] {
            if ui.selectable_label(self.tool == tool, label).clicked() {
                self.set_tool(tool);
            }
        }
        ui.add_space(8.0);
        ui.checkbox(&mut self.filled, "Filled shape");
        ui.add(egui::Slider::new(&mut self.radius, 0.5..=40.0).text("Stroke radius"));
        ui.separator();
        self.palette(ui);
    }
    fn canvas_inspector(&mut self, ui: &mut egui::Ui) {
        ui.label("Dimensions");
        ui.add(
            egui::DragValue::new(&mut self.canvas_width)
                .range(1..=2048)
                .prefix("Width   ")
                .suffix(" px"),
        );
        ui.add(
            egui::DragValue::new(&mut self.canvas_height)
                .range(1..=2048)
                .prefix("Height  ")
                .suffix(" px"),
        );
        if ui.button("Resize canvas").clicked() {
            self.finish_edit();
            self.history.begin(&self.doc);
            self.doc
                .canvas
                .resize(self.canvas_width, self.canvas_height);
            self.history.commit(&self.doc);
            self.changed();
        }
        ui.label(
            egui::RichText::new(
                "Keeps the top-left area. Added space is transparent. Maximum 2048 × 2048.",
            )
            .small()
            .color(Color32::GRAY),
        );
        ui.separator();
        if ui.button("Clear to white").clicked() {
            self.clear([255; 4]);
        }
        if ui.button("Clear to transparent").clicked() {
            self.clear([0; 4]);
        }
        if ui.button("Import image…").clicked() {
            self.import();
        }
        ui.separator();
        ui.label("View");
        if ui.button("Fit canvas").clicked() {
            self.zoom = 1.0;
            self.pan = Vec2::ZERO;
        }
        ui.label(
            egui::RichText::new("Scroll to zoom. Middle-drag to pan.")
                .small()
                .color(Color32::GRAY),
        );
    }

    pub(super) fn paint_inspector(&mut self, ui: &mut egui::Ui) {
        if self.tool == Tool::Select {
            ui.label("Drag a box to select pixels.");
            ui.add_space(12.0);
            ui.label("Drag inside to move. Corner dots resize both dimensions; edge dots resize one side. The circular arrow rotates.");
            ui.add_space(12.0);
            ui.label("Each transform is one undo step.");
            return;
        }
        let tools = [
            (Tool::Brush, Icon::Brush, "Brush", 0),
            (Tool::Brush, Icon::Pencil, "Pencil", 1),
            (Tool::Brush, Icon::Marker, "Marker", 2),
            (Tool::Eraser, Icon::Eraser, "Eraser", 0),
            (Tool::Fill, Icon::Fill, "Bucket", 0),
            (Tool::Dropper, Icon::Dropper, "Picker", 0),
        ];
        for row in tools.chunks(3) {
            ui.horizontal(|ui| {
                for &(tool, icon, label, preset) in row {
                    let selected =
                        self.tool == tool && (tool != Tool::Brush || self.brush_preset == preset);
                    if icons::tile(ui, icon, label, selected).clicked() {
                        let in_scene = self.scene_view && self.tab == Tab::Objects;
                        if tool == Tool::Brush {
                            self.select_brush_preset(preset);
                        } else {
                            self.set_tool(tool);
                        }
                        if in_scene {
                            self.scene_view = true;
                            self.tab = Tab::Objects;
                            self.scene_paint_tools = true;
                        }
                    }
                }
            });
        }
        ui.add_space(16.0);
        if self.tool != Tool::Eraser && self.tool != Tool::Dropper {
            let before = self.transparent_paint;
            ui.horizontal(|ui| {
                ui.label("Paint");
                ui.selectable_value(&mut self.transparent_paint, false, "Color");
                ui.selectable_value(&mut self.transparent_paint, true, "Transparency");
            });
            if before != self.transparent_paint {
                self.finish_edit();
            }
        }
        ui.add_space(12.0);
        if matches!(self.tool, Tool::Brush | Tool::Eraser)
            || self.tool == Tool::Fill && self.transparent_paint
        {
            egui::Frame::new()
                .fill(ui.visuals().faint_bg_color)
                .corner_radius(10)
                .inner_margin(10)
                .show(ui, |ui| {
                    ui.spacing_mut().slider_width = ui.available_width();
                    if matches!(self.tool, Tool::Brush | Tool::Eraser) {
                        let mut thickness = self.radius * 2.0;
                        ui.horizontal(|ui| {
                            ui.label("Thickness");
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.add(
                                        egui::DragValue::new(&mut thickness)
                                            .range(1.0..=160.0)
                                            .suffix(" px"),
                                    );
                                },
                            );
                        });
                        ui.add(egui::Slider::new(&mut thickness, 1.0..=160.0).show_value(false));
                        self.radius = thickness / 2.0;
                    }
                    if self.tool == Tool::Brush || self.tool == Tool::Fill && self.transparent_paint
                    {
                        let mut amount = self.opacity * 100.0;
                        ui.horizontal(|ui| {
                            ui.label(if self.transparent_paint {
                                "Removal"
                            } else {
                                "Opacity"
                            });
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.add(
                                        egui::DragValue::new(&mut amount)
                                            .range(0.0..=100.0)
                                            .suffix("%"),
                                    );
                                },
                            );
                        });
                        ui.add(egui::Slider::new(&mut amount, 0.0..=100.0).show_value(false));
                        self.opacity = amount / 100.0;
                    }
                });
        }
        ui.add_space(16.0);
        if self.transparent_paint && self.tool != Tool::Dropper || self.tool == Tool::Eraser {
            ui.label(
                egui::RichText::new(
                    if self.scene_view { "Erase or brush Transparency to remove surface opacity. Bucket affects the whole object." } else { "Transparent pixels reveal the workspace. PNG export keeps their alpha." },
                )
                .small()
                .color(appearance::palette(ui.ctx()).muted),
            );
        } else {
            self.palette(ui);
        }
    }

    pub(super) fn object_properties(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let Some(index) = self
            .doc
            .objects
            .iter()
            .position(|o| Some(o.id) == self.selected)
        else {
            return;
        };
        let mut edited = self.doc.objects[index].clone();
        ui.separator();
        ui.label("OBJECT PROPERTIES");
        ui.label(&edited.name);
        transform_row(
            ui,
            "Position",
            &mut edited.transform.position,
            0.02,
            -20.0..=20.0,
            "",
        );
        transform_row(
            ui,
            "Rotation",
            &mut edited.transform.rotation,
            0.5,
            -360.0..=360.0,
            "°",
        );
        transform_row(
            ui,
            "Scale",
            &mut edited.transform.scale,
            0.01,
            0.05..=10.0,
            "",
        );
        ui.horizontal(|ui| {
            ui.label("Object color");
            if edited.paint.is_none() {
                ui.color_edit_button_srgba_unmultiplied(&mut edited.color);
            } else {
                ui.label("Use Bucket to recolor");
            }
        });
        if edited != self.doc.objects[index] {
            if let Err(error) = edited.validate() {
                self.error = Some(error);
            } else {
                if !self.property_active {
                    self.history.begin(&self.doc);
                    self.property_active = true;
                }
                self.doc.objects[index] = edited;
                self.changed();
            }
        }
        if self.property_active && !ctx.input(|i| i.pointer.primary_down()) {
            self.finish_edit();
        }
        if ui.button("Delete object").clicked() {
            self.delete_selected();
        }
    }

    pub(super) fn delete_selected(&mut self) {
        self.finish_edit();
        if let Some(id) = self.selected.take() {
            self.history.begin(&self.doc);
            self.doc.objects.retain(|o| o.id != id);
            self.history.commit(&self.doc);
            self.changed();
        }
    }

    pub(super) fn clear(&mut self, color: [u8; 4]) {
        self.finish_edit();
        self.history.begin(&self.doc);
        self.doc.canvas.clear(color);
        self.history.commit(&self.doc);
        self.changed();
    }
}
