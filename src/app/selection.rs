//! Marquee selection and direct transform handles.
use super::*;
mod actions;

fn point_inside_selection(point: Pos2, bounds: Rect, angle: f32) -> bool {
    let d = point - bounds.center();
    let (sin, cos) = angle.sin_cos();
    bounds.contains(bounds.center() + Vec2::new(d.x * cos + d.y * sin, -d.x * sin + d.y * cos))
}

impl Studio {
    pub(super) fn selection_overlay(
        &mut self,
        ctx: &egui::Context,
        painter: &egui::Painter,
        canvas: Rect,
        scale: f32,
        pointer: Option<Pos2>,
    ) {
        let pointer = pointer.filter(|point| {
            self.selection_drag.is_some()
                || self.select_start.is_some()
                || (self.selection.is_none() && self.selected.is_none())
                || !self.selection_actions_rect.contains(*point)
        });
        let to_screen = |p: [f32; 2]| canvas.min + Vec2::new(p[0] * scale, p[1] * scale);
        let mut bounds = None;
        if !self.scene_view {
            if let Some(s) = &self.selection {
                bounds = Some(Rect::from_center_size(
                    to_screen(s.center),
                    Vec2::new(s.size[0] * scale, s.size[1] * scale),
                ));
            }
        } else if let (Some(id), Some(render)) = (self.selected, &self.rendered) {
            let mut min = Pos2::new(f32::INFINITY, f32::INFINITY);
            let mut max = Pos2::new(f32::NEG_INFINITY, f32::NEG_INFINITY);
            for (i, object) in render.object_ids.iter().enumerate() {
                if *object == Some(id) {
                    let x = (i as u32 % render.raster.width) as f32;
                    let y = (i as u32 / render.raster.width) as f32;
                    min.x = min.x.min(x);
                    min.y = min.y.min(y);
                    max.x = max.x.max(x + 1.0);
                    max.y = max.y.max(y + 1.0);
                }
            }
            if min.x.is_finite() {
                bounds = Some(Rect::from_min_max(
                    canvas.min
                        + Vec2::new(
                            min.x / render.raster.width as f32 * canvas.width(),
                            min.y / render.raster.height as f32 * canvas.height(),
                        ),
                    canvas.min
                        + Vec2::new(
                            max.x / render.raster.width as f32 * canvas.width(),
                            max.y / render.raster.height as f32 * canvas.height(),
                        ),
                ));
            }
        }
        let accent = appearance::accent(self.preferences.accent);
        if let Some(r) = bounds {
            let mut corners = [
                r.left_top(),
                r.right_top(),
                r.right_bottom(),
                r.left_bottom(),
            ];
            let angle = self
                .selection
                .as_ref()
                .filter(|_| !self.scene_view)
                .map_or(0.0, |s| s.angle);
            for p in &mut corners {
                let d = *p - r.center();
                let (sin, cos) = angle.sin_cos();
                *p = r.center() + Vec2::new(d.x * cos - d.y * sin, d.x * sin + d.y * cos);
            }
            for i in 0..4 {
                painter.line_segment(
                    [corners[i], corners[(i + 1) % 4]],
                    Stroke::new(1.5_f32, accent),
                );
                painter.circle_filled(corners[i], 5.0, Color32::WHITE);
                painter.circle_stroke(corners[i], 5.0, Stroke::new(1.5_f32, accent));
            }
            let edges =
                std::array::from_fn::<_, 4, _>(|i| corners[i].lerp(corners[(i + 1) % 4], 0.5));
            for edge in edges {
                painter.circle_filled(edge, 5.0, Color32::WHITE);
                painter.circle_stroke(edge, 5.0, Stroke::new(1.5_f32, accent));
            }
            let rotate = edges[0] + (edges[0] - r.center()).normalized() * 28.0;
            painter.line_segment([edges[0], rotate], Stroke::new(1.0_f32, accent));
            painter.circle_filled(rotate, 9.0, accent);
            painter.text(
                rotate,
                egui::Align2::CENTER_CENTER,
                "↻",
                egui::FontId::proportional(14.0),
                Color32::WHITE,
            );
            if let Some(p) = pointer
                && ctx.input(|i| i.pointer.primary_pressed())
                && self.select_start.is_none()
            {
                let mode = if p.distance(rotate) < 14.0 {
                    Some(SelectionMode::Rotate)
                } else if let Some(index) = corners.iter().position(|c| p.distance(*c) < 11.0) {
                    Some(SelectionMode::Resize([
                        if index == 0 || index == 3 { -1.0 } else { 1.0 },
                        if index < 2 { -1.0 } else { 1.0 },
                    ]))
                } else if let Some(index) = edges.iter().position(|c| p.distance(*c) < 11.0) {
                    Some(SelectionMode::Resize(
                        [[0.0, -1.0], [1.0, 0.0], [0.0, 1.0], [-1.0, 0.0]][index],
                    ))
                } else if point_inside_selection(p, r, angle) {
                    Some(SelectionMode::Move)
                } else {
                    None
                };
                if let Some(mode) = mode {
                    self.selection_actions_rect = Rect::NOTHING;
                    self.history.begin(&self.doc);
                    self.selection_drag_point = [f32::NAN; 2];
                    self.selection_drag = Some(SelectionDrag {
                        mode,
                        start: canvas_point(
                            p,
                            canvas,
                            self.doc.canvas.width,
                            self.doc.canvas.height,
                        ),
                        selection: self.selection.clone(),
                        object: self
                            .doc
                            .objects
                            .iter()
                            .find(|o| Some(o.id) == self.selected)
                            .cloned(),
                        size: [r.width() / scale, r.height() / scale],
                    });
                }
            }
            if self.selection_drag.is_none() && self.select_start.is_none() {
                let mut action_bounds = Rect::NOTHING;
                for corner in corners {
                    action_bounds.extend_with(corner);
                }
                self.selection_actions(ctx, action_bounds);
            }
        }
        if let Some(p) = pointer {
            let pixel = canvas_point(p, canvas, self.doc.canvas.width, self.doc.canvas.height);
            if self.selection_drag.is_none()
                && ctx.input(|i| i.pointer.primary_pressed())
                && canvas.contains(p)
            {
                self.select_start = Some(pixel);
                if !self.scene_view {
                    self.selection = None;
                }
            }
            if let Some(drag) = &self.selection_drag
                && ctx.input(|i| i.pointer.primary_down())
                && pixel != self.selection_drag_point
            {
                self.selection_drag_point = pixel;
                let delta = [pixel[0] - drag.start[0], pixel[1] - drag.start[1]];
                if let Some(base) = &drag.selection
                    && !self.scene_view
                {
                    let mut next = base.clone();
                    match drag.mode {
                        SelectionMode::Move => {
                            next.center = [base.center[0] + delta[0], base.center[1] + delta[1]];
                        }
                        SelectionMode::Resize(handle) => {
                            next = base.resize_from_handle(handle, delta);
                        }
                        SelectionMode::Rotate => {
                            next.angle = base.angle
                                + (pixel[1] - base.center[1]).atan2(pixel[0] - base.center[0])
                                - (drag.start[1] - base.center[1])
                                    .atan2(drag.start[0] - base.center[0]);
                        }
                    }
                    self.doc.canvas = next.render();
                    self.selection = Some(next);
                    self.render_dirty = true;
                }
                if let Some(base) = &drag.object
                    && self.scene_view
                    && let Some(o) = self.doc.objects.iter_mut().find(|o| o.id == base.id)
                {
                    o.transform = base.transform.clone();
                    match drag.mode {
                        SelectionMode::Move => {
                            o.transform.position[0] += delta[0] * 0.01;
                            o.transform.position[1] -= delta[1] * 0.01;
                        }
                        SelectionMode::Resize(handle) => {
                            for axis in 0..2 {
                                if handle[axis] != 0.0 {
                                    let factor = (1.0
                                        + handle[axis] * delta[axis] / drag.size[axis].max(1.0))
                                    .clamp(0.01, 20.0);
                                    o.transform.scale[axis] =
                                        (base.transform.scale[axis] * factor).clamp(0.05, 20.0);
                                    let applied =
                                        o.transform.scale[axis] / base.transform.scale[axis];
                                    let shift =
                                        handle[axis] * drag.size[axis] * (applied - 1.0) * 0.5;
                                    o.transform.position[axis] +=
                                        shift * if axis == 0 { 0.01 } else { -0.01 };
                                }
                            }
                        }
                        SelectionMode::Rotate => {
                            o.transform.rotation[2] += delta[0] * 0.5;
                        }
                    }
                    self.render_dirty = true;
                }
            }
            if let Some(start) = self.select_start {
                let r = Rect::from_two_pos(to_screen(start), p);
                painter.rect_filled(r, 0.0, accent.linear_multiply(0.12));
                painter.rect_stroke(
                    r,
                    0.0,
                    Stroke::new(1.2_f32, accent),
                    egui::StrokeKind::Inside,
                );
                if !ctx.input(|i| i.pointer.primary_down()) {
                    self.select_start = None;
                    if !self.scene_view {
                        self.selection =
                            adars::selection::Selection::new(&self.doc.canvas, start, pixel);
                        ctx.request_repaint();
                    } else if let Some(render) = &self.rendered {
                        self.selected = None;
                        for (i, id) in render.object_ids.iter().enumerate() {
                            let point = canvas.min
                                + Vec2::new(
                                    (i as u32 % render.raster.width) as f32
                                        / render.raster.width as f32
                                        * canvas.width(),
                                    (i as u32 / render.raster.width) as f32
                                        / render.raster.height as f32
                                        * canvas.height(),
                                );
                            if r.expand(3.0).contains(point) && id.is_some() {
                                self.selected = *id;
                                break;
                            }
                        }
                        self.render_dirty = true;
                    }
                }
            }
        }
        if self.selection_drag.is_some() && !ctx.input(|i| i.pointer.primary_down()) {
            self.selection_drag = None;
            self.history.commit(&self.doc);
            self.changed();
        }
    }
}
