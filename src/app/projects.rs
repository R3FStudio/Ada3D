//! Recent projects and nonblocking, measured project loading.
use super::*;

impl Studio {
    pub(super) fn cancel_file_tasks(&self) {
        if let Some((_, job)) = &self.loading {
            job.cancel();
        }
        if let Some(dialog) = &self.export_dialog {
            dialog.cancel();
        }
    }
    pub(super) fn remember_project(&mut self, path: &std::path::Path) {
        self.preferences.remember_project(path);
        if let Some(file) = &self.preferences_path
            && let Err(error) = settings::save(file, &self.preferences)
        {
            self.preferences_notice = error;
        }
    }
    pub(super) fn begin_load(&mut self, path: PathBuf, ctx: &egui::Context) {
        if self.loading.is_some() {
            return;
        }
        self.show_welcome = false;
        let read_path = path.clone();
        match jobs::Job::start(ctx, egui::ViewportId::ROOT, move |report| {
            persistence::load_project_with_progress(&read_path, report)
        }) {
            Ok(job) => self.loading = Some((path, job)),
            Err(e) => self.error = Some(e),
        }
    }
    pub(super) fn poll_load(&mut self) {
        let Some(result) = self.loading.as_ref().and_then(|(_, job)| job.poll()) else {
            return;
        };
        let (path, _) = self.loading.take().expect("active load");
        match result {
            Ok(doc) => {
                self.remember_project(&path);
                self.replace(doc, Some(path));
                self.status = "Project loaded.".into();
            }
            Err(error) if error == "Cancelled" => self.status = "Loading cancelled.".into(),
            Err(error) => self.error = Some(error),
        }
    }
    pub(super) fn loading_dialog(&mut self, ctx: &egui::Context) {
        if let Some((path, job)) = &self.loading {
            egui::Modal::new(egui::Id::new("project-loading")).show(ctx, |ui| {
                ui.set_width(370.0);
                ui.heading("Opening project");
                ui.label(path.file_name().unwrap_or_default().to_string_lossy());
                ui.add_space(12.0);
                jobs::progress_ui(ui, &job.progress());
                ui.add_space(12.0);
                if ui.button("Cancel").clicked() {
                    job.cancel();
                }
                ui.label(
                    egui::RichText::new("Progress counts work in the current loading stage.")
                        .small()
                        .color(appearance::palette(ctx).muted),
                );
            });
        }
    }
}
