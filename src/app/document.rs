//! Document lifecycle, file actions, undo and keyboard shortcuts.
use super::*;

impl Studio {
    pub(super) fn handle_close_request(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.viewport().close_requested()) && !self.allow_close {
            self.finish_edit();
            if self.dirty {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.show_welcome = false;
                self.show_help = false;
                self.pending = Some(Pending::Close);
            } else {
                if jobs::active_count() > 0 {
                    self.shutdown_requested = true;
                    self.cancel_file_tasks();
                    ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                } else {
                    self.allow_close = true;
                }
            }
        }
    }

    pub(super) fn changed(&mut self) {
        self.dirty = self.doc != self.saved;
        self.render_dirty = true;
        if !self.doc.objects.iter().any(|o| Some(o.id) == self.selected) {
            self.selected = None;
        }
    }

    pub(super) fn finish_edit(&mut self) {
        self.select_start = None;
        if self.selection_drag.take().is_some() {
            self.history.commit(&self.doc);
            self.changed();
        }
        let surface_edit = self.scene_gesture.take().is_some();
        let canvas_edit = self.gesture.take().is_some();
        if surface_edit || canvas_edit || self.property_active {
            self.history.commit(&self.doc);
            self.property_active = false;
            self.changed();
        }
    }

    pub(super) fn set_tool(&mut self, tool: Tool) {
        self.sampler_restore = None;
        self.finish_edit();
        self.selection = None;
        self.tool = tool;
        self.scene_view = tool == Tool::Select;
        self.render_dirty = true;
    }

    pub(super) fn undo(&mut self, redo: bool) {
        self.finish_edit();
        let changed = if redo {
            self.history.redo(&mut self.doc)
        } else {
            self.history.undo(&mut self.doc)
        };
        if changed {
            self.selection = None;
            self.canvas_width = self.doc.canvas.width;
            self.canvas_height = self.doc.canvas.height;
            self.changed();
            self.status = if redo {
                "Change restored."
            } else {
                "Change undone."
            }
            .into();
        }
    }

    pub(super) fn request(&mut self, action: Pending, ctx: &egui::Context) {
        self.finish_edit();
        if self.dirty {
            self.show_welcome = false;
            self.pending = Some(action);
        } else {
            self.execute(action, ctx);
        }
    }

    pub(super) fn execute(&mut self, action: Pending, ctx: &egui::Context) {
        match action {
            Pending::New => self.replace(Document::default(), None),
            Pending::Open => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Ada3D project", &[persistence::PROJECT_EXTENSION])
                    .pick_file()
                {
                    self.begin_load(path, ctx);
                }
            }
            Pending::OpenPath(path) => self.begin_load(path, ctx),
            Pending::Update => self.install_update(ctx),
            Pending::Close => {
                if jobs::active_count() > 0 {
                    self.shutdown_requested = true;
                    self.cancel_file_tasks();
                    ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                } else {
                    self.allow_close = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }
    }

    pub(super) fn replace(&mut self, doc: Document, path: Option<PathBuf>) {
        self.selection_actions_rect = Rect::NOTHING;
        self.selection = None;
        self.select_start = None;
        self.selection_drag = None;
        self.show_welcome = false;
        self.saved = doc.clone();
        self.doc = doc;
        self.path = path;
        self.history.clear();
        self.dirty = false;
        self.gesture = None;
        self.scene_gesture = None;
        self.scene_paint_tools = false;
        self.property_active = false;
        self.selected = None;
        self.camera = Camera::default();
        self.zoom = 1.0;
        self.pan = Vec2::ZERO;
        self.render_dirty = true;
        self.render_floor = self.render_generation.wrapping_add(1);
        self.render_pending = None;
        self.rendered = None;
        self.texture = None;
        self.display_generation = None;
        self.canvas_width = self.doc.canvas.width;
        self.canvas_height = self.doc.canvas.height;
        self.status = "Ready".into();
    }

    pub(super) fn save(&mut self, save_as: bool) -> bool {
        self.finish_edit();
        let path = if save_as || self.path.is_none() {
            rfd::FileDialog::new()
                .add_filter("Ada3D project", &[persistence::PROJECT_EXTENSION])
                .set_file_name(format!("Untitled.{}", persistence::PROJECT_EXTENSION))
                .save_file()
        } else {
            self.path.clone()
        };
        let Some(mut path) = path else {
            return false;
        };
        path.set_extension(persistence::PROJECT_EXTENSION);
        match persistence::save_project(&path, &self.doc) {
            Ok(()) => {
                self.remember_project(&path);
                self.path = Some(path);
                self.saved = self.doc.clone();
                self.dirty = false;
                self.status = "Project saved.".into();
                true
            }
            Err(error) => {
                self.error = Some(error);
                false
            }
        }
    }

    pub(super) fn import(&mut self) {
        self.finish_edit();
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Image", &["png", "jpg", "jpeg"])
            .pick_file()
        {
            match persistence::import_image(&path) {
                Ok(canvas) => {
                    self.show_welcome = false;
                    self.history.begin(&self.doc);
                    self.doc.canvas = canvas;
                    self.history.commit(&self.doc);
                    self.canvas_width = self.doc.canvas.width;
                    self.canvas_height = self.doc.canvas.height;
                    self.scene_view = false;
                    self.zoom = 1.0;
                    self.pan = Vec2::ZERO;
                    self.changed();
                    self.status =
                        "Image imported. Your 3D objects are still in the project.".into();
                }
                Err(error) => self.error = Some(error),
            }
        }
    }

    pub(super) fn export(&mut self, composite: bool) {
        self.finish_edit();
        if let Some(dialog) = &self.export_dialog
            && dialog.is_running()
        {
            dialog.request_focus();
            return;
        }
        self.export_dialog = Some(exporting::ExportDialog::new(
            &self.doc,
            &self.camera,
            self.path.as_deref(),
            if composite {
                adars::export::Source::Scene
            } else {
                adars::export::Source::Canvas
            },
            adars::export::Format::Png,
        ));
    }

    pub(super) fn export_obj(&mut self) {
        self.finish_edit();
        if let Some(dialog) = &self.export_dialog
            && dialog.is_running()
        {
            dialog.request_focus();
            return;
        }
        self.export_dialog = Some(exporting::ExportDialog::new(
            &self.doc,
            &self.camera,
            self.path.as_deref(),
            adars::export::Source::Scene,
            adars::export::Format::Obj,
        ));
    }
    pub(super) fn shortcuts(&mut self, ctx: &egui::Context) {
        if self.menu_input_blocked || egui::Popup::is_any_open(ctx) {
            return;
        }
        if self.sampler_restore.is_some() && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.finish_sampler();
            return;
        }
        if self.show_welcome {
            return;
        }
        if self.show_settings {
            if !ctx.wants_keyboard_input() && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.show_settings = false;
            }
            return;
        }
        if self.shutdown_requested
            || self.loading.is_some()
            || ctx.wants_keyboard_input()
            || self.pending.is_some()
            || self.error.is_some()
            || self.show_color
        {
            return;
        }
        let command = egui::Modifiers::COMMAND;
        let shift_command = egui::Modifiers {
            shift: true,
            ..command
        };
        let action = ctx.input_mut(|i| {
            if i.consume_key(shift_command, egui::Key::Z) {
                1
            } else if i.consume_key(command, egui::Key::Z) {
                2
            } else if i.consume_key(command, egui::Key::Y) {
                1
            } else if i.consume_key(shift_command, egui::Key::S) {
                4
            } else if i.consume_key(command, egui::Key::S) {
                3
            } else if i.consume_key(command, egui::Key::O) {
                5
            } else if i.consume_key(command, egui::Key::N) {
                6
            } else {
                0
            }
        });
        match action {
            1 => self.undo(true),
            2 => self.undo(false),
            3 => {
                self.save(false);
            }
            4 => {
                self.save(true);
            }
            5 => self.request(Pending::Open, ctx),
            6 => self.request(Pending::New, ctx),
            _ => {}
        }
        let tool = ctx.input(|i| {
            if i.modifiers.any() {
                return None;
            }
            if i.key_pressed(egui::Key::B) {
                Some(Tool::Brush)
            } else if i.key_pressed(egui::Key::E) {
                Some(Tool::Eraser)
            } else if i.key_pressed(egui::Key::F) {
                Some(Tool::Fill)
            } else if i.key_pressed(egui::Key::I) {
                Some(Tool::Dropper)
            } else if i.key_pressed(egui::Key::V) {
                Some(Tool::Select)
            } else {
                None
            }
        });
        if let Some(tool) = tool {
            if self.scene_view && tool != Tool::Select {
                self.set_scene_paint_tool(tool);
            } else {
                if tool == Tool::Select {
                    self.scene_paint_tools = false;
                }
                self.tab = if tool == Tool::Select {
                    Tab::Objects
                } else {
                    Tab::Paint
                };
                self.set_tool(tool);
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Delete)) {
            self.delete_selected();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if self.scene_gesture.is_some() {
                self.finish_edit();
                return;
            }
            if let Some(g) = self.gesture.take() {
                if let Some(base) = g.base {
                    self.doc.canvas = base;
                }
                self.history.commit(&self.doc);
                self.changed();
            } else {
                self.selection = None;
                self.selected = None;
                self.render_dirty = true;
            }
        }
    }
}
