//! Actions that stay beside a selection without blocking its handles.
use super::*;

impl Studio {
    pub(super) fn selection_actions(&mut self, ctx: &egui::Context, bounds: Rect) {
        let screen = ctx.screen_rect().shrink(8.0);
        let y = if bounds.bottom() + 52.0 < screen.bottom() {
            bounds.bottom() + 12.0
        } else {
            bounds.top() - 48.0
        };
        let position = Pos2::new(
            bounds
                .left()
                .clamp(screen.left(), (screen.right() - 320.0).max(screen.left())),
            y.clamp(screen.top(), (screen.bottom() - 40.0).max(screen.top())),
        );
        let bar = egui::Area::new(egui::Id::new("selection-actions"))
            .order(egui::Order::Foreground)
            .fixed_pos(position)
            .default_size(Vec2::new(300.0, 40.0))
            .show(ctx, |ui| {
                if self.menu_input_blocked
                    || egui::Popup::is_any_open(ctx)
                    || self.show_color
                    || self.pending.is_some()
                    || self.error.is_some()
                    || self.loading.is_some()
                {
                    ui.disable();
                }
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if icons::action(ui, Icon::Undo, "Rotate left").clicked() {
                            self.rotate_selection(-std::f32::consts::FRAC_PI_2);
                        }
                        if icons::action(ui, Icon::Redo, "Rotate right").clicked() {
                            self.rotate_selection(std::f32::consts::FRAC_PI_2);
                        }
                        if icons::action(ui, Icon::Back, "Done").clicked() {
                            self.deselect();
                        }
                    });
                });
            });
        self.selection_actions_rect = bar.response.rect;
    }

    pub(in crate::app) fn rotate_selection(&mut self, angle: f32) {
        self.finish_edit();
        self.history.begin(&self.doc);
        if !self.scene_view {
            if let Some(selection) = &mut self.selection {
                selection.angle += angle;
                self.doc.canvas = selection.render();
            }
        } else if let Some(object) = self
            .doc
            .objects
            .iter_mut()
            .find(|o| Some(o.id) == self.selected)
        {
            object.transform.rotation[2] += angle.to_degrees();
        }
        self.history.commit(&self.doc);
        self.changed();
    }

    pub(in crate::app) fn deselect(&mut self) {
        self.finish_edit();
        self.selection_actions_rect = Rect::NOTHING;
        self.selection = None;
        self.selected = None;
        self.render_dirty = true;
    }
}
