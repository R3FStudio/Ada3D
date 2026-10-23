//! Primitive geometry and a CPU perspective renderer.
//!
//! Camera orbit and object Euler rotations are in degrees. Geometry uses a
//! right-handed world with Y up, with a 1.5-unit diameter for each primitive.

use crate::raster::Raster;
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;
use std::path::Path;

const RADIUS: f64 = 0.75;
const NEAR: f64 = 0.1;
const MAX_RENDER_SIDE: u32 = 2048;
pub const MAX_OBJECT_NAME_BYTES: usize = 256;
const SELECTION_COLOR: [u8; 4] = [51, 219, 202, 255];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Primitive {
    Cube,
    Sphere,
    Cylinder,
    Cone,
    Pyramid,
    Torus,
    Capsule,
    Prism,
}

impl Primitive {
    pub const ALL: [Self; 8] = [
        Self::Cube,
        Self::Sphere,
        Self::Cylinder,
        Self::Cone,
        Self::Pyramid,
        Self::Torus,
        Self::Capsule,
        Self::Prism,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Cube => "Cube",
            Self::Sphere => "Sphere",
            Self::Cylinder => "Cylinder",
            Self::Cone => "Cone",
            Self::Pyramid => "Pyramid",
            Self::Torus => "Ring",
            Self::Capsule => "Capsule",
            Self::Prism => "Prism",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Transform {
    pub position: [f32; 3],
    /// Euler angles in degrees, applied in X, Y, Z order.
    pub rotation: [f32; 3],
    pub scale: [f32; 3],
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: [0.0; 3],
            rotation: [0.0; 3],
            scale: [1.0; 3],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneObject {
    pub id: u64,
    pub name: String,
    pub primitive: Primitive,
    pub transform: Transform,
    pub color: [u8; 4],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paint: Option<Raster>,
}

impl SceneObject {
    pub fn new(id: u64, primitive: Primitive, color: [u8; 4]) -> Self {
        Self {
            id,
            name: format!("{} {}", primitive.label(), id),
            primitive,
            transform: Transform::default(),
            color,
            paint: None,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if let Some(paint) = &self.paint {
            paint.validate()?;
            if paint.width != PAINT_SIZE || paint.height != PAINT_SIZE {
                return Err("Object paint dimensions are invalid.".into());
            }
        }
        if self.name.len() > MAX_OBJECT_NAME_BYTES {
            return Err(format!(
                "Object names must be at most {MAX_OBJECT_NAME_BYTES} UTF-8 bytes"
            ));
        }
        if !self.transform.position.iter().all(|n| n.is_finite()) {
            return Err(format!("{} has a non-finite position", self.name));
        }
        if !self.transform.rotation.iter().all(|n| n.is_finite()) {
            return Err(format!("{} has a non-finite rotation", self.name));
        }
        if !self
            .transform
            .scale
            .iter()
            .all(|n| n.is_finite() && *n > 0.0)
        {
            return Err(format!(
                "{} must have finite, positive scale values",
                self.name
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Camera {
    /// Horizontal orbit angle in degrees.
    pub yaw: f32,
    /// Vertical orbit angle in degrees.
    pub pitch: f32,
    pub distance: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            yaw: 32.0,
            pitch: 22.0,
            distance: 7.0,
        }
    }
}

impl Camera {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

pub struct RenderedScene {
    pub raster: Raster,
    /// Visible object ID at each raster pixel, in row-major order.
    /// Selection outlines outside the surface do not claim a picking ID.
    pub object_ids: Vec<Option<u64>>,
    /// Surface coordinates for the corresponding visible mesh fragment.
    pub surface_uvs: Vec<[f32; 2]>,
}

#[derive(Clone, Copy, Debug)]
struct V3 {
    x: f64,
    y: f64,
    z: f64,
}

impl V3 {
    fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    fn add(self, b: Self) -> Self {
        Self::new(self.x + b.x, self.y + b.y, self.z + b.z)
    }

    fn sub(self, b: Self) -> Self {
        Self::new(self.x - b.x, self.y - b.y, self.z - b.z)
    }

    fn mul(self, n: f64) -> Self {
        Self::new(self.x * n, self.y * n, self.z * n)
    }

    fn dot(self, b: Self) -> f64 {
        self.x * b.x + self.y * b.y + self.z * b.z
    }

    fn cross(self, b: Self) -> Self {
        Self::new(
            self.y * b.z - self.z * b.y,
            self.z * b.x - self.x * b.z,
            self.x * b.y - self.y * b.x,
        )
    }

    fn length(self) -> f64 {
        self.dot(self).sqrt()
    }

    fn normalized(self) -> Self {
        let len = self.length();
        if len > 1.0e-12 && len.is_finite() {
            self.mul(1.0 / len)
        } else {
            Self::new(0.0, 1.0, 0.0)
        }
    }

    fn finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }
}

struct Mesh {
    vertices: Vec<V3>,
    triangles: Vec<[usize; 3]>,
}

mod paint;
pub use paint::{MAX_PAINT_BYTES, PAINT_SIZE, texel as paint_texel};
mod export;
mod geometry;
mod renderer;
mod shapes;
pub use export::{export_obj, export_obj_with_progress};
use geometry::{primitive_mesh, transform_vertex};
#[cfg(test)]
use renderer::clip_near;
pub use renderer::{RenderInput, placement, render, render_with_progress};
#[cfg(test)]
mod tests;
