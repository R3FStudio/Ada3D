//! Logo splash and portable startup transitions.
use super::{appearance, icons};
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

pub const FADE_SECONDS: f32 = 0.28;

pub fn ease(value: f32) -> f32 {
    let t = value.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

pub fn splash_opacity(elapsed: f32, exiting: Option<f32>, reduced: bool) -> f32 {
    if reduced {
        return if exiting.is_some() { 0.0 } else { 1.0 };
    }
    ease(elapsed / FADE_SECONDS) * (1.0 - ease(exiting.unwrap_or(0.0) / FADE_SECONDS))
}

pub fn paint(
    ctx: &egui::Context,
    progress: f32,
    opacity: f32,
    elapsed: f32,
    reduced: bool,
    status: &str,
) {
    let colors = appearance::palette(ctx);
    egui::CentralPanel::default()
        .frame(
            egui::Frame::new()
                .fill(Color32::TRANSPARENT)
                .inner_margin(0),
        )
        .show(ctx, |ui| {
            let rect = ui.max_rect();
            let mut p = ui.painter().clone();
            p.set_opacity(opacity);
            let dark = ctx.theme() == egui::Theme::Dark;
            appearance::rounded_gradient(
                &p,
                rect,
                12.0,
                if dark {
                    Color32::from_rgb(17, 22, 40)
                } else {
                    Color32::from_rgb(242, 247, 255)
                },
                if dark {
                    Color32::from_rgb(29, 25, 49)
                } else {
                    Color32::from_rgb(240, 235, 255)
                },
            );
            let center = rect.center() - Vec2::new(0.0, 43.0);
            // Soft concentric color fields frame a single, original logo image.
            for ring in (1..=24).rev() {
                let radius = 66.0 + ring as f32 * 4.0;
                p.circle_filled(
                    center,
                    radius,
                    Color32::from_rgba_unmultiplied(90, 104, 241, 2),
                );
            }
            for ring in 0..3 {
                let radius = 94.0 + ring as f32 * 23.0;
                p.circle_stroke(
                    center,
                    radius,
                    Stroke::new(0.8_f32, colors.muted.linear_multiply(0.10)),
                );
            }
            let lift = if reduced {
                0.0
            } else {
                (1.0 - ease(elapsed / 0.45)) * 12.0
            };
            let logo = icons::logo_texture(ctx);
            p.image(
                logo.id(),
                Rect::from_center_size(center + Vec2::new(0.0, lift), Vec2::splat(116.0)),
                Rect::from_min_max(Pos2::ZERO, egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
            p.text(
                center + Vec2::new(0.0, 94.0),
                egui::Align2::CENTER_CENTER,
                "Ada3D",
                egui::FontId::proportional(32.0),
                ui.visuals().text_color(),
            );
            p.text(
                center + Vec2::new(0.0, 129.0),
                egui::Align2::CENTER_CENTER,
                "R3F Studio",
                egui::FontId::proportional(13.0),
                colors.muted,
            );
            let width = (rect.width() - 112.0).clamp(220.0, 340.0);
            let track = Rect::from_center_size(
                egui::pos2(rect.center().x, rect.bottom() - 75.0),
                Vec2::new(width, 5.0),
            );
            p.rect_filled(track, 3.0, colors.muted.linear_multiply(0.16));
            let fill = Rect::from_min_size(
                track.min,
                Vec2::new(track.width() * progress.clamp(0.0, 1.0), 5.0),
            );
            if fill.width() > 0.5 {
                appearance::rounded_gradient(
                    &p,
                    fill,
                    3.0,
                    Color32::from_rgb(61, 123, 255),
                    Color32::from_rgb(154, 107, 244),
                );
            }
            p.text(
                egui::pos2(rect.center().x, track.bottom() + 20.0),
                egui::Align2::CENTER_CENTER,
                if !status.is_empty() {
                    status
                } else if progress >= 0.995 {
                    "Your workspace is ready"
                } else {
                    "Preparing your workspace"
                },
                egui::FontId::proportional(12.0),
                colors.muted,
            );
            p.text(
                egui::pos2(rect.left() + 24.0, rect.bottom() - 22.0),
                egui::Align2::LEFT_CENTER,
                "DRAW  /  SHAPE  /  CREATE",
                egui::FontId::proportional(9.0),
                colors.muted.linear_multiply(0.65),
            );
            p.text(
                egui::pos2(rect.right() - 24.0, rect.bottom() - 22.0),
                egui::Align2::RIGHT_CENTER,
                env!("CARGO_PKG_VERSION"),
                egui::FontId::proportional(10.0),
                colors.muted.linear_multiply(0.65),
            );
        });
}

/// Fade the editor into view without modifying the native caption controls.
pub fn reveal(ctx: &egui::Context, frame: &eframe::Frame, elapsed: f32) {
    let alpha = 1.0 - ease(elapsed / FADE_SECONDS);
    if alpha <= 0.0 {
        return;
    }
    let p = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("startup-reveal"),
    ));
    let screen = ctx.screen_rect();
    let header = crate::native_window::header_content_rect(ctx, frame);
    let panel = appearance::palette(ctx).panel;
    let color = Color32::from_rgb(panel.r(), panel.g(), panel.b()).gamma_multiply(alpha);
    p.rect_filled(header, 0.0, color);
    p.rect_filled(
        Rect::from_min_max(egui::pos2(screen.left(), header.bottom()), screen.max),
        0.0,
        color,
    );
    ctx.request_repaint_after(std::time::Duration::from_millis(16));
}

#[cfg(test)]
mod tests {
    #[test]
    fn transitions_are_bounded_and_reduced_motion_is_immediate() {
        assert_eq!(super::splash_opacity(0.0, None, false), 0.0);
        assert_eq!(super::splash_opacity(1.0, None, false), 1.0);
        assert_eq!(
            super::splash_opacity(1.0, Some(super::FADE_SECONDS), false),
            0.0
        );
        assert_eq!(super::splash_opacity(0.0, None, true), 1.0);
        assert_eq!(super::splash_opacity(1.0, Some(0.0), true), 0.0);
        let samples: Vec<_> = (0..=10).map(|i| super::ease(i as f32 / 10.0)).collect();
        assert!(samples.windows(2).all(|v| v[0] <= v[1]));
    }
}
