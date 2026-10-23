//! Render-worker scheduling and preview texture updates.
use super::*;

impl Studio {
    pub(super) fn refresh_texture(&mut self, ctx: &egui::Context) {
        while let Some(reply) = self.worker.poll() {
            if self.scene_view
                && reply.generation >= self.render_floor
                && reply.generation <= self.render_generation
                && self
                    .display_generation
                    .is_none_or(|shown| reply.generation > shown)
            {
                upload_texture(&mut self.texture, ctx, &reply.scene.raster);
                self.rendered = Some(reply.scene);
                self.display_generation = Some(reply.generation);
            }
        }
        if self.render_dirty {
            self.render_generation = self.render_generation.wrapping_add(1);
            if self.scene_view {
                let scale = if self.doc.canvas.width.min(self.doc.canvas.height) < 8 {
                    1.0
                } else {
                    (768.0 / self.doc.canvas.width.max(self.doc.canvas.height) as f32).min(1.0)
                };
                let width = (self.doc.canvas.width as f32 * scale).round().max(1.0) as u32;
                let height = (self.doc.canvas.height as f32 * scale).round().max(1.0) as u32;
                self.render_pending = Some(RenderRequest {
                    generation: self.render_generation,
                    width,
                    height,
                    objects: self.doc.objects.clone(),
                    camera: self.camera.clone(),
                    background: self.doc.canvas.clone(),
                    selected: self.selected,
                    wireframe: self.wireframe,
                });
            } else {
                self.render_pending = None;
                self.rendered = None;
                self.display_generation = None;
                self.render_floor = self.render_generation.wrapping_add(1);
                upload_texture(&mut self.texture, ctx, &self.doc.canvas);
            }
            self.render_dirty = false;
        }
        if self.scene_view && !self.worker.is_alive() && !self.worker_failed {
            self.worker_failed = true;
            self.error=Some("The 3D preview worker stopped. Save your project and reopen Ada3D to restore previews.".into());
        }
        if self.worker_failed {
            self.render_pending = None;
        }
        if let Some(request) = self.render_pending.take()
            && let Err(request) = self.worker.submit(request)
        {
            self.render_pending = Some(request);
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }
        if self.scene_view
            && !self.worker_failed
            && self.display_generation != Some(self.render_generation)
        {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }
}
