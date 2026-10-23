//! Ada3D document model and editing algorithms.
pub mod core;
// Keep the public module paths stable for integrations.
pub use core::{
    document, export, history, persistence, progress, raster, render_worker, scene, selection,
    settings,
};

pub mod updater;
