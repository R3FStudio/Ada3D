use adars::{
    document::Document,
    export::{self, Format, Options, Sampling, Source},
    persistence,
    progress::Progress,
    scene::{Camera, Primitive},
};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static SEQUENCE: AtomicU64 = AtomicU64::new(0);
struct Files(PathBuf);
impl Files {
    fn new(ext: &str) -> Self {
        Self(std::env::temp_dir().join(format!(
            "ada-export-{}-{}.{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed),
            ext
        )))
    }
}
impl Drop for Files {
    fn drop(&mut self) {
        for p in [
            &self.0,
            &self.0.with_extension("rc"),
            &self.0.with_extension("icon.rs"),
        ] {
            let _ = fs::remove_file(p);
        }
    }
}

#[test]
fn project_load_counters_match_real_read_and_decoded_bytes() {
    let files = Files::new("ada3dproject");
    let doc = Document::new(128, 128);
    persistence::save_project(&files.0, &doc).unwrap();
    let length = fs::metadata(&files.0).unwrap().len();
    let mut counters = Vec::new();
    let loaded = persistence::load_project_with_progress(&files.0, &mut |p| {
        counters.push(p);
        Ok(())
    })
    .unwrap();
    assert_eq!(loaded, doc);
    for stage in ["Reading project", "Decoding artwork"] {
        let values: Vec<_> = counters.iter().filter(|p| p.stage == stage).collect();
        assert!(values.len() > 2);
        assert_eq!(values.last().unwrap().completed, length);
        assert!(values.iter().all(|p| p.total == length));
        assert!(values.windows(2).all(|v| v[0].completed <= v[1].completed));
    }
    assert_eq!(counters.last().unwrap().stage, "Checking artwork");
    assert_eq!(counters.last().unwrap().fraction(), Some(1.0));
}

#[test]
fn ico_embeds_every_size_with_alpha_and_build_helpers() {
    let files = Files::new("ico");
    let mut doc = Document::new(64, 32);
    doc.canvas
        .pixels
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .for_each(|p| p.copy_from_slice(&[220, 40, 100, 255]));
    let mut options = Options::for_document(&doc);
    options.format = Format::Ico;
    options.resource_file = true;
    options.rust_file = true;
    let mut progress = Vec::new();
    let paths = export::run(&files.0, &doc, &Camera::default(), &options, &mut |p| {
        progress.push(p);
        Ok(())
    })
    .unwrap();
    assert_eq!(paths.len(), 3);
    let bytes = fs::read(&files.0).unwrap();
    assert_eq!(&bytes[..6], &[0, 0, 1, 0, 7, 0]);
    for (index, size) in [16, 24, 32, 48, 64, 128, 256].into_iter().enumerate() {
        let entry = &bytes[6 + index * 16..6 + (index + 1) * 16];
        assert_eq!(
            if entry[0] == 0 {
                256
            } else {
                u32::from(entry[0])
            },
            size
        );
        let length = u32::from_le_bytes(entry[8..12].try_into().unwrap()) as usize;
        let offset = u32::from_le_bytes(entry[12..16].try_into().unwrap()) as usize;
        let image = image::load_from_memory_with_format(
            &bytes[offset..offset + length],
            image::ImageFormat::Png,
        )
        .unwrap()
        .into_rgba8();
        assert_eq!(image.dimensions(), (size, size));
        assert_eq!(image.get_pixel(size / 2, 0)[3], 0);
        assert_eq!(image.get_pixel(size / 2, size / 2).0, [220, 40, 100, 255]);
    }
    let decoded = image::load_from_memory_with_format(&bytes, image::ImageFormat::Ico).unwrap();
    assert_eq!(decoded.width(), 256);
    let rc = fs::read(files.0.with_extension("rc")).unwrap();
    assert_eq!(&rc[..2], &[255, 254]);
    let units: Vec<u16> = rc[2..]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .collect();
    let rc = String::from_utf16(&units).unwrap();
    assert!(rc.contains("101 ICON"));
    assert!(rc.contains(files.0.file_name().unwrap().to_str().unwrap()));
    assert!(
        fs::read_to_string(files.0.with_extension("icon.rs"))
            .unwrap()
            .contains("include_bytes!")
    );
    let icons: Vec<_> = progress
        .iter()
        .filter(|p| p.stage == "Generating icon sizes")
        .collect();
    assert_eq!(icons.last().unwrap().completed, 7);
    assert_eq!(icons.last().unwrap().total, 7);
    for path in paths {
        let expected = fs::metadata(&path).unwrap().len();
        assert!(progress.iter().any(|p| p.stage == "Writing file"
            && p.item.as_deref() == Some(path.to_str().unwrap())
            && p.completed == expected
            && p.total == expected));
    }
}

#[test]
fn nearest_resize_keeps_pixel_values_and_smooth_resize_avoids_alpha_fringes() {
    let files = Files::new("png");
    let mut doc = Document::new(2, 1);
    doc.canvas.pixels = vec![255, 0, 0, 255, 0, 0, 255, 0];
    let mut options = Options::for_document(&doc);
    options.width = 32;
    options.height = 16;
    export::run(
        &files.0,
        &doc,
        &Camera::default(),
        &options,
        &mut |_| Ok(()),
    )
    .unwrap();
    let image = persistence::import_image(&files.0).unwrap();
    assert!(
        image
            .pixels
            .as_chunks::<4>()
            .0
            .iter()
            .all(|p| *p == [255, 0, 0, 255] || *p == [0, 0, 255, 0])
    );
    options.sampling = Sampling::Triangle;
    export::run(
        &files.0,
        &doc,
        &Camera::default(),
        &options,
        &mut |_| Ok(()),
    )
    .unwrap();
    let image = persistence::import_image(&files.0).unwrap();
    assert!(
        image
            .pixels
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|p| p[3] > 0)
            .all(|p| p[0] == 255 && p[1] == 0 && p[2] == 0)
    );
}

#[test]
fn invalid_options_and_cancelled_write_preserve_the_destination() {
    let files = Files::new("png");
    fs::write(&files.0, b"existing artwork").unwrap();
    let mut doc = Document::new(256, 256);
    for (i, p) in doc
        .canvas
        .pixels
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .enumerate()
    {
        let value = (i as u32).wrapping_mul(2654435761).to_le_bytes();
        p.copy_from_slice(&value);
    }
    let mut options = Options::for_document(&doc);
    options.width = 2049;
    assert!(
        export::run(
            &files.0,
            &doc,
            &Camera::default(),
            &options,
            &mut |_| Ok(())
        )
        .is_err()
    );
    options.width = 256;
    let result = export::run(&files.0, &doc, &Camera::default(), &options, &mut |p| {
        if p.stage == "Writing file" && p.completed > 0 {
            Err("Cancelled".into())
        } else {
            Ok(())
        }
    });
    assert!(result.is_err());
    assert_eq!(fs::read(&files.0).unwrap(), b"existing artwork");
}

#[test]
fn scene_export_counts_rendered_triangles_and_composited_rows() {
    let files = Files::new("png");
    let mut doc = Document::new(64, 48);
    doc.add_object(Primitive::Sphere, [240, 80, 40, 255]);
    let mut options = Options::for_document(&doc);
    options.source = Source::Scene;
    let mut progress: Vec<Progress> = Vec::new();
    export::run(&files.0, &doc, &Camera::default(), &options, &mut |p| {
        progress.push(p);
        Ok(())
    })
    .unwrap();
    assert!(
        progress
            .iter()
            .any(|p| p.stage == "Rendering surfaces" && p.total > 0 && p.completed == p.total)
    );
    assert!(
        progress
            .iter()
            .any(|p| p.stage == "Compositing canvas" && p.completed == 48 && p.total == 48)
    );
}

#[test]
fn jpeg_uses_selected_background_and_obj_retains_world_geometry() {
    let files = Files::new("jpg");
    let mut doc = Document::new(8, 8);
    doc.canvas.pixels.fill(0);
    let mut options = Options::for_document(&doc);
    options.format = Format::Jpeg;
    options.background = [20, 30, 40];
    export::run(
        &files.0,
        &doc,
        &Camera::default(),
        &options,
        &mut |_| Ok(()),
    )
    .unwrap();
    let image = persistence::import_image(&files.0).unwrap();
    assert!(
        image
            .pixels
            .as_chunks::<4>()
            .0
            .iter()
            .all(|p| p[3] == 255 && (0..3).all(|c| p[c].abs_diff(options.background[c]) <= 2))
    );
    let obj = Files::new("obj");
    doc.add_object(Primitive::Cube, [255; 4]);
    options.format = Format::Obj;
    export::run(&obj.0, &doc, &Camera::default(), &options, &mut |_| Ok(())).unwrap();
    let text = fs::read_to_string(&obj.0).unwrap();
    assert!(text.contains("\nv "));
    assert!(text.contains("\nf "));
}
