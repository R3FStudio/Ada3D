//! The editable project model, independent of the application UI.

use crate::raster::Raster;
use crate::scene::{Primitive, SceneObject};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const PROJECT_VERSION: u32 = 1;
pub const MAX_OBJECTS: usize = 512;
pub const MAX_TITLE_BYTES: usize = 256;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Document {
    pub version: u32,
    pub title: String,
    pub canvas: Raster,
    pub objects: Vec<SceneObject>,
    pub next_object_id: u64,
}

impl Default for Document {
    fn default() -> Self {
        Self::new(960, 640)
    }
}

impl Document {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            version: PROJECT_VERSION,
            title: "Untitled".into(),
            canvas: Raster::new(width, height, [255; 4]),
            objects: Vec::new(),
            next_object_id: 1,
        }
    }

    /// Returns the new ID, or zero when the count or ID limit is exhausted.
    pub fn add_object(&mut self, primitive: Primitive, color: [u8; 4]) -> u64 {
        if self.objects.len() >= MAX_OBJECTS
            || self.next_object_id == 0
            || self.next_object_id == u64::MAX
            || self
                .objects
                .iter()
                .any(|object| object.id >= self.next_object_id)
        {
            return 0;
        }
        let id = self.next_object_id;
        self.objects.push(SceneObject::new(id, primitive, color));
        self.next_object_id += 1;
        id
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.version != PROJECT_VERSION {
            return Err(format!(
                "Unsupported project version {}. This app supports version {PROJECT_VERSION}.",
                self.version
            ));
        }
        if self.title.len() > MAX_TITLE_BYTES {
            return Err(format!(
                "Project titles must be at most {MAX_TITLE_BYTES} UTF-8 bytes."
            ));
        }
        self.canvas.validate()?;
        if self.objects.len() > MAX_OBJECTS {
            return Err(format!(
                "A project may contain at most {MAX_OBJECTS} 3D objects."
            ));
        }
        if self.next_object_id == 0 {
            return Err("The next object ID must be nonzero.".into());
        }
        if self
            .objects
            .iter()
            .filter_map(|o| o.paint.as_ref())
            .map(|p| p.pixels.len())
            .sum::<usize>()
            > crate::scene::MAX_PAINT_BYTES
        {
            return Err("This project has reached its surface paint limit.".into());
        }
        let mut ids = HashSet::with_capacity(self.objects.len());
        for object in &self.objects {
            object.validate()?;
            if object.id == 0 || !ids.insert(object.id) {
                return Err("Every 3D object must have a unique, nonzero ID.".into());
            }
            if object.id >= self.next_object_id {
                return Err(
                    "The next object ID must be greater than every existing object ID.".into(),
                );
            }
        }
        Ok(())
    }

    /// Heap data plus inline structure size for a cloned history snapshot.
    pub fn estimated_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.title.len()
            + self.canvas.pixels.len()
            + self.objects.len() * std::mem::size_of::<SceneObject>()
            + self
                .objects
                .iter()
                .map(|object| {
                    object.name.len() + object.paint.as_ref().map_or(0, |p| p.pixels.len())
                })
                .sum::<usize>()
    }
}
