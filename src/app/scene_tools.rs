//! Shape placement and surface-paint gestures in the 3D workspace.
use super::*;
use std::collections::{HashMap, HashSet};

impl Studio {
    pub(super) fn set_scene_paint_tool(&mut self, tool: Tool) {
        self.set_tool(tool);
        self.scene_view = true;
        self.tab = Tab::Objects;
        self.scene_paint_tools = true;
        self.wireframe = false;
    }
    pub(super) fn add_scene_object(&mut self, kind: Primitive) {
        self.place_scene_object(kind, [0.0; 3]);
    }
    pub(super) fn place_scene_object(&mut self, kind: Primitive, position: [f32; 3]) {
        self.finish_edit();
        self.history.begin(&self.doc);
        let id = self.doc.add_object(kind, self.color);
        if id != 0 {
            self.doc
                .objects
                .last_mut()
                .expect("new shape")
                .transform
                .position = position;
            self.selected = Some(id);
            self.scene_view = true;
            self.scene_paint_tools = false;
            self.tab = Tab::Objects;
            self.tool = Tool::Select;
            self.history.commit(&self.doc);
            self.changed();
        } else {
            self.error = Some("This project has reached its object limit.".into());
        }
    }
    pub(super) fn scene_inspector(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.horizontal(|ui| {
            if ui
                .selectable_label(!self.scene_paint_tools, "Shapes")
                .clicked()
            {
                self.finish_edit();
                self.scene_paint_tools = false;
                self.tool = Tool::Select;
            }
            if ui
                .selectable_label(self.scene_paint_tools, "Paint")
                .clicked()
            {
                self.set_scene_paint_tool(Tool::Brush);
            }
            if icons::button(ui, Icon::Select, "Select", self.tool == Tool::Select).clicked() {
                self.finish_edit();
                self.tool = Tool::Select;
                self.scene_paint_tools = false;
            }
        });
        ui.add_space(10.0);
        if self.scene_paint_tools && self.tool != Tool::Select {
            self.paint_inspector(ui);
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(
                    "Drag to paint. Right-drag to orbit. Bucket colors the whole object.",
                )
                .small()
                .color(appearance::palette(ctx).muted),
            );
        } else {
            for row in Primitive::ALL.chunks(3) {
                ui.horizontal(|ui| {
                    for &kind in row {
                        let response = shape_tile(ui, kind);
                        response.dnd_set_drag_payload(kind);
                        if response.clicked() {
                            self.add_scene_object(kind);
                        }
                    }
                });
            }
            ui.add_space(8.0);
            ui.collapsing("Shape color", |ui| self.palette(ui));
        }
        if self.selected.is_some() {
            ui.collapsing("Selected object", |ui| self.object_properties(ui, ctx));
        }
        ui.collapsing("Objects", |ui| {
            let items: Vec<_> = self
                .doc
                .objects
                .iter()
                .map(|o| (o.id, o.name.clone()))
                .collect();
            if items.is_empty() {
                ui.label("Add a shape to get started.");
            }
            for (id, name) in items {
                if ui
                    .selectable_label(self.selected == Some(id), name)
                    .clicked()
                {
                    self.finish_edit();
                    self.selected = Some(id);
                    self.render_dirty = true;
                }
            }
        });
        ui.collapsing("View", |ui| {
            if ui.checkbox(&mut self.wireframe, "Mesh edges").changed() {
                self.render_dirty = true;
            }
            if ui.button("Reset view").clicked() {
                self.camera = Camera::default();
                self.zoom = 1.0;
                self.pan = Vec2::ZERO;
                self.render_dirty = true;
            }
        });
    }
    fn prepare_surface(&mut self, id: u64) -> bool {
        let used: usize = self
            .doc
            .objects
            .iter()
            .filter_map(|o| o.paint.as_ref())
            .map(|p| p.pixels.len())
            .sum();
        let Some(object) = self.doc.objects.iter_mut().find(|o| o.id == id) else {
            return false;
        };
        if object.paint.is_none()
            && used + (scene::PAINT_SIZE * scene::PAINT_SIZE * 4) as usize > scene::MAX_PAINT_BYTES
        {
            self.error = Some("This project has reached its surface paint limit.".into());
            return false;
        }
        object.prepare_paint();
        true
    }
    pub(super) fn start_scene_paint(&mut self, point: [f32; 2]) {
        if (self.tool == Tool::Brush || self.transparent_paint && self.tool == Tool::Fill)
            && self.opacity <= 0.0
        {
            return;
        }
        if self.tool == Tool::Brush && !self.transparent_paint && self.color[3] == 0 {
            return;
        }
        if self.render_dirty || self.display_generation != Some(self.render_generation) {
            return;
        }
        let Some(render) = &self.rendered else {
            return;
        };
        let x = (point[0] / self.doc.canvas.width as f32 * render.raster.width as f32)
            .clamp(0.0, (render.raster.width - 1) as f32) as u32;
        let y = (point[1] / self.doc.canvas.height as f32 * render.raster.height as f32)
            .clamp(0.0, (render.raster.height - 1) as f32) as u32;
        let Some(id) = render.object_ids[(y * render.raster.width + x) as usize] else {
            return;
        };
        self.finish_edit();
        self.history.begin(&self.doc);
        if !self.prepare_surface(id) {
            return;
        }
        self.selected = Some(id);
        if self.tool == Tool::Fill {
            let paint = self
                .doc
                .objects
                .iter_mut()
                .find(|o| o.id == id)
                .and_then(|o| o.paint.as_mut())
                .expect("prepared");
            if self.transparent_paint {
                for pixel in paint.pixels.as_chunks_mut::<4>().0 {
                    pixel[3] =
                        (pixel[3] as f32 * (1.0 - self.opacity.clamp(0.0, 1.0))).round() as u8;
                }
            } else {
                paint.clear(self.color);
            }
            self.history.commit(&self.doc);
            self.changed();
        } else {
            self.scene_gesture = Some(point);
            self.paint_scene_segment(point, point);
        }
    }
    pub(super) fn continue_scene_paint(&mut self, point: [f32; 2]) {
        let Some(last) = self.scene_gesture else {
            return;
        };
        if last == point {
            return;
        }
        self.paint_scene_segment(last, point);
        self.scene_gesture = Some(point);
    }
    fn paint_scene_segment(&mut self, from: [f32; 2], to: [f32; 2]) {
        let Some(render) = &self.rendered else {
            return;
        };
        let scale = render.raster.width as f32 / self.doc.canvas.width as f32;
        let a = Vec2::new(from[0] * scale, from[1] * scale);
        let b = Vec2::new(to[0] * scale, to[1] * scale);
        let radius = (self.radius * scale).clamp(0.5, 160.0);
        let min = (a.min(b) - Vec2::splat(radius)).max(Vec2::ZERO);
        let max = (a.max(b) + Vec2::splat(radius)).min(Vec2::new(
            render.raster.width as f32,
            render.raster.height as f32,
        ));
        let delta = b - a;
        let length = delta.length_sq();
        let mut covered: HashMap<u64, HashSet<[u32; 2]>> = HashMap::new();
        for y in min.y as u32..max.y.ceil() as u32 {
            for x in min.x as u32..max.x.ceil() as u32 {
                let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                let t = if length > 0.0 {
                    ((p - a).dot(delta) / length).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                if (p - (a + delta * t)).length_sq() > radius * radius {
                    continue;
                }
                let i = (y * render.raster.width + x) as usize;
                if let Some(id) = render.object_ids[i] {
                    covered
                        .entry(id)
                        .or_default()
                        .insert(scene::paint_texel(render.surface_uvs[i]));
                }
            }
        }
        for (id, texels) in covered {
            if !self.prepare_surface(id) {
                continue;
            }
            let paint = self
                .doc
                .objects
                .iter_mut()
                .find(|o| o.id == id)
                .and_then(|o| o.paint.as_mut())
                .expect("prepared");
            for [x, y] in texels {
                let p = [x as f32 + 0.5, y as f32 + 0.5];
                if self.transparent_paint || self.tool == Tool::Eraser {
                    paint.erase_line_with_opacity(
                        p,
                        p,
                        0.6,
                        if self.tool == Tool::Eraser {
                            1.0
                        } else {
                            self.opacity
                        },
                    );
                } else {
                    paint.stamp(p[0], p[1], 0.6, self.color, self.opacity);
                }
            }
        }
        self.render_dirty = true;
    }
}

fn shape_tile(ui: &mut egui::Ui, kind: Primitive) -> egui::Response {
    let key = egui::Id::new(("shape-thumbnail", kind.label()));
    let texture = ui
        .ctx()
        .data(|d| d.get_temp::<egui::TextureHandle>(key))
        .unwrap_or_else(|| {
            let object = scene::SceneObject::new(1, kind, [172, 192, 232, 255]);
            let picture = scene::render(
                100,
                72,
                &[object],
                &Camera {
                    distance: 3.3,
                    ..Camera::default()
                },
                None,
                None,
                false,
            );
            let texture = ui.ctx().load_texture(
                kind.label(),
                egui::ColorImage::from_rgba_unmultiplied([100, 72], &picture.raster.pixels),
                egui::TextureOptions::LINEAR,
            );
            ui.ctx().data_mut(|d| d.insert_temp(key, texture.clone()));
            texture
        });
    let (rect, response) = ui.allocate_exact_size(Vec2::new(82.0, 78.0), Sense::click_and_drag());
    let hover =
        ui.ctx()
            .animate_bool_with_time(response.id, response.hovered(), ui.style().animation_time);
    ui.painter()
        .rect_filled(rect, 8.0, ui.visuals().faint_bg_color);
    ui.painter().rect_filled(
        rect,
        8.0,
        ui.visuals()
            .widgets
            .hovered
            .bg_fill
            .linear_multiply(hover * 0.7),
    );
    let image = Rect::from_min_size(rect.min + Vec2::new(3.0, 2.0), Vec2::new(76.0, 54.0));
    ui.painter().image(
        texture.id(),
        image,
        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
        Color32::WHITE,
    );
    ui.painter().text(
        rect.center_bottom() - Vec2::new(0.0, 11.0),
        egui::Align2::CENTER_CENTER,
        kind.label(),
        egui::FontId::proportional(12.0),
        ui.visuals().text_color(),
    );
    response.on_hover_text(format!(
        "Add {} ? click or drag into the scene",
        kind.label().to_lowercase()
    ))
}
