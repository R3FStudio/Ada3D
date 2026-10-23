//! Developer capture fixtures and scoped synthetic input.
use super::*;

impl Studio {
    pub fn open_export_capture(&mut self, format: adars::export::Format, run: bool) {
        let Some(path) = self.capture_path.take() else {
            return;
        };
        self.open_blank_capture();
        self.allow_close = true;
        self.export(false);
        self.export_dialog
            .as_ref()
            .expect("export window")
            .enable_capture(path, format, run, self.capture_delay);
    }
    pub fn open_recent_capture(&mut self) {
        self.open_welcome_capture();
        if let Some(parent) = self.capture_path.as_ref().and_then(|p| p.parent()) {
            for name in ["Pixel study", "First scene", "Color explorations"] {
                self.preferences
                    .remember_project(&parent.join(format!("{name}.ada3dproject")));
            }
        }
    }
    pub fn open_pixel_capture(&mut self) {
        self.open_blank_capture();
        let mut doc = Document::new(32, 32);
        for y in 0..32 {
            for x in 0..32 {
                let color = if (x + y) % 2 == 0 {
                    [235, 110, 45, 255]
                } else {
                    [40, 170, 210, 255]
                };
                let i = (y * 32 + x) * 4;
                doc.canvas.pixels[i..i + 4].copy_from_slice(&color);
            }
        }
        self.replace(doc, None);
        self.zoom = 0.5;
    }
    pub fn configure_capture_delay(&mut self, seconds: f64) {
        if seconds.is_finite() {
            self.capture_delay = seconds.clamp(0.05, 30.0);
        }
    }
    pub fn load_study(&mut self) {
        self.show_welcome = false;
        let mut doc = Document {
            title: "A study in dimensions".into(),
            ..Document::default()
        };
        doc.canvas.clear([250, 248, 237, 255]);
        doc.canvas.draw_shape(
            ShapeKind::Rectangle,
            [0.0, 495.0],
            [960.0, 640.0],
            [218, 234, 220, 255],
            1.0,
            true,
        );
        doc.canvas.draw_shape(
            ShapeKind::Ellipse,
            [130.0, 160.0],
            [400.0, 430.0],
            [247, 219, 153, 255],
            1.0,
            true,
        );
        let sphere = doc.add_object(Primitive::Sphere, [0, 133, 124, 255]);
        doc.objects[0].transform.position = [-1.25, 0.0, 0.3];
        let cube = doc.add_object(Primitive::Cube, [239, 143, 112, 255]);
        doc.objects[1].transform.position = [1.1, -0.25, 0.0];
        doc.objects[1].transform.rotation = [12.0, 28.0, -8.0];
        doc.add_object(Primitive::Cone, [235, 181, 79, 255]);
        doc.objects[2].transform.position = [0.0, 1.3, -0.65];
        doc.objects[2].transform.scale = [0.8; 3];
        self.replace(doc, None);
        self.tab = Tab::Objects;
        self.tool = Tool::Select;
        self.scene_view = true;
        self.selected = Some(if cube != 0 { cube } else { sphere });
        self.status = "Sample scene".into();
    }

    // Capture only Ada3D's framebuffer using the native egui backend.

    pub fn enable_capture(&mut self, path: PathBuf) {
        self.show_splash = false;
        self.show_welcome = false;
        self.capture_path = Some(path);
        self.capture_requested = false;
        self.capture_frames = 0;
        self.capture_started = None;
        self.capture_step = 0;
        self.capture_checks = [false; 4];
        self.capture_visual_only = false;
    }

    pub(super) fn capture_frame(&mut self, ctx: &egui::Context, frame: &eframe::Frame) {
        if self.show_splash && !self.splash_capture {
            return;
        }
        let Some(path) = self.capture_path.clone() else {
            return;
        };
        self.capture_frames = self.capture_frames.saturating_add(1);
        if self.capture_frames == 1 {
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        }
        let now = ctx.input(|i| i.time);
        let started = *self.capture_started.get_or_insert(now);
        let elapsed = now - started;
        let image = ctx.input(|input| {
            input.events.iter().find_map(|event| {
                if let egui::Event::Screenshot { image, .. } = event {
                    Some(image.clone())
                } else {
                    None
                }
            })
        });
        if let Some(image) = image {
            let rgba: Vec<u8> = image
                .pixels
                .iter()
                .flat_map(|pixel| pixel.to_array())
                .collect();
            let result = image::save_buffer_with_format(
                &path,
                &rgba,
                image.size[0] as u32,
                image.size[1] as u32,
                image::ColorType::Rgba8,
                image::ImageFormat::Png,
            );
            let window_path = path.with_file_name(format!(
                "{}.window.png",
                path.file_stem().unwrap_or_default().to_string_lossy()
            ));
            let composited = if self.capture_guide {
                native_window::capture_owned(help::TITLE, &window_path)
            } else {
                native_window::capture(frame, &window_path)
            };
            let record = serde_json::json!({"composited_capture":window_path.file_name().map(|s|s.to_string_lossy()),"composited_success":composited.is_ok(),"composited_error":composited.err(),"source":"Ada3D native OpenGL framebuffer","theme":format!("{:?}",ctx.theme()),"settings_page":self.show_settings,"guide_window":self.show_help,"context_menu_verified":self.capture_context==5,"integrated_header":self.native_header,"alpha_background":format!("{:?}",self.preferences.alpha_background),"splash":self.show_splash,"native_window":native_window::report(frame),"welcome_screen":self.show_welcome,"glass_effects":self.preferences.glass_effects,"backdrop":self.backdrop_notice,"width":image.size[0],"height":image.size[1],"frames":self.capture_frames,"elapsed_ms":elapsed*1000.0,"success":result.is_ok(),"error":result.err().map(|e|e.to_string()),"scenario":if self.capture_visual_only {"visual-only"} else {"interactive"},"native_checks":if self.capture_visual_only {serde_json::Value::Null} else {serde_json::json!({"paint_gesture":self.capture_checks[0],"single_undo_step":self.capture_checks[1],"undo_restores_document":self.capture_checks[2],"redo_restores_edit":self.capture_checks[3],"returned_to_3d":self.scene_view && self.tool==Tool::Select})}});
            let _ = std::fs::write(path.with_extension("json"), record.to_string());
            self.capture_path = None;
            self.allow_close = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        } else if !self.capture_requested
            && self.capture_step >= 10
            && self.capture_frames >= 12
            && elapsed
                >= if self.capture_tooltip {
                    self.capture_delay.max(1.3)
                } else {
                    self.capture_delay
                }
            && (self.show_settings
                || !self.scene_view
                || self.display_generation == Some(self.render_generation))
        {
            self.capture_requested = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(30));
    }

    pub fn configure_guide_capture_topic(&self, topic: usize) {
        self.guide.set_topic(topic);
    }
    pub fn open_scene_capture(&mut self, paint: bool) {
        self.open_blank_capture();
        self.doc.canvas.clear([0; 4]);
        self.camera = Camera {
            yaw: 25.0,
            pitch: 15.0,
            distance: 5.5,
        };
        let left = self.doc.add_object(Primitive::Cube, [90, 171, 230, 255]);
        self.doc.objects[0].transform.position = [-0.95, 0.0, 0.0];
        self.doc.objects[0].transform.rotation = [0.0, 12.0, 0.0];
        self.doc.add_object(Primitive::Sphere, [245, 188, 90, 255]);
        self.doc.objects[1].transform.position = [0.95, 0.15, 0.0];
        self.doc.add_object(Primitive::Torus, [175, 133, 229, 255]);
        self.doc.objects[2].transform.position = [0.0, -1.3, -0.2];
        self.doc.objects[2].transform.scale = [0.75; 3];
        self.tab = Tab::Objects;
        self.scene_view = true;
        self.tool = Tool::Select;
        self.selected = Some(left);
        if paint {
            self.set_scene_paint_tool(Tool::Brush);
            self.color = [250, 82, 105, 255];
            self.radius = 18.0;
            let render =
                scene::render(960, 640, &self.doc.objects, &self.camera, None, None, false);
            let pixels: Vec<_> = render
                .object_ids
                .iter()
                .enumerate()
                .filter(|(_, id)| **id == Some(left))
                .map(|(i, _)| [(i % 960) as f32, (i / 960) as f32])
                .collect();
            let center = [
                pixels.iter().map(|p| p[0]).sum::<f32>() / pixels.len() as f32,
                pixels.iter().map(|p| p[1]).sum::<f32>() / pixels.len() as f32,
            ];
            self.rendered = Some(render);
            self.render_dirty = false;
            self.display_generation = Some(self.render_generation);
            self.start_scene_paint([center[0] - 25.0, center[1] - 18.0]);
            self.continue_scene_paint([center[0] + 25.0, center[1] + 18.0]);
            self.finish_edit();
        }
        self.saved = self.doc.clone();
        self.history.clear();
        self.dirty = false;
        self.render_dirty = true;
    }

    pub fn open_blank_capture(&mut self) {
        self.capture_visual_only = true;
        self.replace(Document::default(), None);
        self.tab = Tab::Paint;
        self.set_tool(Tool::Brush);
        self.capture_step = 10;
    }

    pub fn open_startup_capture(&mut self) {
        self.show_splash = true;
        self.splash_capture = false;
        self.show_welcome = true;
        self.capture_visual_only = true;
        self.capture_step = 10;
    }

    pub fn open_transparency_capture(&mut self) {
        self.open_blank_capture();
        self.doc.canvas.draw_shape(
            ShapeKind::Rectangle,
            [150.0, 120.0],
            [450.0, 440.0],
            [45, 104, 238, 255],
            2.0,
            true,
        );
        self.doc.canvas.draw_shape(
            ShapeKind::Ellipse,
            [550.0, 160.0],
            [800.0, 410.0],
            [255, 151, 56, 255],
            2.0,
            true,
        );
        self.transparent_paint = true;
        self.opacity = 1.0;
        self.radius = 25.0;
        self.start_gesture([200.0, 180.0]);
        self.draw_gesture([395.0, 375.0]);
        self.finish_edit();
        self.set_tool(Tool::Fill);
        self.start_gesture([10.0, 10.0]);
        self.set_tool(Tool::Brush);
        self.preferences.alpha_background = AlphaBackground::Workspace;
        self.capture_step = 10;
    }

    pub fn open_context_capture(&mut self) {
        self.open_paint_study_capture();
        self.capture_context = 1;
    }
    pub fn open_tooltip_capture(&mut self) {
        self.open_paint_study_capture();
        self.capture_tooltip = true;
    }
    pub fn configure_settings_capture_tab(&mut self, tab: usize) {
        self.settings_tab = tab.min(2);
    }

    pub fn open_selection_capture(&mut self) {
        self.open_paint_study_capture();
        self.tool = Tool::Select;
        self.selection =
            adars::selection::Selection::new(&self.doc.canvas, [180.0, 190.0], [380.0, 510.0]);
    }

    pub fn open_color_capture(&mut self, _ctx: &egui::Context) {
        self.open_paint_study_capture();
        self.show_color = true;
    }

    pub fn open_guide_capture(&mut self) {
        self.show_splash = false;
        self.show_welcome = false;
        self.capture_visual_only = true;
        self.capture_step = 10;
        self.capture_guide = self.capture_path.is_some();
        self.guide.reset();
        if let Some(path) = self.capture_path.take() {
            self.guide.enable_capture(path);
            self.allow_close = true;
        }
        self.show_help = true;
    }

    pub fn open_paint_study_capture(&mut self) {
        self.open_blank_capture();
        self.doc.title = "Color study".into();
        self.doc.canvas.clear([246, 247, 252, 255]);
        self.doc.canvas.draw_shape(
            ShapeKind::Ellipse,
            [620.0, 65.0],
            [815.0, 260.0],
            [255, 188, 83, 255],
            2.0,
            true,
        );
        self.doc.canvas.draw_shape(
            ShapeKind::Ellipse,
            [-150.0, 360.0],
            [770.0, 930.0],
            [42, 177, 167, 255],
            2.0,
            true,
        );
        self.doc.canvas.draw_shape(
            ShapeKind::Ellipse,
            [330.0, 420.0],
            [1200.0, 850.0],
            [60, 112, 226, 255],
            2.0,
            true,
        );
        self.doc.canvas.draw_shape(
            ShapeKind::Rectangle,
            [185.0, 205.0],
            [370.0, 500.0],
            [145, 103, 228, 255],
            2.0,
            true,
        );
        self.doc.canvas.draw_shape(
            ShapeKind::Ellipse,
            [440.0, 250.0],
            [600.0, 410.0],
            [252, 137, 120, 255],
            2.0,
            true,
        );
        self.doc
            .canvas
            .line([185.0, 165.0], [405.0, 165.0], 8.0, [45, 60, 109, 255], 1.0);
        self.saved = self.doc.clone();
        self.dirty = false;
        self.render_dirty = true;
    }

    pub fn open_splash_capture(&mut self) {
        self.capture_visual_only = true;
        self.capture_step = 10;
        self.show_splash = true;
        self.splash_capture = self.capture_path.is_some();
    }

    pub fn open_menu_capture(&mut self) {
        self.open_blank_capture();
        self.capture_menu = true;
    }

    pub fn open_welcome_capture(&mut self) {
        self.open_blank_capture();
        self.show_welcome = true;
    }

    pub fn open_settings_capture(&mut self) {
        self.show_welcome = false;
        self.capture_visual_only = true;
        self.show_settings = true;
        self.capture_step = 10;
    }
    pub(super) fn capture_input(&mut self, input: &mut egui::RawInput) {
        if self.capture_tooltip && self.capture_path.is_some() {
            input
                .events
                .retain(|e| matches!(e, egui::Event::Screenshot { .. }));
            input.focused = true;
            if self.open_rect.is_finite() && self.capture_frames < 3 {
                input
                    .events
                    .push(egui::Event::PointerMoved(self.open_rect.center()));
            }
            return;
        }
        if self.capture_context > 0 && self.capture_path.is_some() {
            input
                .events
                .retain(|e| matches!(e, egui::Event::Screenshot { .. }));
            input.focused = true;
            if self.canvas_rect.is_finite() && self.capture_context < 4 {
                let pos = self.canvas_rect.center();
                input.events.push(egui::Event::PointerMoved(pos));
                if self.capture_context > 1 {
                    input.events.push(egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Secondary,
                        pressed: self.capture_context == 2,
                        modifiers: egui::Modifiers::NONE,
                    });
                }
                self.capture_context += 1;
            }
            return;
        }
        if self.capture_path.is_none() || self.capture_step >= 10 {
            return;
        }
        // Verification owns its synthetic input; native focus/pointer notifications
        // arriving between steps must not cancel the test gesture.
        input.events.clear();
        input.focused = true;
        input.modifiers = egui::Modifiers::NONE;
        if self.capture_step == 0 && self.display_generation != Some(self.render_generation) {
            return;
        }
        if self.capture_step == 9 {
            if self.scene_view && self.display_generation == Some(self.render_generation) {
                self.capture_step = 10;
            }
            return;
        }
        let pointer = |input: &mut egui::RawInput, pos: Pos2, pressed: Option<bool>| {
            input.events.push(egui::Event::PointerMoved(pos));
            if let Some(pressed) = pressed {
                input.events.push(egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                });
            }
        };
        match self.capture_step {
            0 => pointer(input, self.paint_tab_rect.center(), Some(true)),
            1 => pointer(input, self.paint_tab_rect.center(), Some(false)),
            2 => pointer(
                input,
                self.canvas_rect.min + self.canvas_rect.size() * 0.7,
                Some(true),
            ),
            3 => pointer(
                input,
                self.canvas_rect.min + self.canvas_rect.size() * 0.8,
                None,
            ),
            4 => pointer(
                input,
                self.canvas_rect.min + self.canvas_rect.size() * 0.8,
                Some(false),
            ),
            5 | 6 => {
                if self.capture_step == 5 {
                    self.capture_checks[0] = self.doc != self.saved;
                    self.capture_checks[1] = self.history.state_count() == 1;
                } else {
                    self.capture_checks[2] = self.doc == self.saved;
                }
                let modifiers = egui::Modifiers {
                    ctrl: true,
                    command: true,
                    shift: self.capture_step == 6,
                    ..egui::Modifiers::NONE
                };
                input.modifiers = modifiers;
                input.events.push(egui::Event::Key {
                    key: egui::Key::Z,
                    physical_key: Some(egui::Key::Z),
                    pressed: true,
                    repeat: false,
                    modifiers,
                });
                input.events.push(egui::Event::Key {
                    key: egui::Key::Z,
                    physical_key: Some(egui::Key::Z),
                    pressed: false,
                    repeat: false,
                    modifiers,
                });
            }
            7 => {
                self.capture_checks[3] = self.doc != self.saved;
                pointer(input, self.objects_tab_rect.center(), Some(true));
            }
            8 => pointer(input, self.objects_tab_rect.center(), Some(false)),
            _ => {}
        }
        self.capture_step += 1;
    }
}
