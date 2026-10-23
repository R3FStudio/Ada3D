//! Shape outlines shared by thumbnails and canvas rasterization.
use super::ShapeKind;
impl ShapeKind {
    pub const ALL: [Self; 12] = [
        Self::Line,
        Self::Rectangle,
        Self::Ellipse,
        Self::RoundedRectangle,
        Self::Triangle,
        Self::Diamond,
        Self::Pentagon,
        Self::Hexagon,
        Self::Star,
        Self::Arrow,
        Self::Heart,
        Self::Cross,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Line => "Line",
            Self::Rectangle => "Rectangle",
            Self::Ellipse => "Ellipse",
            Self::RoundedRectangle => "Rounded",
            Self::Triangle => "Triangle",
            Self::Diamond => "Diamond",
            Self::Pentagon => "Pentagon",
            Self::Hexagon => "Hexagon",
            Self::Star => "Star",
            Self::Arrow => "Arrow",
            Self::Heart => "Heart",
            Self::Cross => "Cross",
        }
    }
    pub(super) fn polygon(self, left: f64, top: f64, right: f64, bottom: f64) -> Vec<[f64; 2]> {
        use std::f64::consts::{PI, TAU};
        let vertices = match self {
            Self::Triangle => vec![[0.5, 0.0], [1.0, 1.0], [0.0, 1.0]],
            Self::Diamond => vec![[0.5, 0.0], [1.0, 0.5], [0.5, 1.0], [0.0, 0.5]],
            Self::Arrow => vec![
                [0.0, 0.3],
                [0.6, 0.3],
                [0.6, 0.0],
                [1.0, 0.5],
                [0.6, 1.0],
                [0.6, 0.7],
                [0.0, 0.7],
            ],
            Self::Cross => vec![
                [0.33, 0.0],
                [0.67, 0.0],
                [0.67, 0.33],
                [1.0, 0.33],
                [1.0, 0.67],
                [0.67, 0.67],
                [0.67, 1.0],
                [0.33, 1.0],
                [0.33, 0.67],
                [0.0, 0.67],
                [0.0, 0.33],
                [0.33, 0.33],
            ],
            Self::Heart => (0..64)
                .map(|i| {
                    let t = i as f64 * TAU / 64.0;
                    [
                        0.5 + t.sin().powi(3) * 0.5,
                        (13.0
                            - (13.0 * t.cos()
                                - 5.0 * (2.0 * t).cos()
                                - 2.0 * (3.0 * t).cos()
                                - (4.0 * t).cos()))
                            / 30.0,
                    ]
                })
                .collect(),
            Self::Pentagon | Self::Hexagon | Self::Star => {
                let count = match self {
                    Self::Pentagon => 5,
                    Self::Hexagon => 6,
                    _ => 10,
                };
                (0..count)
                    .map(|i| {
                        let angle = i as f64 * TAU / count as f64 - PI / 2.0;
                        let radius = if self == Self::Star && i % 2 == 1 {
                            0.22
                        } else {
                            0.5
                        };
                        [0.5 + radius * angle.cos(), 0.5 + radius * angle.sin()]
                    })
                    .collect()
            }
            _ => Vec::new(),
        };
        vertices
            .into_iter()
            .map(|p| [left + p[0] * (right - left), top + p[1] * (bottom - top)])
            .collect()
    }
}
pub(super) fn polygon_distance(point: [f64; 2], vertices: &[[f64; 2]]) -> f64 {
    let mut distance = f64::INFINITY;
    let mut inside = false;
    for i in 0..vertices.len() {
        let a = vertices[i];
        let b = vertices[(i + 1) % vertices.len()];
        let edge = [b[0] - a[0], b[1] - a[1]];
        let delta = [point[0] - a[0], point[1] - a[1]];
        let length = edge[0] * edge[0] + edge[1] * edge[1];
        let t = if length > 0.0 {
            ((delta[0] * edge[0] + delta[1] * edge[1]) / length).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let d = [delta[0] - t * edge[0], delta[1] - t * edge[1]];
        distance = distance.min(d[0] * d[0] + d[1] * d[1]);
        if (a[1] > point[1]) != (b[1] > point[1])
            && point[0] < (b[0] - a[0]) * (point[1] - a[1]) / (b[1] - a[1]) + a[0]
        {
            inside = !inside;
        }
    }
    let distance = distance.sqrt();
    if inside { -distance } else { distance }
}
