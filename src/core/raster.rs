//! Independent, bounded RGBA raster editing. Coordinates use canvas pixels;
//! pixel centers are `(x + 0.5, y + 0.5)`. Stored colors have straight alpha.

use serde::{Deserialize, Serialize};

mod shapes;

pub const MAX_DIMENSION: u32 = 2048;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShapeKind {
    Line,
    Rectangle,
    Ellipse,
    RoundedRectangle,
    Triangle,
    Diamond,
    Pentagon,
    Hexagon,
    Star,
    Arrow,
    Heart,
    Cross,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Raster {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl Raster {
    /// Construct a canvas, clamping both dimensions to `1..=MAX_DIMENSION`.
    pub fn new(width: u32, height: u32, color: [u8; 4]) -> Self {
        let width = width.clamp(1, MAX_DIMENSION);
        let height = height.clamp(1, MAX_DIMENSION);
        let mut pixels = vec![0; width as usize * height as usize * 4];
        for pixel in pixels.as_chunks_mut::<4>().0 {
            pixel.copy_from_slice(&color);
        }
        Self {
            width,
            height,
            pixels,
        }
    }

    /// Out-of-bounds or malformed raster reads return transparent black.
    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        if x >= self.width || y >= self.height {
            return [0; 4];
        }
        let Some(offset) = (y as usize)
            .checked_mul(self.width as usize)
            .and_then(|row| row.checked_add(x as usize))
            .and_then(|index| index.checked_mul(4))
        else {
            return [0; 4];
        };
        let Some(end) = offset.checked_add(4) else {
            return [0; 4];
        };
        let Some(pixel) = self.pixels.get(offset..end) else {
            return [0; 4];
        };
        [pixel[0], pixel[1], pixel[2], pixel[3]]
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.width == 0
            || self.height == 0
            || self.width > MAX_DIMENSION
            || self.height > MAX_DIMENSION
        {
            return Err(format!(
                "Canvas dimensions must be between 1 and {MAX_DIMENSION} pixels."
            ));
        }
        let expected = self.width as usize * self.height as usize * 4;
        if self.pixels.len() != expected {
            return Err("Canvas pixel data does not match its dimensions.".into());
        }
        Ok(())
    }

    pub fn stamp(&mut self, x: f32, y: f32, radius: f32, color: [u8; 4], opacity: f32) {
        self.line([x, y], [x, y], radius, color, opacity);
    }

    /// Paint a round-capped line. Each covered pixel is blended once, so a
    /// single segment does not build opacity from overlapping brush stamps.
    pub fn line(
        &mut self,
        from: [f32; 2],
        to: [f32; 2],
        radius: f32,
        color: [u8; 4],
        opacity: f32,
    ) {
        if !opacity.is_finite() || color[3] == 0 {
            return;
        }
        let opacity = opacity.clamp(0.0, 1.0) as f64;
        if opacity == 0.0 {
            return;
        }
        self.capsule(from, to, radius, |pixel, coverage| {
            blend_pixel(pixel, color, opacity * coverage);
        });
    }

    pub fn erase_stamp(&mut self, x: f32, y: f32, radius: f32) {
        self.erase_line([x, y], [x, y], radius);
    }

    /// Erasure removes alpha instead of painting the background color.
    pub fn erase_line(&mut self, from: [f32; 2], to: [f32; 2], radius: f32) {
        self.erase_line_with_opacity(from, to, radius, 1.0);
    }

    /// Remove alpha by the requested strength without painting a background color.
    pub fn erase_line_with_opacity(
        &mut self,
        from: [f32; 2],
        to: [f32; 2],
        radius: f32,
        strength: f32,
    ) {
        if !strength.is_finite() {
            return;
        }
        let strength = strength.clamp(0.0, 1.0) as f64;
        self.capsule(from, to, radius, |pixel, coverage| {
            pixel[3] = (pixel[3] as f64 * (1.0 - strength * coverage)).round() as u8;
            if pixel[3] == 0 {
                pixel.fill(0);
            }
        });
    }

    fn capsule(
        &mut self,
        from: [f32; 2],
        to: [f32; 2],
        radius: f32,
        mut apply: impl FnMut(&mut [u8], f64),
    ) {
        if self.validate().is_err()
            || !valid_points(from, to)
            || !radius.is_finite()
            || radius <= 0.0
        {
            return;
        }
        let radius = (radius as f64).min(MAX_DIMENSION as f64 * 2.0);
        let a = [from[0] as f64, from[1] as f64];
        let b = [to[0] as f64, to[1] as f64];
        let bounds = [
            a[0].min(b[0]) - radius - 1.0,
            a[1].min(b[1]) - radius - 1.0,
            a[0].max(b[0]) + radius + 1.0,
            a[1].max(b[1]) + radius + 1.0,
        ];
        let [left, top, right, bottom] = self.clip_bounds(bounds);
        for y in top..bottom {
            for x in left..right {
                let distance = segment_distance([x as f64 + 0.5, y as f64 + 0.5], a, b);
                let coverage = (radius + 0.5 - distance).clamp(0.0, 1.0);
                if coverage > 0.0 {
                    let offset = (y as usize * self.width as usize + x as usize) * 4;
                    apply(&mut self.pixels[offset..offset + 4], coverage);
                }
            }
        }
    }

    /// Replace an exact RGBA, four-connected region. Returns whether it changed.
    /// Marking pixels when they enter the work list prevents duplicate visits.
    pub fn fill(&mut self, x: u32, y: u32, color: [u8; 4]) -> bool {
        if self.validate().is_err() || x >= self.width || y >= self.height {
            return false;
        }
        let target = self.pixel(x, y);
        if target == color {
            return false;
        }
        let width = self.width as usize;
        let count = width * self.height as usize;
        let start = y as usize * width + x as usize;
        let mut work = vec![start];
        self.pixels[start * 4..start * 4 + 4].copy_from_slice(&color);
        while let Some(index) = work.pop() {
            let column = index % width;
            let neighbors = [
                (column > 0).then(|| index - 1),
                (column + 1 < width).then(|| index + 1),
                (index >= width).then(|| index - width),
                (index + width < count).then(|| index + width),
            ];
            for neighbor in neighbors.into_iter().flatten() {
                let offset = neighbor * 4;
                if self.pixels[offset..offset + 4] == target {
                    self.pixels[offset..offset + 4].copy_from_slice(&color);
                    work.push(neighbor);
                }
            }
        }
        true
    }

    /// Remove alpha from one exact-RGBA connected region, preserving RGB until alpha reaches zero.
    pub fn fill_transparency(&mut self, x: u32, y: u32, strength: f32) -> bool {
        if !strength.is_finite() || self.validate().is_err() || x >= self.width || y >= self.height
        {
            return false;
        }
        let mut color = self.pixel(x, y);
        color[3] = (color[3] as f32 * (1.0 - strength.clamp(0.0, 1.0))).round() as u8;
        if color[3] == 0 {
            color = [0; 4];
        }
        self.fill(x, y, color)
    }

    pub fn draw_shape(
        &mut self,
        kind: ShapeKind,
        from: [f32; 2],
        to: [f32; 2],
        color: [u8; 4],
        thickness: f32,
        filled: bool,
    ) {
        if self.validate().is_err()
            || !valid_points(from, to)
            || !thickness.is_finite()
            || thickness <= 0.0
            || color[3] == 0
        {
            return;
        }
        let radius = (thickness as f64 * 0.5).clamp(0.25, MAX_DIMENSION as f64 * 2.0);
        if kind == ShapeKind::Line {
            self.line(from, to, radius as f32, color, 1.0);
            return;
        }
        let left = (from[0] as f64).min(to[0] as f64);
        let top = (from[1] as f64).min(to[1] as f64);
        let right = (from[0] as f64).max(to[0] as f64);
        let bottom = (from[1] as f64).max(to[1] as f64);
        let half = [(right - left) * 0.5, (bottom - top) * 0.5];
        if half[0] < 0.0001 || half[1] < 0.0001 {
            self.line(from, to, radius as f32, color, 1.0);
            return;
        }
        let center = [(left + right) * 0.5, (top + bottom) * 0.5];
        let polygon = kind.polygon(left, top, right, bottom);
        let pad = if filled { 1.0 } else { radius + 1.0 };
        let [x0, y0, x1, y1] = self.clip_bounds([left - pad, top - pad, right + pad, bottom + pad]);
        for y in y0..y1 {
            for x in x0..x1 {
                let delta = [
                    (x as f64 + 0.5 - center[0]).abs(),
                    (y as f64 + 0.5 - center[1]).abs(),
                ];
                let distance = match kind {
                    ShapeKind::Rectangle => {
                        let q = [delta[0] - half[0], delta[1] - half[1]];
                        q[0].max(0.0).hypot(q[1].max(0.0)) + q[0].max(q[1]).min(0.0)
                    }
                    ShapeKind::Ellipse => {
                        // The implicit ellipse gives exact inside/outside tests;
                        // its gradient provides a local edge-distance estimate.
                        let normalized = (delta[0] / half[0]).hypot(delta[1] / half[1]);
                        if normalized < 0.0001 {
                            -half[0].min(half[1])
                        } else {
                            let gradient = (delta[0] / (half[0] * half[0]))
                                .hypot(delta[1] / (half[1] * half[1]))
                                / normalized;
                            (normalized - 1.0) / gradient
                        }
                    }
                    ShapeKind::RoundedRectangle => {
                        let corner = half[0].min(half[1]) * 0.3;
                        let q = [delta[0] - half[0] + corner, delta[1] - half[1] + corner];
                        q[0].max(0.0).hypot(q[1].max(0.0)) + q[0].max(q[1]).min(0.0) - corner
                    }
                    ShapeKind::Line => unreachable!(),
                    _ => shapes::polygon_distance([x as f64 + 0.5, y as f64 + 0.5], &polygon),
                };
                let coverage = if filled {
                    (0.5 - distance).clamp(0.0, 1.0)
                } else {
                    (radius + 0.5 - distance.abs()).clamp(0.0, 1.0)
                };
                if coverage > 0.0 {
                    let offset = (y as usize * self.width as usize + x as usize) * 4;
                    blend_pixel(&mut self.pixels[offset..offset + 4], color, coverage);
                }
            }
        }
    }

    pub fn clear(&mut self, color: [u8; 4]) {
        if self.validate().is_err() {
            return;
        }
        for pixel in self.pixels.as_chunks_mut::<4>().0 {
            pixel.copy_from_slice(&color);
        }
    }

    /// Resize from the top-left, preserving overlapping pixels. New area is
    /// transparent, and discarded pixels can be recovered through history.
    pub fn resize(&mut self, width: u32, height: u32) {
        if self.validate().is_err() {
            return;
        }
        let mut resized = Self::new(width, height, [0; 4]);
        let rows = self.height.min(resized.height) as usize;
        let row_bytes = self.width.min(resized.width) as usize * 4;
        for y in 0..rows {
            let from = y * self.width as usize * 4;
            let to = y * resized.width as usize * 4;
            resized.pixels[to..to + row_bytes]
                .copy_from_slice(&self.pixels[from..from + row_bytes]);
        }
        *self = resized;
    }

    fn clip_bounds(&self, bounds: [f64; 4]) -> [u32; 4] {
        [
            bounds[0].floor().clamp(0.0, self.width as f64) as u32,
            bounds[1].floor().clamp(0.0, self.height as f64) as u32,
            bounds[2].ceil().clamp(0.0, self.width as f64) as u32,
            bounds[3].ceil().clamp(0.0, self.height as f64) as u32,
        ]
    }
}

fn valid_points(from: [f32; 2], to: [f32; 2]) -> bool {
    from.into_iter().chain(to).all(f32::is_finite)
}

fn segment_distance(point: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let delta = [b[0] - a[0], b[1] - a[1]];
    let length = delta[0].hypot(delta[1]);
    if length <= f64::EPSILON {
        return (point[0] - a[0]).hypot(point[1] - a[1]);
    }
    let unit = [delta[0] / length, delta[1] / length];
    let along = (point[0] - a[0]) * unit[0] + (point[1] - a[1]) * unit[1];
    if along <= 0.0 {
        return (point[0] - a[0]).hypot(point[1] - a[1]);
    }
    if along >= length {
        return (point[0] - b[0]).hypot(point[1] - b[1]);
    }
    // Evaluate the line equation at the small canvas coordinate. Projecting
    // and subtracting enormous off-canvas endpoints would lose this precision.
    let line_constant = a[0] * unit[1] - a[1] * unit[0];
    (point[0] * unit[1] - point[1] * unit[0] - line_constant).abs()
}

fn blend_pixel(destination: &mut [u8], source: [u8; 4], coverage: f64) {
    let source_alpha = source[3] as f64 / 255.0 * coverage.clamp(0.0, 1.0);
    if source_alpha <= 0.0 {
        return;
    }
    let destination_alpha = destination[3] as f64 / 255.0;
    let retained_alpha = destination_alpha * (1.0 - source_alpha);
    let alpha = source_alpha + retained_alpha;
    for channel in 0..3 {
        destination[channel] = ((source[channel] as f64 * source_alpha
            + destination[channel] as f64 * retained_alpha)
            / alpha)
            .round() as u8;
    }
    destination[3] = (alpha * 255.0).round() as u8;
}
