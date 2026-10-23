//! Embedded editor screenshots for the illustrated guide.
use eframe::egui::{self, Color32, Rect, Vec2};
pub fn photo(ctx: &egui::Context, index: usize) -> egui::TextureHandle {
    let index = index % 8;
    let key = egui::Id::new(("guide-photo", index));
    if let Some(texture) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(key)) {
        return texture;
    }
    let bytes: &[u8] = match index {
        0 => include_bytes!("../../assets/help/paint.png"),
        1 => include_bytes!("../../assets/help/transparency.png"),
        2 => include_bytes!("../../assets/help/selection.png"),
        3 => include_bytes!("../../assets/help/scene.png"),
        4 => include_bytes!("../../assets/help/export.png"),
        5 => include_bytes!("../../assets/help/settings.png"),
        6 => include_bytes!("../../assets/help/scene-paint.png"),
        _ => include_bytes!("../../assets/help/shapes.png"),
    };
    let decoded = image::load_from_memory(bytes)
        .expect("validated embedded guide image")
        .to_rgba8();
    let texture = ctx.load_texture(
        format!("guide-{index}"),
        egui::ColorImage::from_rgba_unmultiplied(
            [decoded.width() as usize, decoded.height() as usize],
            decoded.as_raw(),
        ),
        egui::TextureOptions::LINEAR,
    );
    ctx.data_mut(|d| d.insert_temp(key, texture.clone()));
    texture
}
pub fn paint(ctx: &egui::Context, painter: &egui::Painter, rect: Rect, index: usize, opacity: f32) {
    let texture = photo(ctx, index);
    painter.image(
        texture.id(),
        rect,
        Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
        Color32::WHITE.linear_multiply(opacity),
    );
}
pub fn callout(p: &egui::Painter, image: Rect, point: [f32; 2], radius: f32, number: usize) {
    let center = image.min + Vec2::new(point[0] * image.width(), point[1] * image.height());
    let r = radius * image.width();
    p.circle_stroke(
        center,
        r,
        egui::Stroke::new(5.0_f32, Color32::from_black_alpha(100)),
    );
    p.circle_stroke(
        center,
        r,
        egui::Stroke::new(2.5_f32, Color32::from_rgb(255, 195, 94)),
    );
    let badge = center + Vec2::new(r * 0.75, -r * 0.75);
    p.circle_filled(badge, 11.0, Color32::from_rgb(255, 195, 94));
    p.text(
        badge,
        egui::Align2::CENTER_CENTER,
        number.to_string(),
        egui::FontId::proportional(12.0),
        Color32::from_rgb(35, 32, 42),
    );
}
