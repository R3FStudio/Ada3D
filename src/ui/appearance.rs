use adars::settings::{Accent, Backdrop, Preferences, ThemeMode};
use eframe::egui::{self, Color32, Rect, Stroke};

#[derive(Clone, Copy)]
pub struct Palette {
    pub panel: Color32,
    pub workspace_top: Color32,
    pub workspace_bottom: Color32,
    pub muted: Color32,
    pub border: Color32,
    pub grid: Color32,
}
pub fn palette(ctx: &egui::Context) -> Palette {
    let glass = ctx.data(|d| d.get_temp::<bool>(egui::Id::new("glass")).unwrap_or(false));
    let mica = ctx.data(|d| {
        d.get_temp::<bool>(egui::Id::new("windows-mica"))
            .unwrap_or(false)
    });
    let tint = |r, g, b| {
        Color32::from_rgba_unmultiplied(
            r,
            g,
            b,
            if glass {
                if ctx.theme() == egui::Theme::Dark {
                    142
                } else {
                    164
                }
            } else {
                255
            },
        )
    };
    if ctx.theme() == egui::Theme::Dark {
        Palette {
            panel: if mica {
                tint(29, 32, 40)
            } else {
                tint(30, 30, 30)
            },
            workspace_top: if mica {
                tint(36, 40, 51)
            } else {
                tint(37, 37, 37)
            },
            workspace_bottom: if mica {
                tint(19, 22, 29)
            } else {
                tint(22, 22, 22)
            },
            muted: Color32::from_rgb(173, 173, 173),
            border: Color32::from_rgb(65, 65, 65),
            grid: Color32::from_rgb(61, 61, 61),
        }
    } else {
        Palette {
            panel: if mica {
                tint(247, 249, 253)
            } else {
                tint(247, 247, 247)
            },
            workspace_top: if mica {
                tint(208, 215, 228)
            } else {
                tint(218, 218, 218)
            },
            workspace_bottom: if mica {
                tint(235, 239, 247)
            } else {
                tint(238, 238, 238)
            },
            muted: Color32::from_rgb(100, 100, 100),
            border: Color32::from_rgb(212, 212, 212),
            grid: Color32::from_rgb(192, 192, 192),
        }
    }
}
pub fn accent(value: Accent) -> Color32 {
    let [r, g, b] = value.rgb();
    Color32::from_rgb(r, g, b)
}
pub fn install(ctx: &egui::Context, prefs: &Preferences) {
    ctx.data_mut(|d| {
        d.insert_temp(egui::Id::new("glass"), prefs.glass_effects);
        d.insert_temp(
            egui::Id::new("windows-mica"),
            cfg!(target_os = "windows") && prefs.backdrop == Backdrop::Mica,
        );
    });
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        let mut style = egui::Style {
            visuals: match theme {
                egui::Theme::Light => egui::Visuals::light(),
                egui::Theme::Dark => egui::Visuals::dark(),
            },
            ..Default::default()
        };
        let panel = if theme == egui::Theme::Dark {
            Color32::from_rgb(30, 30, 30)
        } else {
            Color32::from_rgb(247, 247, 247)
        };
        style.visuals.override_text_color = Some(if theme == egui::Theme::Dark {
            Color32::from_rgb(232, 235, 244)
        } else {
            Color32::from_rgb(37, 46, 65)
        });
        style.visuals.panel_fill = panel;
        style.visuals.window_fill = panel;
        style.visuals.window_corner_radius = egui::CornerRadius::same(12);
        style.visuals.menu_corner_radius = egui::CornerRadius::same(10);
        style.visuals.popup_shadow = egui::epaint::Shadow {
            offset: [0, 6],
            blur: 20,
            spread: 0,
            color: Color32::from_black_alpha(85),
        };
        style.visuals.window_shadow = style.visuals.popup_shadow;
        for widget in [
            &mut style.visuals.widgets.inactive,
            &mut style.visuals.widgets.hovered,
            &mut style.visuals.widgets.active,
        ] {
            widget.corner_radius = egui::CornerRadius::same(6);
        }
        style.visuals.selection.bg_fill = accent(prefs.accent);
        style.visuals.selection.stroke = Stroke::new(1.0_f32, Color32::WHITE);
        style.visuals.widgets.active.bg_fill = accent(prefs.accent);
        style.visuals.widgets.active.fg_stroke = Stroke::new(
            1.0_f32,
            if theme == egui::Theme::Dark {
                Color32::WHITE
            } else {
                Color32::from_rgb(37, 46, 65)
            },
        );
        style.spacing.item_spacing = egui::vec2(6.0, 4.0);
        style.spacing.button_padding = egui::vec2(6.0, 3.0);
        style.spacing.interact_size = egui::vec2(28.0, 22.0);
        style.spacing.slider_width = 110.0;
        style.spacing.text_edit_width = 96.0;
        style
            .text_styles
            .insert(egui::TextStyle::Small, egui::FontId::proportional(10.0));
        style
            .text_styles
            .insert(egui::TextStyle::Heading, egui::FontId::proportional(16.0));
        style.animation_time = if prefs.reduce_motion { 0.0 } else { 0.22 };
        style
            .text_styles
            .insert(egui::TextStyle::Body, egui::FontId::proportional(12.0));
        style
            .text_styles
            .insert(egui::TextStyle::Button, egui::FontId::proportional(12.0));
        ctx.set_style_of(theme, style);
    }
    ctx.set_theme(match prefs.theme {
        ThemeMode::System => egui::ThemePreference::System,
        ThemeMode::Light => egui::ThemePreference::Light,
        ThemeMode::Dark => egui::ThemePreference::Dark,
    });
}
pub fn gradient(p: &egui::Painter, r: Rect, a: Color32, b: Color32, horizontal: bool) {
    let mut mesh = egui::Mesh::default();
    for (pos, color) in [
        (r.left_top(), a),
        (r.right_top(), if horizontal { b } else { a }),
        (r.right_bottom(), b),
        (r.left_bottom(), if horizontal { a } else { b }),
    ] {
        mesh.vertices.push(egui::epaint::Vertex {
            pos,
            uv: egui::epaint::WHITE_UV,
            color,
        });
    }
    mesh.indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
    p.add(egui::Shape::mesh(mesh));
}

pub fn rounded_gradient(p: &egui::Painter, r: Rect, radius: f32, a: Color32, b: Color32) {
    let mut mesh = egui::Mesh::default();
    let vertex = |pos: egui::Pos2| {
        let t = ((pos.x - r.left()) / r.width()).clamp(0.0, 1.0);
        let color = Color32::from_rgba_premultiplied(
            egui::lerp(a.r() as f32..=b.r() as f32, t) as u8,
            egui::lerp(a.g() as f32..=b.g() as f32, t) as u8,
            egui::lerp(a.b() as f32..=b.b() as f32, t) as u8,
            egui::lerp(a.a() as f32..=b.a() as f32, t) as u8,
        );
        egui::epaint::Vertex {
            pos,
            uv: egui::epaint::WHITE_UV,
            color,
        }
    };
    mesh.vertices.push(vertex(r.center()));
    for (center, start) in [
        (
            r.right_top() + egui::vec2(-radius, radius),
            -std::f32::consts::FRAC_PI_2,
        ),
        (r.right_bottom() - egui::vec2(radius, radius), 0.0),
        (
            r.left_bottom() + egui::vec2(radius, -radius),
            std::f32::consts::FRAC_PI_2,
        ),
        (
            r.left_top() + egui::vec2(radius, radius),
            std::f32::consts::PI,
        ),
    ] {
        for step in 0..=6 {
            let angle = start + step as f32 * std::f32::consts::FRAC_PI_2 / 6.0;
            mesh.vertices.push(vertex(
                center + egui::vec2(angle.cos(), angle.sin()) * radius,
            ));
        }
    }
    for i in 1..mesh.vertices.len() as u32 {
        mesh.indices.extend_from_slice(&[
            0,
            i,
            if i + 1 == mesh.vertices.len() as u32 {
                1
            } else {
                i + 1
            },
        ]);
    }
    p.add(egui::Shape::mesh(mesh));
}
