use eframe::egui::{self, Color32, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2};

#[derive(Clone, Copy)]
pub enum Icon {
    Brush,
    Pencil,
    Marker,
    New,
    Open,
    Eraser,
    Fill,
    Dropper,
    Shapes,
    Cube,
    Canvas,
    Select,
    Save,
    Help,
    Settings,
    Undo,
    Redo,
    Back,
    Import,
    Export,
    Home,
    Delete,
    Fit,
    Menu,
}

/// Shared menu row with aligned icon, label and optional keyboard shortcut.
pub fn menu_item(ui: &mut Ui, icon: Icon, label: &str, shortcut: &str) -> Response {
    ui.set_width(300.0);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(300.0, 29.0), Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    let hover = ui.ctx().animate_bool_with_time(
        response.id.with("hover"),
        response.hovered(),
        ui.style().animation_time,
    );
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        5.0,
        ui.visuals().widgets.hovered.bg_fill.linear_multiply(hover),
    );
    let color = ui.visuals().text_color();
    draw(
        painter,
        icon,
        Rect::from_center_size(rect.left_center() + Vec2::new(15.0, 0.0), Vec2::splat(17.0)),
        color,
    );
    painter.text(
        rect.left_center() + Vec2::new(33.0, 0.0),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(13.0),
        color,
    );
    painter.text(
        rect.right_center() - Vec2::new(10.0, 0.0),
        egui::Align2::RIGHT_CENTER,
        shortcut,
        egui::FontId::proportional(11.0),
        ui.visuals().weak_text_color(),
    );
    response
}

pub fn button(ui: &mut Ui, icon: Icon, label: &str, selected: bool) -> Response {
    control(ui, icon, label, selected, Vec2::new(92.0, 26.0), false)
}
pub fn ribbon(ui: &mut Ui, icon: Icon, label: &str, selected: bool) -> Response {
    control(ui, icon, label, selected, Vec2::new(94.0, 26.0), false)
}
pub fn tile(ui: &mut Ui, icon: Icon, label: &str, selected: bool) -> Response {
    let (r, response) = ui.allocate_exact_size(Vec2::new(82.0, 58.0), Sense::click());
    let hover = ui.ctx().animate_bool_with_time(
        response.id.with("hover"),
        response.hovered(),
        ui.style().animation_time,
    );
    let active = ui.ctx().animate_bool_with_time(
        response.id.with("active"),
        selected,
        ui.style().animation_time,
    );
    let base = ui.visuals().faint_bg_color;
    if active > 0.0 {
        crate::appearance::rounded_gradient(
            ui.painter(),
            r,
            10.0,
            ui.visuals().selection.bg_fill.linear_multiply(active),
            Color32::from_rgb(98, 79, 225).linear_multiply(active),
        );
    } else {
        ui.painter().rect_filled(r, 10.0, base);
        ui.painter().rect_filled(
            r,
            10.0,
            ui.visuals().widgets.hovered.bg_fill.linear_multiply(hover),
        );
    }
    let color = if selected {
        Color32::WHITE
    } else {
        ui.visuals().text_color()
    };
    draw(
        ui.painter(),
        icon,
        Rect::from_center_size(r.center_top() + Vec2::new(0.0, 20.0), Vec2::splat(25.0)),
        color,
    );
    ui.painter().text(
        r.center_bottom() - Vec2::new(0.0, 11.0),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(11.0),
        color,
    );
    if label.is_empty() {
        response
    } else {
        response.on_hover_text(label)
    }
}
pub fn action(ui: &mut Ui, icon: Icon, label: &str) -> Response {
    let width = action_width(ui, label);
    control(ui, icon, label, false, Vec2::new(width, 26.0), false)
}
pub fn action_width(ui: &Ui, label: &str) -> f32 {
    if label.is_empty() {
        28.0
    } else {
        38.0 + ui
            .painter()
            .layout_no_wrap(
                label.to_owned(),
                egui::FontId::proportional(11.0),
                Color32::WHITE,
            )
            .size()
            .x
    }
}
pub fn primary_action(ui: &mut Ui, icon: Icon, label: &str) -> Response {
    control(
        ui,
        icon,
        label,
        true,
        Vec2::new(action_width(ui, label), 28.0),
        false,
    )
}
fn control(
    ui: &mut Ui,
    icon: Icon,
    label: &str,
    selected: bool,
    size: Vec2,
    vertical: bool,
) -> Response {
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let press = ui.ctx().animate_bool_with_time(
        response.id.with("press"),
        response.is_pointer_button_down_on(),
        ui.style().animation_time,
    );
    let rect = rect.shrink(press * 1.5);
    let accent = ui.visuals().selection.bg_fill;
    let active = ui.ctx().animate_bool_with_time(
        response.id.with("selection"),
        selected,
        ui.style().animation_time,
    );
    if active > 0.0 {
        crate::appearance::rounded_gradient(
            ui.painter(),
            rect,
            10.0,
            accent.linear_multiply(active),
            Color32::from_rgb(114, 73, 215).linear_multiply(active),
        );
    } else {
        let hover = ui.ctx().animate_bool_with_time(
            response.id.with("hover"),
            response.hovered(),
            ui.style().animation_time,
        );
        ui.painter().rect_filled(
            rect,
            6.0,
            ui.visuals().widgets.hovered.bg_fill.linear_multiply(hover),
        );
    }
    if !selected && size.y >= 60.0 {
        ui.painter().rect_stroke(
            rect,
            4.0,
            Stroke::new(1.0_f32, ui.visuals().widgets.noninteractive.bg_stroke.color),
            egui::StrokeKind::Inside,
        );
    }
    let color = if selected {
        Color32::WHITE
    } else {
        ui.visuals().text_color()
    };
    let center = if vertical {
        rect.center_top() + Vec2::new(0.0, 14.0)
    } else {
        rect.left_center() + Vec2::new(if label.is_empty() { 14.0 } else { 15.0 }, 0.0)
    };
    draw(
        ui.painter(),
        icon,
        Rect::from_center_size(center, Vec2::splat(if vertical { 19.0 } else { 16.0 })),
        color,
    );
    ui.painter().text(
        if vertical {
            rect.center_bottom() - Vec2::new(0.0, 8.0)
        } else {
            rect.left_center() + Vec2::new(29.0, 0.0)
        },
        if vertical {
            egui::Align2::CENTER_CENTER
        } else {
            egui::Align2::LEFT_CENTER
        },
        label,
        egui::FontId::proportional(11.0),
        color,
    );
    if label.is_empty() {
        response
    } else {
        response.on_hover_text(label)
    }
}

pub fn draw(p: &egui::Painter, icon: Icon, r: Rect, c: Color32) {
    let s = Stroke::new(1.7_f32, c);
    let q = |x: f32, y: f32| Pos2::new(r.left() + x * r.width(), r.top() + y * r.height());
    match icon {
        Icon::Menu => {
            for y in [0.25, 0.5, 0.75] {
                p.line_segment([q(0.15, y), q(0.85, y)], s);
            }
        }
        Icon::Import | Icon::Export => {
            p.line_segment([q(0.15, 0.62), q(0.15, 0.87)], s);
            p.line_segment([q(0.15, 0.87), q(0.85, 0.87)], s);
            p.line_segment([q(0.85, 0.87), q(0.85, 0.62)], s);
            let down = matches!(icon, Icon::Import);
            let a = if down { 0.12 } else { 0.65 };
            let b = if down { 0.65 } else { 0.12 };
            p.line_segment([q(0.5, a), q(0.5, b)], s);
            let tail = if down { b - 0.22 } else { b + 0.22 };
            p.line_segment([q(0.28, tail), q(0.5, b)], s);
            p.line_segment([q(0.72, tail), q(0.5, b)], s);
        }
        Icon::Home => {
            p.add(egui::Shape::line(
                vec![q(0.1, 0.45), q(0.5, 0.12), q(0.9, 0.45)],
                s,
            ));
            p.rect_stroke(
                Rect::from_two_pos(q(0.25, 0.45), q(0.75, 0.9)),
                1.0,
                s,
                egui::StrokeKind::Inside,
            );
        }
        Icon::Delete => {
            p.line_segment([q(0.15, 0.25), q(0.85, 0.25)], s);
            p.line_segment([q(0.36, 0.12), q(0.64, 0.12)], s);
            p.rect_stroke(
                Rect::from_two_pos(q(0.25, 0.3), q(0.75, 0.88)),
                2.0,
                s,
                egui::StrokeKind::Inside,
            );
            for x in [0.42, 0.58] {
                p.line_segment([q(x, 0.43), q(x, 0.74)], s);
            }
        }
        Icon::Fit => {
            for (x, y, dx, dy) in [
                (0.15, 0.15, 1.0, 1.0),
                (0.85, 0.15, -1.0, 1.0),
                (0.15, 0.85, 1.0, -1.0),
                (0.85, 0.85, -1.0, -1.0),
            ] {
                p.line_segment([q(x, y), q(x + dx * 0.2, y)], s);
                p.line_segment([q(x, y), q(x, y + dy * 0.2)], s);
            }
        }
        Icon::Back => {
            p.line_segment([q(0.85, 0.5), q(0.15, 0.5)], s);
            p.line_segment([q(0.15, 0.5), q(0.45, 0.2)], s);
            p.line_segment([q(0.15, 0.5), q(0.45, 0.8)], s);
        }
        Icon::New | Icon::Open => {
            p.rect_stroke(r.shrink(3.0), 3.0, s, egui::StrokeKind::Inside);
            if matches!(icon, Icon::New) {
                p.line_segment([q(0.3, 0.5), q(0.7, 0.5)], s);
                p.line_segment([q(0.5, 0.3), q(0.5, 0.7)], s);
            } else {
                p.line_segment([q(0.1, 0.35), q(0.9, 0.35)], s);
                p.line_segment([q(0.15, 0.15), q(0.4, 0.15)], Stroke::new(4.0_f32, c));
            }
        }
        Icon::Pencil | Icon::Marker => {
            let width: f32 = if matches!(icon, Icon::Marker) {
                6.0
            } else {
                3.0
            };
            p.line_segment([q(0.25, 0.7), q(0.75, 0.2)], Stroke::new(width, c));
            p.add(egui::Shape::convex_polygon(
                vec![q(0.1, 0.9), q(0.17, 0.66), q(0.34, 0.83)],
                c,
                Stroke::NONE,
            ));
            p.line_segment([q(0.62, 0.16), q(0.81, 0.35)], s);
        }
        Icon::Brush => {
            p.line_segment([q(0.30, 0.62), q(0.74, 0.15)], Stroke::new(4.0_f32, c));
            p.add(egui::Shape::convex_polygon(
                vec![q(0.08, 0.93), q(0.16, 0.62), q(0.39, 0.80)],
                c,
                Stroke::NONE,
            ));
        }
        Icon::Eraser => {
            p.add(egui::Shape::convex_polygon(
                vec![q(0.12, 0.65), q(0.58, 0.13), q(0.90, 0.42), q(0.43, 0.94)],
                Color32::TRANSPARENT,
                s,
            ));
            p.line_segment([q(0.37, 0.39), q(0.69, 0.69)], s);
        }
        Icon::Fill => {
            p.add(egui::Shape::convex_polygon(
                vec![q(0.12, 0.45), q(0.48, 0.10), q(0.82, 0.45), q(0.48, 0.81)],
                Color32::TRANSPARENT,
                s,
            ));
            p.line_segment([q(0.19, 0.50), q(0.73, 0.50)], s);
            p.circle_filled(q(0.86, 0.83), 2.4, c);
        }
        Icon::Dropper => {
            p.line_segment([q(0.14, 0.87), q(0.70, 0.31)], Stroke::new(3.0_f32, c));
            p.line_segment([q(0.53, 0.16), q(0.87, 0.50)], s);
        }
        Icon::Shapes => {
            p.circle_stroke(q(0.35, 0.36), r.width() * 0.27, s);
            p.rect_stroke(
                Rect::from_two_pos(q(0.43, 0.45), q(0.95, 0.95)),
                1.0,
                s,
                egui::StrokeKind::Inside,
            );
        }
        Icon::Cube => {
            let a = q(0.12, 0.29);
            let b = q(0.50, 0.09);
            let d = q(0.88, 0.29);
            let e = q(0.50, 0.51);
            for points in [
                [a, b],
                [b, d],
                [d, e],
                [e, a],
                [a, q(0.12, 0.74)],
                [e, q(0.50, 0.96)],
                [d, q(0.88, 0.74)],
                [q(0.12, 0.74), q(0.50, 0.96)],
                [q(0.50, 0.96), q(0.88, 0.74)],
            ] {
                p.line_segment(points, s);
            }
        }
        Icon::Canvas => {
            p.rect_stroke(r.shrink(2.0), 2.0, s, egui::StrokeKind::Inside);
            p.line_segment([q(0.1, 0.70), q(0.42, 0.38)], s);
            p.line_segment([q(0.42, 0.38), q(0.76, 0.82)], s);
            p.circle_filled(q(0.73, 0.28), 2.0, c);
        }
        Icon::Save => {
            p.rect_stroke(r.shrink(2.0), 2.0, s, egui::StrokeKind::Inside);
            p.rect_stroke(
                Rect::from_two_pos(q(0.3, 0.1), q(0.7, 0.4)),
                1.0,
                s,
                egui::StrokeKind::Inside,
            );
            p.rect_stroke(
                Rect::from_two_pos(q(0.25, 0.6), q(0.75, 0.9)),
                1.0,
                s,
                egui::StrokeKind::Inside,
            );
        }
        Icon::Help => {
            p.circle_stroke(r.center(), r.width() * 0.42, s);
            p.text(
                r.center(),
                egui::Align2::CENTER_CENTER,
                "?",
                egui::FontId::proportional(r.height() * 0.8),
                c,
            );
        }
        Icon::Settings => {
            p.circle_stroke(r.center(), r.width() * 0.28, s);
            p.circle_stroke(r.center(), r.width() * 0.1, s);
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::TAU / 8.0;
                let v = Vec2::new(a.cos(), a.sin());
                p.line_segment(
                    [
                        r.center() + v * r.width() * 0.27,
                        r.center() + v * r.width() * 0.43,
                    ],
                    s,
                );
            }
        }
        Icon::Undo | Icon::Redo => {
            let mirror = matches!(icon, Icon::Redo);
            let f = |x, y| q(if mirror { 1.0 - x } else { x }, y);
            p.add(egui::Shape::line(
                vec![
                    f(0.2, 0.3),
                    f(0.65, 0.3),
                    f(0.85, 0.5),
                    f(0.75, 0.8),
                    f(0.4, 0.8),
                ],
                s,
            ));
            p.line_segment([f(0.2, 0.3), f(0.4, 0.1)], s);
            p.line_segment([f(0.2, 0.3), f(0.4, 0.5)], s);
        }
        Icon::Select => {
            p.add(egui::Shape::convex_polygon(
                vec![q(0.18, 0.07), q(0.86, 0.63), q(0.54, 0.64), q(0.43, 0.95)],
                Color32::TRANSPARENT,
                s,
            ));
        }
    }
}

/// Use the same transparent mark for native windows and in-app branding.
pub fn window_icon() -> egui::IconData {
    let image = image::load_from_memory(include_bytes!("../../assets/brand/app-icon.png"))
        .expect("bundled Ada icon")
        .into_rgba8();
    egui::IconData {
        width: image.width(),
        height: image.height(),
        rgba: image.into_raw(),
    }
}
pub fn logo(ui: &mut Ui, size: f32) {
    let texture = logo_texture(ui.ctx());
    ui.image((texture.id(), Vec2::splat(size)));
}

pub fn title_logo(ui: &mut Ui) {
    let texture = logo_texture(ui.ctx());
    let (rect, _) = ui.allocate_exact_size(Vec2::new(28.0, 26.0), Sense::hover());
    ui.painter().image(
        texture.id(),
        Rect::from_center_size(rect.center(), Vec2::splat(20.0)),
        Rect::from_min_max(Pos2::ZERO, egui::pos2(1.0, 1.0)),
        Color32::WHITE,
    );
}

pub fn logo_texture(ctx: &egui::Context) -> egui::TextureHandle {
    let key = egui::Id::new("adars-logo-texture");
    if let Some(texture) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(key)) {
        texture
    } else {
        let texture = ctx.load_texture(
            "adars-mark",
            {
                let image =
                    image::load_from_memory(include_bytes!("../../assets/brand/app-icon.png"))
                        .expect("bundled Ada logo")
                        .into_rgba8();
                egui::ColorImage::from_rgba_unmultiplied(
                    [image.width() as usize, image.height() as usize],
                    image.as_raw(),
                )
            },
            egui::TextureOptions::LINEAR,
        );
        ctx.data_mut(|d| d.insert_temp(key, texture.clone()));
        texture
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn native_mark_has_no_background_and_keeps_antialiased_edges() {
        let icon = super::window_icon();
        assert_eq!((icon.width, icon.height), (256, 256));
        for (x, y) in [(0, 0), (32, 32), (224, 32), (32, 224), (224, 224)] {
            assert_eq!(icon.rgba[(y * 256 + x) * 4 + 3], 0);
        }
        assert_eq!(icon.rgba[(128 * 256 + 128) * 4 + 3], 255);
        assert!(
            icon.rgba
                .as_chunks::<4>()
                .0
                .iter()
                .any(|p| p[3] > 0 && p[3] < 255)
        );
    }
}
