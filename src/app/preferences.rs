//! Preference changes and persistence.
use super::*;

impl Studio {
    pub fn configure_theme(&mut self, ctx: &egui::Context, theme: ThemeMode) {
        self.preferences.theme = theme;
        appearance::install(ctx, &self.preferences);
    }

    pub fn configure_backdrop(&mut self, ctx: &egui::Context, enabled: bool, backdrop: Backdrop) {
        self.preferences.glass_effects = enabled;
        self.preferences.backdrop = backdrop;
        appearance::install(ctx, &self.preferences);
    }

    pub(super) fn persist_preferences(&mut self, ctx: &egui::Context) {
        appearance::install(ctx, &self.preferences);
        self.preferences_notice = match &self.preferences_path {
            Some(path) => settings::save(path, &self.preferences)
                .map(|()| "Preferences saved".into())
                .unwrap_or_else(|e| e),
            None => "Changes applied".into(),
        };
    }

    pub(super) fn select_brush_preset(&mut self, preset: u8) {
        self.set_tool(Tool::Brush);
        self.brush_preset = preset;
        (self.radius, self.opacity) = match preset {
            1 => (1.5, 1.0),
            2 => (14.0, 0.4),
            _ => (8.0, 1.0),
        };
    }

    pub(super) fn settings_page(&mut self, ctx: &egui::Context) {
        let before = self.preferences.clone();
        let update = self.updater.snapshot();
        let mut action = crate::settings_ui::UpdateAction::None;
        if crate::settings_ui::show(
            ctx,
            &mut self.preferences,
            &mut self.settings_tab,
            &self.preferences_notice,
            &self.backdrop_notice,
            self.show_settings,
            &update,
            &mut action,
        ) {
            self.show_settings = false;
        }
        match action {
            crate::settings_ui::UpdateAction::Check => self.check_updates(ctx, false),
            crate::settings_ui::UpdateAction::Install => self.request(Pending::Update, ctx),
            crate::settings_ui::UpdateAction::None => {}
        }
        if self.preferences != before {
            appearance::install(ctx, &self.preferences);
            self.preferences_notice = if let Some(path) = &self.preferences_path {
                match settings::save(path, &self.preferences) {
                    Ok(()) => "Preferences saved".into(),
                    Err(e) => e,
                }
            } else {
                "Changes applied".into()
            };
            ctx.request_repaint();
        }
    }
}
