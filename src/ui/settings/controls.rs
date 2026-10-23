//! Animated navigation, choice cards and preference switches.
use crate::{
    appearance,
    icons::{self, Icon},
};
use eframe::egui::{self, Color32, Pos2, Rect, Response, Sense, Stroke, Vec2};

pub(super) fn navigation(ui: &mut egui::Ui, icon: Icon, label: &str, selected: bool) -> Response {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 44.0), Sense::click());
    let active = ui
        .ctx()
        .animate_bool_with_time(response.id, selected, ui.style().animation_time);
    let hover = ui.ctx().animate_bool_with_time(
        response.id.with("hover"),
        response.hovered(),
        ui.style().animation_time,
    );
    ui.painter().rect_filled(
        rect,
        10.0,
        ui.visuals()
            .widgets
            .hovered
            .bg_fill
            .linear_multiply(hover * 0.5),
    );
    ui.painter().rect_filled(
        rect,
        10.0,
        ui.visuals()
            .selection
            .bg_fill
            .linear_multiply(active * 0.22),
    );
    if active > 0.0 {
        ui.painter().rect_filled(
            Rect::from_min_size(
                rect.left_center() - Vec2::new(0.0, 11.0),
                Vec2::new(3.0, 22.0),
            ),
            2.0,
            ui.visuals().selection.bg_fill.linear_multiply(active),
        );
    }
    let color = ui.visuals().text_color();
    icons::draw(
        ui.painter(),
        icon,
        Rect::from_center_size(rect.left_center() + Vec2::new(25.0, 0.0), Vec2::splat(20.0)),
        color,
    );
    ui.painter().text(
        rect.left_center() + Vec2::new(45.0, 0.0),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(15.0),
        color,
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            ui.is_enabled(),
            selected,
            label,
        )
    });
    response
}

pub(super) fn choice(
    ui: &mut egui::Ui,
    width: f32,
    title: &str,
    description: &str,
    selected: bool,
    preview: Option<Option<bool>>,
) -> Response {
    let height = if preview.is_some() { 76.0 } else { 58.0 };
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());
    let active = ui
        .ctx()
        .animate_bool_with_time(response.id, selected, ui.style().animation_time);
    let hover = ui.ctx().animate_bool_with_time(
        response.id.with("hover"),
        response.hovered(),
        ui.style().animation_time,
    );
    ui.painter().rect_filled(
        rect,
        8.0,
        if ui.visuals().dark_mode {
            Color32::from_rgb(46, 46, 46)
        } else {
            Color32::from_rgb(246, 246, 246)
        },
    );
    ui.painter().rect_filled(
        rect,
        10.0,
        ui.visuals()
            .selection
            .bg_fill
            .linear_multiply(active * 0.12 + hover * 0.05),
    );
    ui.painter().rect_stroke(
        rect,
        10.0,
        Stroke::new(
            1.0_f32 + active,
            if selected {
                ui.visuals().selection.bg_fill
            } else {
                appearance::palette(ui.ctx()).border
            },
        ),
        egui::StrokeKind::Inside,
    );
    let mut y = rect.top() + 14.0;
    if let Some(dark) = preview {
        let mini = Rect::from_min_size(
            rect.min + Vec2::new(12.0, 10.0),
            Vec2::new(rect.width() - 24.0, 24.0),
        );
        let light = Color32::from_rgb(235, 235, 235);
        let dark_color = Color32::from_rgb(38, 38, 38);
        match dark {
            Some(true) => {
                ui.painter().rect_filled(mini, 5.0, dark_color);
            }
            Some(false) => {
                ui.painter().rect_filled(mini, 5.0, light);
            }
            None => appearance::rounded_gradient(ui.painter(), mini, 5.0, light, dark_color),
        }
        ui.painter().rect_filled(
            Rect::from_min_size(mini.min + Vec2::new(7.0, 6.0), Vec2::new(18.0, 4.0)),
            2.0,
            ui.visuals().selection.bg_fill,
        );
        ui.painter().rect_filled(
            Rect::from_min_size(
                mini.min + Vec2::new(7.0, 15.0),
                Vec2::new(mini.width() - 32.0, 10.0),
            ),
            2.0,
            Color32::from_white_alpha(190),
        );
        y = rect.top() + 45.0;
    }
    ui.painter().text(
        Pos2::new(rect.left() + 12.0, y),
        egui::Align2::LEFT_CENTER,
        title,
        egui::FontId::proportional(15.0),
        ui.visuals().text_color(),
    );
    ui.painter().text(
        Pos2::new(rect.left() + 12.0, y + 19.0),
        egui::Align2::LEFT_CENTER,
        description,
        egui::FontId::proportional(11.0),
        appearance::palette(ui.ctx()).muted,
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            ui.is_enabled(),
            selected,
            title,
        )
    });
    response
}

pub(super) fn toggle(
    ui: &mut egui::Ui,
    value: &mut bool,
    title: &str,
    description: &str,
    duration: f32,
) {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 50.0), Sense::click());
    if response.clicked() {
        *value = !*value;
    }
    let active = ui
        .ctx()
        .animate_bool_with_time(response.id, *value, duration);
    let switch = Rect::from_center_size(
        rect.right_center() - Vec2::new(25.0, 0.0),
        Vec2::new(42.0, 24.0),
    );
    ui.painter()
        .rect_filled(switch, 12.0, ui.visuals().widgets.inactive.bg_fill);
    ui.painter().rect_filled(
        switch,
        12.0,
        ui.visuals().selection.bg_fill.linear_multiply(active),
    );
    ui.painter().circle_filled(
        Pos2::new(switch.left() + 12.0 + 18.0 * active, switch.center().y),
        8.0,
        Color32::WHITE,
    );
    ui.painter().text(
        rect.left_top() + Vec2::new(0.0, 16.0),
        egui::Align2::LEFT_CENTER,
        title,
        egui::FontId::proportional(15.0),
        ui.visuals().text_color(),
    );
    ui.painter().text(
        rect.left_top() + Vec2::new(0.0, 37.0),
        egui::Align2::LEFT_CENTER,
        description,
        egui::FontId::proportional(12.0),
        appearance::palette(ui.ctx()).muted,
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Checkbox, ui.is_enabled(), *value, title)
    });
}
