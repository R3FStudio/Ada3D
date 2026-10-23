//! Persistent object-space surface paint and bounded texture coordinates.
use super::*;
pub const PAINT_SIZE: u32 = 256;
pub const MAX_PAINT_BYTES: usize = 8 * 1024 * 1024;

impl SceneObject {
    pub fn surface_color(&self, uv: [f32; 2]) -> [u8; 4] {
        self.paint.as_ref().map_or(self.color, |paint| {
            let [x, y] = texel(uv);
            paint.pixel(x, y)
        })
    }
    pub fn prepare_paint(&mut self) {
        if self.paint.is_none() {
            self.paint = Some(Raster::new(PAINT_SIZE, PAINT_SIZE, self.color));
        }
    }
}
pub fn texel(uv: [f32; 2]) -> [u32; 2] {
    uv.map(|v| {
        (v.clamp(0.0, 1.0) * PAINT_SIZE as f32)
            .floor()
            .min((PAINT_SIZE - 1) as f32) as u32
    })
}

pub(super) fn surface_uv(primitive: Primitive, p: V3) -> [f32; 2] {
    if primitive == Primitive::Torus {
        let radial = p.x.hypot(p.z) - 0.52;
        return [
            ((p.z.atan2(p.x) / std::f64::consts::TAU + 1.0) % 1.0) as f32,
            ((p.y.atan2(radial) / std::f64::consts::TAU + 1.0) % 1.0) as f32,
        ];
    }
    // Six separate regions keep opposite faces independent. Coordinates depend
    // on object space, so paint follows every subsequent object transform.
    let a = [p.x.abs(), p.y.abs(), p.z.abs()];
    let (face, u, v, size) = if a[0] >= a[1] && a[0] >= a[2] {
        (if p.x >= 0.0 { 0 } else { 1 }, p.z, -p.y, a[0])
    } else if a[1] >= a[2] {
        (if p.y >= 0.0 { 2 } else { 3 }, p.x, p.z, a[1])
    } else {
        (if p.z >= 0.0 { 4 } else { 5 }, p.x, -p.y, a[2])
    };
    let size = size.max(1e-12);
    let u = ((u / size + 1.0) * 0.5).clamp(0.006, 0.994);
    let v = ((v / size + 1.0) * 0.5).clamp(0.004, 0.996);
    [
        ((face % 3) as f64 + u) as f32 / 3.0,
        ((face / 3) as f64 + v) as f32 / 2.0,
    ]
}
