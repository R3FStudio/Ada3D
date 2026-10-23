//! Capture only this viewport's framebuffer for native export regression checks.
use super::*;
pub(super) struct Capture {
    path: PathBuf,
    started: Option<f64>,
    requested: bool,
    delay: f64,
}
impl ExportDialog {
    pub fn enable_capture(&self, path: PathBuf, format: Format, run_export: bool, delay: f64) {
        self.focus.store(false, Ordering::Relaxed);
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.options.format = format;
        state.path.set_extension(format.extension());
        state.path_text = state.path.to_string_lossy().into_owned();
        if format == Format::Ico {
            state.options.resource_file = true;
            state.options.rust_file = true;
        }
        if run_export {
            state.path = path.with_extension(format.extension());
            state.path_text = state.path.to_string_lossy().into_owned();
        }
        state.auto_export = run_export;
        state.capture = Some(Capture {
            path,
            started: None,
            requested: false,
            delay,
        });
    }
}
pub(super) fn tick(ctx: &egui::Context, state: &mut State) {
    let Some(capture) = state.capture.as_mut() else {
        return;
    };
    let now = ctx.input(|i| i.time);
    let start = *capture.started.get_or_insert(now);
    let image = ctx.input(|i| {
        i.events.iter().find_map(|event| {
            if let egui::Event::Screenshot { image, .. } = event {
                Some(image.clone())
            } else {
                None
            }
        })
    });
    if let Some(image) = image {
        let rgba: Vec<u8> = image.pixels.iter().flat_map(|p| p.to_array()).collect();
        let result = image::save_buffer_with_format(
            &capture.path,
            &rgba,
            image.width() as u32,
            image.height() as u32,
            image::ColorType::Rgba8,
            image::ImageFormat::Png,
        );
        let progress = state.job.as_ref().map(|job| job.progress());
        let record = serde_json::json!({"success":result.is_ok(),"source":"export viewport framebuffer","native_window":native_window::report_owned(TITLE),"format":state.options.format.label(),"job_active":state.job.is_some(),"export_complete":!state.outputs.is_empty(),"message":state.message,"outputs":state.outputs,"progress":progress.map(|p|serde_json::json!({"stage":p.stage,"completed":p.completed,"total":p.total,"unit":p.unit})),"width":image.width(),"height":image.height()});
        let _ = std::fs::write(capture.path.with_extension("json"), record.to_string());
        state.capture = None;
        ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Close);
        ctx.request_repaint_of(egui::ViewportId::ROOT);
    } else if now - start >= capture.delay.max(0.4) && !capture.requested {
        capture.requested = true;
        ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
    }
    ctx.request_repaint_after(std::time::Duration::from_millis(30));
}
