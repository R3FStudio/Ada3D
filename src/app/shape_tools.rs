//! 2D shape thumbnails and drag placement; brush tools remain separate.
use super::*;
impl Studio {
    pub(super) fn canvas_shape_palette(&mut self, ui: &mut egui::Ui) {
        for row in ShapeKind::ALL.chunks(3) {
            ui.horizontal(|ui| {
                for &kind in row {
                    let selected = self.tool == Tool::Shape(kind);
                    let response = tile(ui, kind, selected);
                    response.dnd_set_drag_payload(kind);
                    if response.clicked() {
                        self.set_tool(Tool::Shape(kind));
                    }
                }
            });
        }
    }
    pub(super) fn place_canvas_shape(&mut self, kind: ShapeKind, center: [f32; 2]) {
        self.set_tool(Tool::Shape(kind));
        let size =
            (self.doc.canvas.width.min(self.doc.canvas.height) as f32 * 0.24).clamp(12.0, 180.0);
        let from = [center[0] - size / 2.0, center[1] - size / 2.0];
        let to = [center[0] + size / 2.0, center[1] + size / 2.0];
        self.start_gesture(from);
        self.draw_gesture(to);
        self.finish_edit();
    }
    pub fn open_shapes_capture(&mut self) {
        self.open_blank_capture();
        self.tab = Tab::Shapes;
        self.filled = true;
        for (kind, center, color) in [
            (
                ShapeKind::RoundedRectangle,
                [210.0, 190.0],
                [45, 104, 238, 255],
            ),
            (ShapeKind::Star, [480.0, 190.0], [253, 191, 45, 255]),
            (ShapeKind::Heart, [750.0, 190.0], [236, 54, 73, 255]),
            (ShapeKind::Hexagon, [210.0, 440.0], [0, 143, 144, 255]),
            (ShapeKind::Arrow, [480.0, 440.0], [164, 61, 199, 255]),
            (ShapeKind::Triangle, [750.0, 440.0], [255, 137, 39, 255]),
        ] {
            self.color = color;
            self.place_canvas_shape(kind, center);
        }
        self.tool = Tool::Shape(ShapeKind::Star);
        self.color = [45, 104, 238, 255];
    }
}
fn tile(ui: &mut egui::Ui, kind: ShapeKind, selected: bool) -> egui::Response {
    let key = egui::Id::new(("2d-shape-thumbnail", kind.label()));
    let texture = ui
        .ctx()
        .data(|d| d.get_temp::<egui::TextureHandle>(key))
        .unwrap_or_else(|| {
            let mut raster = Raster::new(100, 72, [0; 4]);
            raster.draw_shape(
                kind,
                [22.0, 10.0],
                [78.0, 60.0],
                [167, 192, 245, 255],
                3.0,
                kind != ShapeKind::Line,
            );
            let texture = ui.ctx().load_texture(
                kind.label(),
                egui::ColorImage::from_rgba_unmultiplied([100, 72], &raster.pixels),
                egui::TextureOptions::LINEAR,
            );
            ui.ctx().data_mut(|d| d.insert_temp(key, texture.clone()));
            texture
        });
    let (rect, response) = ui.allocate_exact_size(Vec2::new(82.0, 72.0), Sense::click_and_drag());
    let hover =
        ui.ctx()
            .animate_bool_with_time(response.id, response.hovered(), ui.style().animation_time);
    let fill = if selected {
        ui.visuals().selection.bg_fill
    } else {
        ui.visuals().faint_bg_color
    };
    ui.painter().rect_filled(rect, 8.0, fill);
    ui.painter().rect_filled(
        rect,
        8.0,
        ui.visuals()
            .widgets
            .hovered
            .bg_fill
            .linear_multiply(hover * 0.5),
    );
    ui.painter().image(
        texture.id(),
        Rect::from_min_size(rect.min + Vec2::new(3.0, 0.0), Vec2::new(76.0, 52.0)),
        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
        Color32::WHITE,
    );
    ui.painter().text(
        rect.center_bottom() - Vec2::new(0.0, 11.0),
        egui::Align2::CENTER_CENTER,
        kind.label(),
        egui::FontId::proportional(12.0),
        ui.visuals().text_color(),
    );
    response.on_hover_text("Choose to draw, or drag onto the canvas")
}
