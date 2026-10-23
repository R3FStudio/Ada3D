//! Shared project and export menus used by the title bar.
use super::*;

impl Studio {
    pub(super) fn block_behind_menu(&mut self, ui: &mut egui::Ui) {
        self.menu_input_blocked = true;
        self.finish_edit();
        ui.ctx()
            .memory_mut(|memory| memory.set_modal_layer(ui.layer_id()));
        // Catch the dismissal click without changing the menu's measured size.
        let screen = ui.ctx().screen_rect();
        let mut backdrop = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(screen)
                .sense(Sense::click_and_drag()),
        );
        backdrop.set_min_size(screen.size());
        let _ = backdrop.response();
    }

    pub(super) fn project_menu(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        self.block_behind_menu(ui);
        if icons::menu_item(ui, Icon::New, "New project", "Ctrl+N").clicked() {
            ui.close();
            self.request(Pending::New, ctx);
        }
        if icons::menu_item(ui, Icon::Open, "Open project…", "Ctrl+O").clicked() {
            ui.close();
            self.request(Pending::Open, ctx);
        }
        if icons::menu_item(ui, Icon::Save, "Save", "Ctrl+S").clicked() {
            ui.close();
            self.save(false);
        }
        if icons::menu_item(ui, Icon::Save, "Save as…", "").clicked() {
            ui.close();
            self.save(true);
        }
        ui.separator();
        if icons::menu_item(ui, Icon::Import, "Import image…", "").clicked() {
            ui.close();
            self.import();
        }
        self.export_menu(ui);
        ui.separator();
        if icons::menu_item(ui, Icon::Home, "Welcome screen", "").clicked() {
            ui.close();
            self.finish_edit();
            self.show_welcome = true;
        }
        if icons::menu_item(ui, Icon::Settings, "Settings", "").clicked() {
            ui.close();
            self.finish_edit();
            self.show_settings = true;
        }
        if icons::menu_item(ui, Icon::Help, "Help", "").clicked() {
            ui.close();
            self.finish_edit();
            self.guide.reset();
            self.show_help = true;
        }
    }

    pub(super) fn export_menu(&mut self, ui: &mut egui::Ui) {
        if icons::menu_item(ui, Icon::Export, "Export canvas…", "").clicked() {
            ui.close();
            self.export(false);
        }
        if icons::menu_item(ui, Icon::Cube, "Export scene…", "").clicked() {
            ui.close();
            self.export(true);
        }
        if ui
            .add_enabled_ui(!self.doc.objects.is_empty(), |ui| {
                icons::menu_item(ui, Icon::Cube, "Export geometry as OBJ…", "")
            })
            .inner
            .clicked()
        {
            ui.close();
            self.export_obj();
        }
    }
}
