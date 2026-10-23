//! Bounded floating raster selection; transforms always sample original pixels.
use crate::raster::Raster;
#[derive(Clone)]
pub struct Selection {
    pub background: Raster,
    pub pixels: Raster,
    pub center: [f32; 2],
    pub size: [f32; 2],
    pub angle: f32,
}
impl Selection {
    /// Resize from a corner/edge while the opposite anchor stays fixed.
    /// Handle components are -1, 0 or 1 in the selection's rotated axes.
    pub fn resize_from_handle(&self, handle: [f32; 2], delta: [f32; 2]) -> Self {
        let mut next = self.clone();
        if !delta.iter().all(|v| v.is_finite()) {
            return next;
        }
        let (sin, cos) = self.angle.sin_cos();
        let local = [
            delta[0] * cos + delta[1] * sin,
            -delta[0] * sin + delta[1] * cos,
        ];
        let mut shift = [0.0; 2];
        for axis in 0..2 {
            if handle[axis] != 0.0 {
                next.size[axis] = (self.size[axis] + handle[axis] * local[axis]).clamp(1.0, 4096.0);
                shift[axis] = handle[axis] * (next.size[axis] - self.size[axis]) * 0.5;
            }
        }
        next.center = [
            self.center[0] + shift[0] * cos - shift[1] * sin,
            self.center[1] + shift[0] * sin + shift[1] * cos,
        ];
        next
    }

    pub fn new(raster: &Raster, a: [f32; 2], b: [f32; 2]) -> Option<Self> {
        let x = a[0].min(b[0]).floor().clamp(0.0, raster.width as f32) as u32;
        let y = a[1].min(b[1]).floor().clamp(0.0, raster.height as f32) as u32;
        let right = a[0].max(b[0]).ceil().clamp(0.0, raster.width as f32) as u32;
        let bottom = a[1].max(b[1]).ceil().clamp(0.0, raster.height as f32) as u32;
        if right <= x || bottom <= y {
            return None;
        }
        let mut pixels = Raster::new(right - x, bottom - y, [0; 4]);
        let mut background = raster.clone();
        for py in 0..pixels.height {
            for px in 0..pixels.width {
                let color = raster.pixel(x + px, y + py);
                let dst = ((py * pixels.width + px) * 4) as usize;
                pixels.pixels[dst..dst + 4].copy_from_slice(&color);
                let dst = (((y + py) * raster.width + x + px) * 4) as usize;
                background.pixels[dst..dst + 4].fill(0);
            }
        }
        Some(Self {
            background,
            pixels,
            center: [(x + right) as f32 / 2.0, (y + bottom) as f32 / 2.0],
            size: [(right - x) as f32, (bottom - y) as f32],
            angle: 0.0,
        })
    }
    pub fn render(&self) -> Raster {
        let mut output = self.background.clone();
        if !self
            .center
            .iter()
            .chain(self.size.iter())
            .all(|v| v.is_finite())
            || !self.angle.is_finite()
        {
            return output;
        }
        let w = self.size[0].clamp(1.0, 4096.0);
        let h = self.size[1].clamp(1.0, 4096.0);
        let (sin, cos) = self.angle.sin_cos();
        let radius = (w * w + h * h).sqrt() / 2.0;
        let left = (self.center[0] - radius)
            .floor()
            .clamp(0.0, output.width as f32) as u32;
        let right = (self.center[0] + radius)
            .ceil()
            .clamp(0.0, output.width as f32) as u32;
        let top = (self.center[1] - radius)
            .floor()
            .clamp(0.0, output.height as f32) as u32;
        let bottom = (self.center[1] + radius)
            .ceil()
            .clamp(0.0, output.height as f32) as u32;
        for y in top..bottom {
            for x in left..right {
                let dx = x as f32 + 0.5 - self.center[0];
                let dy = y as f32 + 0.5 - self.center[1];
                let u = (dx * cos + dy * sin) / w + 0.5;
                let v = (-dx * sin + dy * cos) / h + 0.5;
                if !(0.0..1.0).contains(&u) || !(0.0..1.0).contains(&v) {
                    continue;
                }
                let src = self.pixels.pixel(
                    (u * self.pixels.width as f32) as u32,
                    (v * self.pixels.height as f32) as u32,
                );
                let dst = output.pixel(x, y);
                let a = src[3] as f32 / 255.0;
                let b = dst[3] as f32 / 255.0;
                let alpha = a + b * (1.0 - a);
                let offset = ((y * output.width + x) * 4) as usize;
                for c in 0..3 {
                    output.pixels[offset + c] = if alpha > 0.0 {
                        ((src[c] as f32 * a + dst[c] as f32 * b * (1.0 - a)) / alpha).round() as u8
                    } else {
                        0
                    };
                }
                output.pixels[offset + 3] = (alpha * 255.0).round() as u8;
            }
        }
        output
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_resize_handles_preserve_the_opposite_rotated_anchor() {
        let r = Raster::new(80, 80, [255; 4]);
        let mut base = Selection::new(&r, [20.0, 25.0], [50.0, 45.0]).unwrap();
        for angle in [0.0, 0.6, std::f32::consts::FRAC_PI_2] {
            base.angle = angle;
            let (sin, cos) = angle.sin_cos();
            let anchor = |s: &Selection, h: [f32; 2]| {
                let x = -h[0] * s.size[0] * 0.5;
                let y = -h[1] * s.size[1] * 0.5;
                [
                    s.center[0] + x * cos - y * sin,
                    s.center[1] + x * sin + y * cos,
                ]
            };
            for handle in [
                [-1.0, -1.0],
                [1.0, -1.0],
                [1.0, 1.0],
                [-1.0, 1.0],
                [0.0, -1.0],
                [1.0, 0.0],
                [0.0, 1.0],
                [-1.0, 0.0],
            ] {
                let next = base.resize_from_handle(handle, [12.0, -7.0]);
                let a = anchor(&base, handle);
                let b = anchor(&next, handle);
                assert!((a[0] - b[0]).abs() < 0.0001 && (a[1] - b[1]).abs() < 0.0001);
                for (axis, component) in handle.into_iter().enumerate() {
                    if component == 0.0 {
                        assert_eq!(next.size[axis], base.size[axis]);
                    }
                }
            }
        }
        let next = base.resize_from_handle([1.0, 1.0], [1e8, -1e8]);
        assert!(next.size.iter().all(|v| (1.0..=4096.0).contains(v)));
        let next = base.resize_from_handle([1.0, 1.0], [f32::NAN, 0.0]);
        assert_eq!(next.size, base.size);
        assert_eq!(next.center, base.center);
    }
    #[test]
    fn identity_keeps_rgba_exact() {
        let r = Raster::new(12, 10, [30, 80, 150, 128]);
        let s = Selection::new(&r, [2.0, 3.0], [8.0, 9.0]).unwrap();
        assert_eq!(s.render(), r);
    }
    #[test]
    fn translated_selection_clears_source_and_preserves_alpha() {
        let r = Raster::new(12, 10, [30, 80, 150, 128]);
        let mut s = Selection::new(&r, [0.0, 0.0], [2.0, 2.0]).unwrap();
        s.center = [8.0, 8.0];
        let result = s.render();
        assert_eq!(result.pixel(0, 0), [0; 4]);
        assert_eq!(result.pixel(8, 8), [30, 80, 150, 192]);
    }
    #[test]
    fn reverse_and_outside_drag_is_bounded() {
        let r = Raster::new(12, 10, [255; 4]);
        let mut s = Selection::new(&r, [90.0, 90.0], [-50.0, -50.0]).unwrap();
        assert_eq!(s.pixels, r);
        s.size = [f32::INFINITY, 2.0];
        assert_eq!(s.render(), s.background);
        assert!(Selection::new(&r, [1.0, 1.0], [1.0, 1.0]).is_none());
    }
    #[test]
    fn rotation_and_scale_keep_output_dimensions() {
        let r = Raster::new(12, 10, [255; 4]);
        let mut s = Selection::new(&r, [1.0, 1.0], [5.0, 6.0]).unwrap();
        s.angle = std::f32::consts::FRAC_PI_2;
        s.size = [8.0, 10.0];
        let result = s.render();
        assert_eq!((result.width, result.height), (12, 10));
        assert!(result.validate().is_ok());
    }
}
