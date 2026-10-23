//! Update checks run only at startup or from Settings.
use super::*;
impl Studio {
    pub(super) fn check_updates(&self, ctx: &egui::Context, automatic: bool) {
        let repaint = ctx.clone();
        self.updater
            .check(adars::updater::repository(), automatic, move || {
                repaint.request_repaint()
            });
    }
    pub(super) fn install_update(&mut self, ctx: &egui::Context) {
        if jobs::active_count() != 0 {
            self.error = Some(
                "Wait for the current file operation to finish before installing the update."
                    .into(),
            );
            return;
        }
        match self.updater.install_on_exit() {
            Ok(()) => {
                self.allow_close = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            Err(error) => {
                self.updater.set_error(error);
                self.status = "Couldn’t install the update. Please try again later.".into();
            }
        }
    }
}
