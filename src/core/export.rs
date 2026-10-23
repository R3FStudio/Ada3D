//! Raster, icon and mesh export with measured work counters.
use crate::{
    document::Document,
    persistence,
    progress::{Progress, Reporter},
    raster::MAX_DIMENSION,
    scene::{self, Camera},
};
use image::{ImageEncoder, RgbaImage, imageops::FilterType};
use std::{
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Format {
    #[default]
    Png,
    Jpeg,
    Ico,
    Obj,
}
impl Format {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::Ico => "ico",
            Self::Obj => "obj",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Png => "PNG image",
            Self::Jpeg => "JPEG image",
            Self::Ico => "Windows icon (.ico)",
            Self::Obj => "Wavefront geometry (.obj)",
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Source {
    #[default]
    Canvas,
    Scene,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Sampling {
    #[default]
    Nearest,
    Triangle,
    Lanczos,
}
impl Sampling {
    pub fn label(self) -> &'static str {
        match self {
            Self::Nearest => "Nearest · pixel art",
            Self::Triangle => "Bilinear",
            Self::Lanczos => "Lanczos · artwork",
        }
    }
    fn filter(self) -> FilterType {
        match self {
            Self::Nearest => FilterType::Nearest,
            Self::Triangle => FilterType::Triangle,
            Self::Lanczos => FilterType::Lanczos3,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum IconFit {
    #[default]
    Contain,
    Cover,
    Stretch,
}
#[derive(Clone, Debug)]
pub struct Options {
    pub format: Format,
    pub source: Source,
    pub width: u32,
    pub height: u32,
    pub sampling: Sampling,
    pub preserve_alpha: bool,
    pub background: [u8; 3],
    pub jpeg_quality: u8,
    pub png_compression: u8,
    pub icon_sizes: Vec<u32>,
    pub icon_fit: IconFit,
    pub resource_file: bool,
    pub rust_file: bool,
    pub resource_id: u16,
}
impl Options {
    pub fn for_document(doc: &Document) -> Self {
        Self {
            format: Format::Png,
            source: Source::Canvas,
            width: doc.canvas.width,
            height: doc.canvas.height,
            sampling: Sampling::Nearest,
            preserve_alpha: true,
            background: [255; 3],
            jpeg_quality: 92,
            png_compression: 1,
            icon_sizes: vec![16, 24, 32, 48, 64, 128, 256],
            icon_fit: IconFit::Contain,
            resource_file: false,
            rust_file: false,
            resource_id: 101,
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.width == 0
            || self.height == 0
            || self.width > MAX_DIMENSION
            || self.height > MAX_DIMENSION
        {
            return Err(format!("Export dimensions must be 1–{MAX_DIMENSION}."));
        }
        if !(1..=100).contains(&self.jpeg_quality) || self.png_compression > 2 {
            return Err("Invalid encoding options.".into());
        }
        if self.format == Format::Ico
            && (self.icon_sizes.is_empty()
                || self.icon_sizes.len() > 7
                || self
                    .icon_sizes
                    .iter()
                    .any(|n| ![16, 24, 32, 48, 64, 128, 256].contains(n))
                || self.resource_id == 0)
        {
            return Err("Choose supported icon sizes and a nonzero resource ID.".into());
        }
        Ok(())
    }
}

pub fn output_paths(path: &Path, options: &Options) -> Vec<PathBuf> {
    let mut paths = vec![path.to_owned()];
    if options.format == Format::Ico {
        if options.resource_file {
            paths.push(path.with_extension("rc"));
        }
        if options.rust_file {
            paths.push(path.with_extension("icon.rs"));
        }
    }
    paths
}

pub fn run(
    path: &Path,
    doc: &Document,
    camera: &Camera,
    options: &Options,
    report: &mut Reporter<'_>,
) -> Result<Vec<PathBuf>, String> {
    options.validate()?;
    doc.validate()?;
    if path
        .extension()
        .and_then(|s| s.to_str())
        .is_none_or(|s| !s.eq_ignore_ascii_case(options.format.extension()))
    {
        return Err("Destination extension does not match the selected format.".into());
    }
    if options.format == Format::Obj {
        if doc.objects.is_empty() {
            return Err("Add scene objects before exporting geometry.".into());
        }
        scene::export_obj_with_progress(path, &doc.objects, report)?;
        report(Progress::new("Export complete", 1, 1, "files"))?;
        return Ok(vec![path.to_owned()]);
    }
    report(Progress::new("Preparing artwork", 0, 1, "images"))?;
    let raster = if options.source == Source::Scene {
        scene::render_with_progress(
            scene::RenderInput {
                width: doc.canvas.width,
                height: doc.canvas.height,
                objects: &doc.objects,
                camera,
                background: Some(&doc.canvas),
                selected: None,
                wireframe: false,
            },
            report,
        )?
        .raster
    } else {
        doc.canvas.clone()
    };
    let source = RgbaImage::from_raw(raster.width, raster.height, raster.pixels)
        .ok_or("Invalid raster buffer")?;
    report(Progress::new("Preparing artwork", 1, 1, "images"))?;
    let mut bytes = Vec::new();
    if options.format == Format::Ico {
        let mut sizes = options.icon_sizes.clone();
        sizes.sort_unstable();
        sizes.dedup();
        let mut frames = Vec::new();
        report(Progress::new(
            "Generating icon sizes",
            0,
            sizes.len() as u64,
            "icons",
        ))?;
        for (index, size) in sizes.iter().copied().enumerate() {
            let mut image = icon_image(&source, size, options);
            if !options.preserve_alpha {
                flatten(&mut image, options.background);
            }
            frames.push(
                image::codecs::ico::IcoFrame::as_png(
                    image.as_raw(),
                    size,
                    size,
                    image::ExtendedColorType::Rgba8,
                )
                .map_err(|e| e.to_string())?,
            );
            report(Progress::new(
                "Generating icon sizes",
                (index + 1) as u64,
                sizes.len() as u64,
                "icons",
            ))?;
        }
        image::codecs::ico::IcoEncoder::new(&mut bytes)
            .encode_images(&frames)
            .map_err(|e| e.to_string())?;
    } else {
        report(Progress::new("Resizing artwork", 0, 1, "images"))?;
        let mut image = resize(&source, options.width, options.height, options.sampling);
        if !options.preserve_alpha || options.format == Format::Jpeg {
            flatten(&mut image, options.background);
        }
        report(Progress::new("Resizing artwork", 1, 1, "images"))?;
        let stage = if options.format == Format::Png {
            "Encoding PNG"
        } else {
            "Encoding JPEG"
        };
        report(Progress::new(stage, 0, 0, "bytes"))?;
        let writer = EncodingWriter {
            bytes: &mut bytes,
            report,
            stage,
        };
        if options.format == Format::Png {
            let compression = match options.png_compression {
                0 => image::codecs::png::CompressionType::Fast,
                2 => image::codecs::png::CompressionType::Best,
                _ => image::codecs::png::CompressionType::Default,
            };
            image::codecs::png::PngEncoder::new_with_quality(
                writer,
                compression,
                image::codecs::png::FilterType::Adaptive,
            )
            .write_image(
                image.as_raw(),
                image.width(),
                image.height(),
                image::ExtendedColorType::Rgba8,
            )
            .map_err(|e| e.to_string())?;
        } else {
            let rgb = image::DynamicImage::ImageRgba8(image).into_rgb8();
            image::codecs::jpeg::JpegEncoder::new_with_quality(writer, options.jpeg_quality)
                .encode(
                    rgb.as_raw(),
                    rgb.width(),
                    rgb.height(),
                    image::ExtendedColorType::Rgb8,
                )
                .map_err(|e| e.to_string())?;
        }
    }
    // Generate companion files before committing the primary icon.
    let mut companions = Vec::new();
    if options.format == Format::Ico && (options.resource_file || options.rust_file) {
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or("Icon file name must be valid UTF-8 for embedding helpers.")?;
        if options.resource_file {
            if name.contains(['\r', '\n']) {
                return Err("Resource file names cannot contain line breaks.".into());
            }
            let escaped = name.replace('\\', "\\\\").replace('"', "\\\"");
            let text = format!(
                "// Ada3D icon resource\r\n{} ICON \"{}\"\r\n",
                options.resource_id, escaped
            );
            let mut encoded = vec![0xff, 0xfe];
            for unit in text.encode_utf16() {
                encoded.extend_from_slice(&unit.to_le_bytes());
            }
            companions.push((path.with_extension("rc"), encoded));
        }
        if options.rust_file {
            companions.push((
                path.with_extension("icon.rs"),
                format!("pub const ADA_ICON: &[u8] = include_bytes!({name:?});\n").into_bytes(),
            ));
        }
    }
    persistence::atomic_write_with_progress(path, &bytes, report)?;
    let mut paths = vec![path.to_owned()];
    for (companion, data) in companions {
        persistence::atomic_write_with_progress(&companion, &data, report).map_err(|e| {
            format!(
                "Icon saved to {}. Could not write {}: {e}",
                path.display(),
                companion.display()
            )
        })?;
        paths.push(companion);
    }
    report(Progress::new(
        "Export complete",
        paths.len() as u64,
        paths.len() as u64,
        "files",
    ))?;
    Ok(paths)
}

struct EncodingWriter<'a, 'b> {
    bytes: &'a mut Vec<u8>,
    report: &'a mut Reporter<'b>,
    stage: &'static str,
}
impl Write for EncodingWriter<'_, '_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let before = self.bytes.len();
        self.bytes.extend_from_slice(buf);
        if before / 65536 != self.bytes.len() / 65536 {
            (self.report)(Progress::new(
                self.stage,
                self.bytes.len() as u64,
                0,
                "bytes",
            ))
            .map_err(std::io::Error::other)?;
        }
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn flatten(image: &mut RgbaImage, background: [u8; 3]) {
    for p in image.pixels_mut() {
        let a = u32::from(p[3]);
        for c in 0..3 {
            p[c] = ((u32::from(p[c]) * a + u32::from(background[c]) * (255 - a) + 127) / 255) as u8;
        }
        p[3] = 255;
    }
}

fn resize(source: &RgbaImage, width: u32, height: u32, sampling: Sampling) -> RgbaImage {
    if source.dimensions() == (width, height) {
        return source.clone();
    }
    if sampling == Sampling::Nearest {
        return image::imageops::resize(source, width, height, FilterType::Nearest);
    }
    // Filter premultiplied color so transparent pixels do not create fringes.
    let premultiplied = image::Rgba32FImage::from_fn(source.width(), source.height(), |x, y| {
        let p = source.get_pixel(x, y);
        let a = f32::from(p[3]) / 255.0;
        image::Rgba([
            f32::from(p[0]) / 255.0 * a,
            f32::from(p[1]) / 255.0 * a,
            f32::from(p[2]) / 255.0 * a,
            a,
        ])
    });
    let filtered = image::imageops::resize(&premultiplied, width, height, sampling.filter());
    RgbaImage::from_fn(width, height, |x, y| {
        let p = filtered.get_pixel(x, y);
        let a = p[3].clamp(0.0, 1.0);
        let mut out = [0; 4];
        if a > 0.00001 {
            for c in 0..3 {
                out[c] = (p[c] / a * 255.0).round().clamp(0.0, 255.0) as u8;
            }
            out[3] = (a * 255.0).round() as u8;
        }
        image::Rgba(out)
    })
}

fn icon_image(source: &RgbaImage, size: u32, options: &Options) -> RgbaImage {
    match options.icon_fit {
        IconFit::Stretch => resize(source, size, size, options.sampling),
        IconFit::Cover => {
            let side = source.width().min(source.height());
            let square = image::imageops::crop_imm(
                source,
                (source.width() - side) / 2,
                (source.height() - side) / 2,
                side,
                side,
            )
            .to_image();
            resize(&square, size, size, options.sampling)
        }
        IconFit::Contain => {
            let factor = size as f64 / source.width().max(source.height()) as f64;
            let w = (source.width() as f64 * factor).round().max(1.0) as u32;
            let h = (source.height() as f64 * factor).round().max(1.0) as u32;
            let image = resize(source, w, h, options.sampling);
            let mut canvas = RgbaImage::new(size, size);
            image::imageops::replace(
                &mut canvas,
                &image,
                i64::from((size - w) / 2),
                i64::from((size - h) / 2),
            );
            canvas
        }
    }
}
