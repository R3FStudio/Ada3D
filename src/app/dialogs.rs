//! Color sampling and document-confirmation dialogs.
use super::*;

impl Studio {
    pub(super) fn begin_sampler(&mut self) {
        self.finish_edit();
        self.sampler_restore = Some((self.tool, self.scene_view));
        self.tool = Tool::Dropper;
        self.show_color = false;
        self.status = if self.scene_view {
            "Click an object to sample a surface color. Escape cancels."
        } else {
            "Click the canvas to sample a color. Escape cancels."
        }
        .into();
    }

    pub(super) fn finish_sampler(&mut self) {
        if let Some((tool, scene)) = self.sampler_restore.take() {
            self.tool = tool;
            self.scene_view = scene;
            self.show_color = true;
        }
    }

    pub(super) fn sample_color(&mut self, point: [f32; 2]) {
        if self.scene_view {
            if self.render_dirty || self.display_generation != Some(self.render_generation) {
                return;
            }
            if let Some(render) = &self.rendered {
                let x = (point[0] / self.doc.canvas.width as f32 * render.raster.width as f32)
                    .clamp(0.0, (render.raster.width - 1) as f32) as u32;
                let y = (point[1] / self.doc.canvas.height as f32 * render.raster.height as f32)
                    .clamp(0.0, (render.raster.height - 1) as f32) as u32;
                let index = (y * render.raster.width + x) as usize;
                self.color = render.object_ids[index]
                    .and_then(|id| self.doc.objects.iter().find(|o| o.id == id))
                    .map_or_else(
                        || render.raster.pixel(x, y),
                        |o| o.surface_color(render.surface_uvs[index]),
                    );
            }
        } else {
            self.color = self.doc.canvas.pixel(
                (point[0] as u32).min(self.doc.canvas.width - 1),
                (point[1] as u32).min(self.doc.canvas.height - 1),
            );
        }
        self.status = "Color sampled.".into();
        self.finish_sampler();
    }

    pub(super) fn color_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_color {
            return;
        }
        let mut open = true;
        egui::Window::new("Choose a color")
            .open(&mut open)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .collapsible(false)
            .resizable(false)
            .auto_sized()
            .min_width(280.0)
            .max_width(300.0)
            .show(ctx, |ui| {
                ui.spacing_mut().slider_width = 280.0;
                ui.spacing_mut().interact_size = Vec2::new(28.0, 26.0);
                let mut color = egui::ecolor::Hsva::from_srgba_unmultiplied(self.color);
                if egui::color_picker::color_picker_hsva_2d(
                    ui,
                    &mut color,
                    egui::color_picker::Alpha::OnlyBlend,
                ) {
                    self.color = color.to_srgba_unmultiplied();
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if icons::action(ui, Icon::Dropper, "")
                        .on_hover_text("Sample a color from the canvas")
                        .clicked()
                    {
                        self.begin_sampler();
                    }
                    ui.label(format!(
                        "#{:02X}{:02X}{:02X} · alpha {}",
                        self.color[0], self.color[1], self.color[2], self.color[3]
                    ));
                    if ui.button("Done").clicked() {
                        self.show_color = false;
                    }
                });
            });
        self.show_color &= open;
    }

    pub(super) fn dialogs(&mut self, ctx: &egui::Context) {
        self.color_dialog(ctx);
        if let Some(action) = self.pending.clone() {
            let mut decision = 0;
            egui::Window::new("Keep your work?")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.label("Your project has unsaved changes.");
                    ui.horizontal(|ui| {
                        if ui.button("Save and continue").clicked() {
                            decision = 1;
                        }
                        if ui.button("Discard changes").clicked() {
                            decision = 2;
                        }
                        if ui.button("Cancel").clicked() {
                            decision = 3;
                        }
                    });
                });
            match decision {
                1 => {
                    if self.save(false) {
                        self.pending = None;
                        self.execute(action, ctx);
                    }
                }
                2 => {
                    self.pending = None;
                    self.execute(action, ctx);
                }
                3 => self.pending = None,
                _ => {}
            }
        }
        if let Some(error) = self.error.clone() {
            egui::Window::new("Couldn't finish that action")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.set_max_width(430.0);
                    ui.label(error);
                    if ui.button("Got it").clicked() {
                        self.error = None;
                    }
                });
        }
    }
}
