//! Behavioral tests for independently implemented editing and persistence.

use adars::document::{Document, MAX_OBJECTS};
use adars::history::{History, MAX_HISTORY_BYTES, MAX_HISTORY_STATES};
use adars::persistence::{
    MAX_PROJECT_FILE_BYTES, export_png, import_image, load_project, save_project,
};
use adars::raster::{MAX_DIMENSION, Raster, ShapeKind};
use adars::scene::Primitive;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct TestFile(PathBuf);

impl TestFile {
    fn new(extension: &str) -> Self {
        let sequence = TEST_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        Self(std::env::temp_dir().join(format!(
            "adars-test-{}-{nanos}-{sequence}.{extension}",
            std::process::id()
        )))
    }
    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

#[test]
fn fill_stays_in_a_four_connected_region() {
    let mut raster = Raster::new(9, 7, [255; 4]);
    // A one-pixel barrier reaching both canvas edges.
    raster.line([4.5, -10.0], [4.5, 20.0], 0.5, [0, 0, 0, 255], 1.0);
    assert!(raster.fill(1, 3, [220, 30, 80, 255]));
    for y in 0..7 {
        assert_eq!(raster.pixel(0, y), [220, 30, 80, 255]);
        assert_eq!(raster.pixel(3, y), [220, 30, 80, 255]);
        assert_eq!(raster.pixel(4, y), [0, 0, 0, 255]);
        assert_eq!(raster.pixel(5, y), [255; 4]);
        assert_eq!(raster.pixel(8, y), [255; 4]);
    }
    assert!(!raster.fill(1, 3, [220, 30, 80, 255]));
    assert!(!raster.fill(u32::MAX, 0, [0; 4]));

    let mut diagonal = Raster::new(2, 2, [0, 0, 0, 255]);
    diagonal.pixels[0..4].copy_from_slice(&[255; 4]);
    diagonal.pixels[12..16].copy_from_slice(&[255; 4]);
    diagonal.fill(0, 0, [100, 50, 25, 255]);
    assert_eq!(diagonal.pixel(1, 1), [255; 4]);
}

#[test]
fn semi_transparent_paint_uses_straight_alpha_and_erasure_removes_it() {
    let mut raster = Raster::new(3, 3, [0; 4]);
    raster.stamp(1.5, 1.5, 0.5, [255, 0, 0, 128], 1.0);
    assert_eq!(raster.pixel(1, 1), [255, 0, 0, 128]);
    raster.stamp(1.5, 1.5, 0.5, [0, 0, 255, 128], 1.0);
    assert_eq!(raster.pixel(1, 1), [85, 0, 170, 192]);
    let before = raster.clone();
    raster.stamp(1.5, 1.5, 0.5, [50, 220, 20, 0], 1.0);
    assert_eq!(raster, before);
    raster.erase_line([1.5, 1.5], [1.5, 1.5], 0.5);
    assert_eq!(raster.pixel(1, 1), [0; 4]);

    let mut white = Raster::new(1, 1, [255; 4]);
    white.stamp(0.5, 0.5, 0.5, [255, 0, 0, 255], 0.5);
    assert_eq!(white.pixel(0, 0), [255, 128, 128, 255]);
}

#[test]
fn strokes_are_continuous_and_finite_inputs_are_bounded() {
    let mut raster = Raster::new(16, 3, [0; 4]);
    raster.line([-20.0, 1.5], [50.0, 1.5], 0.5, [20, 60, 200, 255], 1.0);
    for x in 0..16 {
        assert_eq!(raster.pixel(x, 1), [20, 60, 200, 255]);
    }
    assert_eq!(raster.pixel(3, 0), [0; 4]);
    let before = raster.clone();
    raster.line([f32::NAN, 0.0], [1.0, 1.0], 1.0, [255; 4], 1.0);
    raster.line([0.0, 0.0], [1.0, 1.0], f32::INFINITY, [255; 4], 1.0);
    raster.stamp(1.5, 1.5, 1.0, [255; 4], f32::NAN);
    raster.erase_stamp(f32::INFINITY, 1.5, 1.0);
    assert_eq!(raster, before);
    raster.line(
        [f32::MAX, f32::MAX],
        [f32::MAX, f32::MAX],
        f32::MAX,
        [255; 4],
        1.0,
    );
    assert_eq!(raster, before);
    let mut spanning = Raster::new(16, 3, [0; 4]);
    spanning.line([-f32::MAX, 1.5], [f32::MAX, 1.5], 0.5, [255; 4], 1.0);
    for x in 0..16 {
        assert_eq!(spanning.pixel(x, 1), [255; 4]);
    }
    assert!(raster.validate().is_ok());
    assert_eq!(raster.pixel(u32::MAX, u32::MAX), [0; 4]);
}

#[test]
fn shapes_accept_reverse_drags_and_distinguish_filled_from_outline() {
    let mut outline = Raster::new(11, 11, [0; 4]);
    outline.draw_shape(
        ShapeKind::Rectangle,
        [9.5, 9.5],
        [1.5, 1.5],
        [30, 80, 200, 255],
        1.0,
        false,
    );
    assert_eq!(outline.pixel(5, 5), [0; 4]);
    assert_eq!(outline.pixel(1, 5), [30, 80, 200, 255]);
    outline.draw_shape(
        ShapeKind::Ellipse,
        [2.0, 2.0],
        [9.0, 9.0],
        [200, 50, 20, 255],
        1.0,
        true,
    );
    assert_eq!(outline.pixel(5, 5), [200, 50, 20, 255]);
    assert_eq!(outline.pixel(0, 0), [0; 4]);
    let mut degenerate = Raster::new(5, 5, [0; 4]);
    degenerate.draw_shape(
        ShapeKind::Rectangle,
        [2.5, 0.5],
        [2.5, 4.5],
        [255; 4],
        1.0,
        false,
    );
    assert_eq!(degenerate.pixel(2, 2), [255; 4]);
}

#[test]
fn resize_preserves_overlap_and_expands_with_transparent_pixels() {
    let mut raster = Raster::new(2, 2, [255; 4]);
    raster.stamp(1.5, 1.5, 0.5, [12, 34, 56, 255], 1.0);
    raster.resize(4, 3);
    assert_eq!(raster.pixel(1, 1), [12, 34, 56, 255]);
    assert_eq!(raster.pixel(0, 0), [255; 4]);
    assert_eq!(raster.pixel(3, 2), [0; 4]);
    raster.resize(1, 1);
    assert_eq!(raster.pixels, vec![255; 4]);
    assert!(raster.validate().is_ok());
    assert_eq!(Raster::new(0, 0, [0; 4]).width, 1);
    let bounded = Raster::new(u32::MAX, 1, [0; 4]);
    assert_eq!(bounded.width, MAX_DIMENSION);
}

#[test]
fn malformed_raster_operations_do_not_index_invalid_buffers() {
    let mut malformed = Raster {
        width: 100,
        height: 100,
        pixels: vec![0; 4],
    };
    assert!(malformed.validate().is_err());
    let before = malformed.clone();
    malformed.stamp(50.0, 50.0, 10.0, [255; 4], 1.0);
    malformed.erase_stamp(50.0, 50.0, 10.0);
    malformed.draw_shape(
        ShapeKind::Ellipse,
        [0.0, 0.0],
        [50.0, 50.0],
        [255; 4],
        1.0,
        true,
    );
    malformed.clear([255; 4]);
    malformed.resize(1, 1);
    assert!(!malformed.fill(0, 0, [255; 4]));
    assert_eq!(malformed, before);
    assert_eq!(malformed.pixel(50, 50), [0; 4]);
}

#[test]
fn a_gesture_is_one_undo_step_across_paint_and_scene_changes() {
    let mut document = Document::new(16, 16);
    let initial = document.clone();
    let mut history = History::new();
    history.begin(&document);
    document
        .canvas
        .line([1.5, 1.5], [12.5, 12.5], 1.0, [200, 50, 60, 255], 1.0);
    document
        .canvas
        .line([12.5, 12.5], [12.5, 2.5], 1.0, [200, 50, 60, 255], 1.0);
    let id = document.add_object(Primitive::Cube, [100, 160, 230, 255]);
    assert_eq!(id, 1);
    assert!(history.commit(&document));
    let edited = document.clone();
    assert!(history.can_undo());
    assert!(history.undo(&mut document));
    assert_eq!(document, initial);
    assert!(history.can_redo());
    assert!(history.redo(&mut document));
    assert_eq!(document, edited);
    assert!(!history.redo(&mut document));
}

#[test]
fn empty_transactions_preserve_redo_and_new_edits_create_a_branch() {
    let mut document = Document::new(3, 3);
    let mut history = History::new();
    history.begin(&document);
    document.canvas.fill(1, 1, [50, 60, 70, 255]);
    assert!(history.commit(&document));
    assert!(history.undo(&mut document));
    history.begin(&document);
    assert!(!history.commit(&document));
    assert!(history.can_redo());
    history.begin(&document);
    document.add_object(Primitive::Sphere, [100, 200, 50, 255]);
    assert!(history.commit(&document));
    assert!(!history.can_redo());
    assert!(history.undo(&mut document));
    assert!(document.objects.is_empty());
    history.clear();
    assert!(!history.can_undo());
    assert!(!history.can_redo());
}

#[test]
fn history_respects_count_and_memory_caps_including_pending_transactions() {
    let mut small = Document::new(1, 1);
    let mut history = History::new();
    for index in 0..40 {
        history.begin(&small);
        small.title = format!("State {index}");
        history.commit(&small);
        assert!(history.state_count() <= MAX_HISTORY_STATES);
    }
    assert_eq!(history.state_count(), MAX_HISTORY_STATES);
    history.clear();
    let mut large = Document::new(MAX_DIMENSION, MAX_DIMENSION);
    for index in 0..6 {
        history.begin(&large);
        assert!(history.estimated_memory_bytes() <= MAX_HISTORY_BYTES);
        large.title = format!("Large state {index}");
        history.commit(&large);
        assert!(history.estimated_memory_bytes() <= MAX_HISTORY_BYTES);
    }
    assert!(history.state_count() < 6);
    assert!(history.undo(&mut large));
    assert!(history.estimated_memory_bytes() <= MAX_HISTORY_BYTES);
    assert!(history.redo(&mut large));
    assert!(history.estimated_memory_bytes() <= MAX_HISTORY_BYTES);
}

#[test]
fn project_roundtrip_preserves_raster_scene_and_stable_ids() {
    let target = TestFile::new("ada3dproject");
    let mut document = Document::new(32, 24);
    document.title = "Original study".into();
    document.canvas.erase_stamp(5.5, 5.5, 2.0);
    document.canvas.draw_shape(
        ShapeKind::Ellipse,
        [10.0, 5.0],
        [25.0, 20.0],
        [50, 100, 200, 180],
        2.0,
        true,
    );
    document.add_object(Primitive::Cylinder, [80, 200, 100, 255]);
    document.add_object(Primitive::Cone, [220, 180, 40, 255]);
    save_project(target.path(), &document).unwrap();
    assert_eq!(load_project(target.path()).unwrap(), document);
    document.title = "Updated study".into();
    // Saving a second time replaces the existing destination.
    save_project(target.path(), &document).unwrap();
    assert_eq!(load_project(target.path()).unwrap(), document);
}

#[test]
fn project_io_rejects_other_extensions_without_touching_files() {
    let document = Document::new(2, 2);
    for extension in ["adars", "forma", "json", "png"] {
        let target = TestFile::new(extension);
        let bytes = serde_json::to_vec(&document).unwrap();
        fs::write(target.path(), &bytes).unwrap();
        assert!(load_project(target.path()).is_err());
        assert!(save_project(target.path(), &document).is_err());
        assert_eq!(fs::read(target.path()).unwrap(), bytes);
    }
}

#[test]
fn invalid_project_save_preserves_the_existing_file() {
    let target = TestFile::new("ada3dproject");
    let mut document = Document::new(2, 2);
    save_project(target.path(), &document).unwrap();
    let before = fs::read(target.path()).unwrap();
    document.canvas.pixels.pop();
    assert!(save_project(target.path(), &document).is_err());
    assert_eq!(fs::read(target.path()).unwrap(), before);
}

#[test]
fn invalid_version_dimensions_and_ids_are_rejected_on_load() {
    let target = TestFile::new("ada3dproject");
    let base = Document::new(2, 2);
    let mut invalid = base.clone();
    invalid.version += 1;
    fs::write(target.path(), serde_json::to_vec(&invalid).unwrap()).unwrap();
    assert!(load_project(target.path()).is_err());
    invalid = base.clone();
    invalid.canvas.width = MAX_DIMENSION + 1;
    fs::write(target.path(), serde_json::to_vec(&invalid).unwrap()).unwrap();
    assert!(load_project(target.path()).is_err());
    invalid = base;
    invalid.add_object(Primitive::Cube, [255; 4]);
    invalid.objects.push(invalid.objects[0].clone());
    fs::write(target.path(), serde_json::to_vec(&invalid).unwrap()).unwrap();
    assert!(load_project(target.path()).is_err());
    invalid.objects.pop();
    invalid.next_object_id = 1;
    assert!(invalid.validate().is_err());
}

#[test]
fn project_file_and_object_limits_are_enforced() {
    let target = TestFile::new("ada3dproject");
    let file = fs::File::create(target.path()).unwrap();
    file.set_len(MAX_PROJECT_FILE_BYTES + 1).unwrap();
    drop(file);
    assert!(load_project(target.path()).is_err());

    let mut document = Document::new(1, 1);
    for _ in 0..MAX_OBJECTS {
        assert_ne!(document.add_object(Primitive::Cube, [255; 4]), 0);
    }
    assert_eq!(document.add_object(Primitive::Cube, [255; 4]), 0);
    assert!(document.validate().is_ok());
    document.objects.push(document.objects[0].clone());
    assert!(document.validate().is_err());
}

#[test]
fn png_roundtrip_preserves_rgba_and_invalid_export_preserves_destination() {
    let target = TestFile::new("png");
    let mut raster = Raster::new(8, 8, [0; 4]);
    raster.stamp(3.5, 3.5, 2.0, [220, 80, 30, 170], 1.0);
    export_png(target.path(), &raster).unwrap();
    assert_eq!(import_image(target.path()).unwrap(), raster);
    let before = fs::read(target.path()).unwrap();
    raster.pixels.pop();
    assert!(export_png(target.path(), &raster).is_err());
    assert_eq!(fs::read(target.path()).unwrap(), before);
}

#[test]
fn image_dimensions_are_checked_before_pixel_decoding() {
    let target = TestFile::new("png");
    // A genuine, very wide image exercises header limits independently of
    // project dimensions. Only one row is allocated to create this fixture.
    let pixels = vec![0u8; (MAX_DIMENSION as usize + 1) * 4];
    image::save_buffer_with_format(
        target.path(),
        &pixels,
        MAX_DIMENSION + 1,
        1,
        image::ColorType::Rgba8,
        image::ImageFormat::Png,
    )
    .unwrap();
    assert!(import_image(target.path()).is_err());
    fs::write(target.path(), b"This is not an image").unwrap();
    assert!(import_image(target.path()).is_err());
}

#[test]
fn transparent_brush_and_bucket_survive_png_export() {
    let mut raster = Raster::new(24, 16, [200, 80, 40, 255]);
    raster.erase_line_with_opacity([4.0, 8.0], [10.0, 8.0], 2.0, 1.0);
    assert_eq!(raster.pixel(7, 8), [0; 4]);
    assert_eq!(raster.pixel(20, 8), [200, 80, 40, 255]);
    assert!(raster.fill_transparency(20, 8, 0.5));
    assert_eq!(raster.pixel(20, 8), [200, 80, 40, 128]);
    let file = TestFile::new("png");
    export_png(file.path(), &raster).unwrap();
    let loaded = import_image(file.path()).unwrap();
    assert_eq!(loaded, raster);
}
#[test]
fn transparent_bucket_stays_inside_exact_connected_region() {
    let mut raster = Raster::new(7, 5, [80, 100, 120, 255]);
    for y in 0..5 {
        let i = (y * 7 + 3) * 4;
        raster.pixels[i..i + 4].copy_from_slice(&[10, 20, 30, 255]);
    }
    assert!(raster.fill_transparency(1, 2, 1.0));
    assert_eq!(raster.pixel(1, 2), [0; 4]);
    assert_eq!(raster.pixel(5, 2), [80, 100, 120, 255]);
    assert_eq!(raster.pixel(3, 2), [10, 20, 30, 255]);
    let before = raster.clone();
    assert!(!raster.fill_transparency(5, 2, f32::NAN));
    assert!(!raster.fill_transparency(7, 2, 1.0));
    assert!(!raster.fill_transparency(5, 2, 0.0));
    assert_eq!(raster, before);
}
