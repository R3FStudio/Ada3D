//! Canvas presentation and paint gestures.
use super::*;

impl Studio {
    pub(super) fn canvas(&mut self, ctx: &egui::Context) {
        self.refresh_texture(ctx);
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(Color32::TRANSPARENT)
                    .inner_margin(0),
            )
            .show(ctx, |ui| {
                let (area, response) =
                    ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
                let mut popup = egui::Popup::context_menu(&response);
                if popup.is_open()
                    && ctx.input(|i| i.pointer.secondary_clicked())
                    && let Some(point) = ctx.pointer_hover_pos().filter(|p| area.contains(*p))
                    && popup
                        .get_popup_rect()
                        .is_some_and(|rect| !rect.contains(point))
                {
                    // The modal backdrop owns this click, so retarget the menu directly.
                    ctx.data_mut(|data| {
                        data.remove::<f64>(response.id.with("menu-opened"));
                    });
                    popup = popup
                        .open_memory(Some(egui::SetOpenCommand::Bool(true)))
                        .close_behavior(egui::PopupCloseBehavior::IgnoreClicks);
                }
                let menu = popup.show(|ui| {
                    self.block_behind_menu(ui);
                    ui.spacing_mut().interact_size.y = 28.0;
                    let now = ctx.input(|i| i.time);
                    let id = response.id.with("menu-opened");
                    let start = ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(id, || now));
                    let fade = if self.preferences.reduce_motion {
                        1.0
                    } else {
                        ((now - start) / 0.16).clamp(0.0, 1.0) as f32
                    };
                    ui.set_opacity(fade);
                    if fade < 1.0 {
                        ctx.request_repaint();
                    }
                    ui.label(if self.scene_view { "Scene" } else { "Canvas" });
                    if ui
                        .add_enabled_ui(self.history.can_undo(), |ui| {
                            icons::menu_item(ui, Icon::Undo, "Undo", "Ctrl+Z")
                        })
                        .inner
                        .clicked()
                    {
                        self.undo(false);
                        ui.close();
                    }
                    if ui
                        .add_enabled_ui(self.history.can_redo(), |ui| {
                            icons::menu_item(ui, Icon::Redo, "Redo", "Ctrl+Shift+Z")
                        })
                        .inner
                        .clicked()
                    {
                        self.undo(true);
                        ui.close();
                    }
                    ui.separator();
                    if self.tool == Tool::Select
                        && (self.selection.is_some() || self.selected.is_some())
                    {
                        if icons::menu_item(ui, Icon::Undo, "Rotate selection left", "").clicked() {
                            self.rotate_selection(-std::f32::consts::FRAC_PI_2);
                            ui.close();
                        }
                        if icons::menu_item(ui, Icon::Redo, "Rotate selection right", "").clicked()
                        {
                            self.rotate_selection(std::f32::consts::FRAC_PI_2);
                            ui.close();
                        }
                        if icons::menu_item(ui, Icon::Back, "Deselect", "Esc").clicked() {
                            self.deselect();
                            ui.close();
                        }
                        ui.separator();
                    }
                    if icons::menu_item(ui, Icon::Select, "Select and transform", "").clicked() {
                        self.finish_edit();
                        self.tool = Tool::Select;
                        if self.scene_view {
                            self.scene_paint_tools = false;
                        }
                        ui.close();
                    }
                    if icons::menu_item(
                        ui,
                        Icon::Dropper,
                        if self.scene_view {
                            "Pick surface color"
                        } else {
                            "Pick canvas color"
                        },
                        "",
                    )
                    .clicked()
                    {
                        self.finish_edit();
                        self.tool = Tool::Dropper;
                        if self.scene_view {
                            self.scene_paint_tools = true;
                            self.tab = Tab::Objects;
                        }
                        ui.close();
                    }
                    if icons::menu_item(ui, Icon::Dropper, "Choose / sample color…", "").clicked()
                    {
                        self.show_color = true;
                        ui.close();
                    }
                    if self.scene_view
                        && ui
                            .add_enabled_ui(self.selected.is_some(), |ui| {
                                icons::menu_item(ui, Icon::Delete, "Delete object", "Del")
                            })
                            .inner
                            .clicked()
                    {
                        self.delete_selected();
                        ui.close();
                    }
                    if icons::menu_item(ui, Icon::Fit, "Fit view", "").clicked() {
                        self.zoom = 1.0;
                        self.pan = Vec2::ZERO;
                        ui.close();
                    }
                    ui.separator();
                    if icons::menu_item(ui, Icon::Export, "Export artwork…", "").clicked() {
                        self.export(self.scene_view);
                        ui.close();
                    }
                });
                if menu.is_some() && self.capture_context > 0 {
                    self.capture_context = 5;
                }
                if menu.is_none() {
                    ctx.data_mut(|d| {
                        d.remove::<f64>(response.id.with("menu-opened"));
                    });
                }
                let painter = ui.painter_at(area);
                let colors = appearance::palette(ctx);
                appearance::gradient(
                    &painter,
                    area,
                    colors.workspace_top,
                    colors.workspace_bottom,
                    false,
                );
                if self.preferences.workspace_grid {
                    for y in 0..(area.height() / 24.0) as usize {
                        for x in 0..(area.width() / 24.0) as usize {
                            painter.circle_filled(
                                area.min
                                    + Vec2::new(x as f32 * 24.0 + 12.0, y as f32 * 24.0 + 12.0),
                                0.8,
                                colors.grid,
                            );
                        }
                    }
                }

                let base = ((area.width() - 80.0) / self.doc.canvas.width as f32)
                    .min((area.height() - 100.0) / self.doc.canvas.height as f32)
                    .max(0.01);
                let motion = if self.preferences.reduce_motion {
                    0.0
                } else {
                    0.18
                };
                let display_zoom =
                    ctx.animate_value_with_time(egui::Id::new("canvas-zoom"), self.zoom, motion);
                let scale = base * display_zoom;
                let size = Vec2::new(
                    self.doc.canvas.width as f32 * scale,
                    self.doc.canvas.height as f32 * scale,
                );
                let rect = Rect::from_center_size(
                    area.center()
                        + Vec2::new(
                            ctx.animate_value_with_time(
                                egui::Id::new("pan-x"),
                                self.pan.x,
                                if self.preferences.reduce_motion {
                                    0.0
                                } else {
                                    0.16
                                },
                            ),
                            ctx.animate_value_with_time(
                                egui::Id::new("pan-y"),
                                self.pan.y,
                                if self.preferences.reduce_motion {
                                    0.0
                                } else {
                                    0.16
                                },
                            ),
                        ),
                    size,
                );
                self.canvas_rect = rect;
                painter.text(
                    Pos2::new(area.left() + 22.0, area.top() + 20.0),
                    egui::Align2::LEFT_TOP,
                    if self.scene_view {
                        "3D view"
                    } else {
                        "2D canvas"
                    },
                    egui::FontId::proportional(10.0),
                    colors.muted,
                );
                if self.preferences.alpha_background == AlphaBackground::Checkerboard {
                    painter.rect_filled(rect, 0.0, Color32::WHITE);
                    let clip = rect.intersect(area);
                    let tile = 16.0;
                    for y in 0..(clip.height() / tile).ceil() as i32 {
                        for x in 0..(clip.width() / tile).ceil() as i32 {
                            if (x + y) % 2 == 0 {
                                let r = Rect::from_min_size(
                                    clip.min + Vec2::new(x as f32 * tile, y as f32 * tile),
                                    Vec2::splat(tile),
                                )
                                .intersect(clip);
                                painter.rect_filled(r, 0.0, Color32::from_rgb(225, 229, 235));
                            }
                        }
                    }
                }
                if let Some(texture) = &self.texture {
                    painter.image(
                        texture.id(),
                        rect,
                        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                        Color32::WHITE,
                    );
                }
                painter.rect_stroke(
                    rect,
                    0.0,
                    Stroke::new(1.0_f32, colors.border),
                    egui::StrokeKind::Outside,
                );
                if self.scene_view && self.display_generation != Some(self.render_generation) {
                    painter.text(
                        Pos2::new(area.center().x, area.bottom() - 28.0),
                        egui::Align2::CENTER_CENTER,
                        if self.worker_failed {
                            "3D preview unavailable"
                        } else {
                            "Updating 3D preview…"
                        },
                        egui::FontId::proportional(12.0),
                        appearance::accent(self.preferences.accent),
                    );
                }
                if self.tool == Tool::Dropper && response.hovered() {
                    ctx.set_cursor_icon(egui::CursorIcon::Crosshair);
                }
                let pointer = ctx.input(|i| i.pointer.interact_pos());
                let inside = pointer.is_some_and(|p| rect.contains(p) && area.contains(p));
                let blocked = self.loading.is_some()
                    || self.pending.is_some()
                    || self.error.is_some()
                    || self.show_color
                    || self.menu_input_blocked
                    || egui::Popup::is_any_open(ctx);
                if response.hovered()
                    && !blocked
                    && self.gesture.is_none()
                    && self.scene_gesture.is_none()
                {
                    let scroll = ctx.input(|i| i.smooth_scroll_delta.y);
                    if scroll.abs() > 0.01 {
                        self.zoom = (self.zoom * (scroll * 0.002).exp()).clamp(0.15, 8.0);
                    }
                    if ctx.input(|i| i.pointer.button_down(egui::PointerButton::Middle)) {
                        self.pan += ctx.input(|i| i.pointer.delta());
                    }
                    if self.scene_view && ctx.input(|i| i.pointer.secondary_down()) {
                        let delta = ctx.input(|i| i.pointer.delta());
                        if delta != Vec2::ZERO {
                            self.camera.yaw += delta.x * 0.45;
                            self.camera.pitch =
                                (self.camera.pitch + delta.y * 0.45).clamp(-85.0, 85.0);
                            self.render_dirty = true;
                        }
                    }
                }
                if !blocked && let Some(p) = pointer {
                    let pixel =
                        canvas_point(p, rect, self.doc.canvas.width, self.doc.canvas.height);
                    if inside
                        && self.tool != Tool::Select
                        && response.hovered()
                        && ctx.input(|i| i.pointer.button_pressed(egui::PointerButton::Primary))
                    {
                        if self.tool == Tool::Dropper {
                            self.sample_color(pixel);
                        } else if self.scene_view
                            && self.scene_paint_tools
                            && matches!(self.tool, Tool::Brush | Tool::Eraser | Tool::Fill)
                        {
                            self.start_scene_paint(pixel);
                        } else if self.scene_view
                            && !self.render_dirty
                            && self.display_generation == Some(self.render_generation)
                        {
                            self.finish_edit();
                            self.selected = self.rendered.as_ref().and_then(|r| {
                                let index = pick_index(p, rect, r.raster.width, r.raster.height);
                                r.object_ids.get(index).copied().flatten()
                            });
                            self.render_dirty = true;
                        } else if !self.scene_view {
                            self.start_gesture(pixel);
                        }
                    }
                    if ctx.input(|i| i.pointer.primary_down()) && self.scene_gesture.is_some() {
                        self.continue_scene_paint(pixel);
                    }
                    if ctx.input(|i| i.pointer.primary_down()) && self.gesture.is_some() {
                        self.draw_gesture(pixel);
                    }
                    if inside
                        && (!self.scene_view || self.scene_paint_tools)
                        && self.preferences.brush_cursor
                        && matches!(self.tool, Tool::Brush | Tool::Eraser)
                    {
                        painter.circle_stroke(
                            p,
                            self.radius * scale,
                            Stroke::new(1.0_f32, Color32::from_rgb(38, 89, 82)),
                        );
                    }
                }
                if !blocked
                    && self.scene_view
                    && inside
                    && let Some(kind) = response.dnd_release_payload::<Primitive>()
                    && let Some(point) = pointer
                {
                    let normalized = [
                        (point.x - rect.left()) / rect.width(),
                        (point.y - rect.top()) / rect.height(),
                    ];
                    let position = scene::placement(
                        &self.camera,
                        normalized,
                        self.doc.canvas.width,
                        self.doc.canvas.height,
                    );
                    self.place_scene_object(*kind, position);
                }
                if !blocked
                    && !self.scene_view
                    && inside
                    && let Some(kind) = response.dnd_release_payload::<ShapeKind>()
                    && let Some(point) = pointer
                {
                    let center =
                        canvas_point(point, rect, self.doc.canvas.width, self.doc.canvas.height);
                    self.place_canvas_shape(*kind, center);
                }
                if self.tool == Tool::Select {
                    let selection_pointer = pointer.filter(|_| {
                        !blocked
                            && (response.hovered()
                                || self.selection_drag.is_some()
                                || self.select_start.is_some())
                    });
                    self.selection_overlay(ctx, &painter, rect, scale, selection_pointer);
                }
                if (self.gesture.is_some() || self.scene_gesture.is_some())
                    && !ctx.input(|i| i.pointer.primary_down())
                {
                    self.finish_edit();
                }
            });
        if self.render_dirty {
            ctx.request_repaint();
        }
    }

    pub(super) fn start_gesture(&mut self, point: [f32; 2]) {
        if self.tool == Tool::Dropper {
            self.sample_color(point);
            return;
        }
        if self.tool == Tool::Select {
            return;
        }
        self.history.begin(&self.doc);
        if self.tool == Tool::Fill {
            let x = (point[0] as u32).min(self.doc.canvas.width - 1);
            let y = (point[1] as u32).min(self.doc.canvas.height - 1);
            if self.transparent_paint {
                self.doc.canvas.fill_transparency(x, y, self.opacity);
            } else {
                self.doc.canvas.fill(x, y, self.color);
            }
            self.history.commit(&self.doc);
            self.changed();
            return;
        }
        let base = Some(self.doc.canvas.clone());
        self.gesture = Some(Gesture {
            start: point,
            last: point,
            base,
            tool: self.tool,
            transparent: self.transparent_paint,
        });
        match self.tool {
            Tool::Brush if self.transparent_paint => {
                self.doc
                    .canvas
                    .erase_line_with_opacity(point, point, self.radius, self.opacity)
            }
            Tool::Brush => {
                self.doc
                    .canvas
                    .stamp(point[0], point[1], self.radius, self.color, self.opacity)
            }
            Tool::Eraser => self.doc.canvas.erase_stamp(point[0], point[1], self.radius),
            _ => {}
        }
        self.render_dirty = true;
    }

    pub(super) fn draw_gesture(&mut self, point: [f32; 2]) {
        let Some(g) = self.gesture.as_mut() else {
            return;
        };
        // Clamp captured drags so leaving the window cannot create unbounded work.
        let point = [
            point[0].clamp(-80.0, self.doc.canvas.width as f32 + 80.0),
            point[1].clamp(-80.0, self.doc.canvas.height as f32 + 80.0),
        ];
        if point == g.last {
            return;
        }
        match g.tool {
            Tool::Brush if g.transparent => {
                self.doc
                    .canvas
                    .erase_line_with_opacity(g.last, point, self.radius, self.opacity)
            }
            Tool::Brush => {
                self.doc
                    .canvas
                    .line(g.last, point, self.radius, self.color, self.opacity)
            }
            Tool::Eraser => self.doc.canvas.erase_line(g.last, point, self.radius),
            Tool::Shape(kind) => {
                if let Some(base) = &g.base {
                    self.doc.canvas = base.clone();
                }
                self.doc.canvas.draw_shape(
                    kind,
                    g.start,
                    point,
                    self.color,
                    self.radius * 2.0,
                    self.filled,
                );
            }
            _ => {}
        }
        g.last = point;
        self.render_dirty = true;
    }
}
