//! Validated project/image I/O. Write complete sibling files before replacing
//! the destination, so validation, encoding, or write failures preserve it.

use crate::document::Document;
use crate::progress::{Progress, Reporter};
use crate::raster::{MAX_DIMENSION, Raster};
use image::{DynamicImage, ImageDecoder, ImageEncoder, ImageFormat, ImageReader, Limits};
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub const PROJECT_EXTENSION: &str = "ada3dproject";
pub const MAX_PROJECT_FILE_BYTES: u64 = 96 * 1024 * 1024;
pub const MAX_IMAGE_FILE_BYTES: u64 = 64 * 1024 * 1024;
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub fn save_project(path: &Path, document: &Document) -> Result<(), String> {
    validate_project_path(path)?;
    document.validate()?;
    let data = serde_json::to_vec(document)
        .map_err(|error| format!("Could not encode the project: {error}"))?;
    if data.len() as u64 > MAX_PROJECT_FILE_BYTES {
        return Err("The encoded project exceeds the file size limit.".into());
    }
    atomic_write(path, &data)
}

pub fn load_project(path: &Path) -> Result<Document, String> {
    load_project_with_progress(path, &mut |_| Ok(()))
}

pub fn load_project_with_progress(
    path: &Path,
    report: &mut Reporter<'_>,
) -> Result<Document, String> {
    validate_project_path(path)?;
    let mut file =
        File::open(path).map_err(|e| format!("Could not open {}: {e}", path.display()))?;
    let meta = file.metadata().map_err(|e| e.to_string())?;
    if !meta.is_file() || meta.len() > MAX_PROJECT_FILE_BYTES {
        return Err("Invalid or oversized project file.".into());
    }
    let mut data = Vec::new();
    report(Progress::new("Reading project", 0, meta.len(), "bytes"))?;
    let mut buffer = [0; 65536];
    loop {
        let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        if data.len() as u64 + n as u64 > MAX_PROJECT_FILE_BYTES {
            return Err("Project grew beyond the size limit.".into());
        }
        data.extend_from_slice(&buffer[..n]);
        report(Progress::new(
            "Reading project",
            data.len() as u64,
            meta.len().max(data.len() as u64),
            "bytes",
        ))?;
    }
    let total = data.len() as u64;
    report(Progress::new("Decoding artwork", 0, total, "bytes"))?;
    struct DecodeReader<'a, 'b, 'c> {
        bytes: &'a [u8],
        position: usize,
        report: &'b mut Reporter<'c>,
    }
    impl Read for DecodeReader<'_, '_, '_> {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let before = self.position;
            let n = (&self.bytes[self.position..]).read(buf)?;
            self.position += n;
            if self.position / 65536 != before / 65536 {
                (self.report)(Progress::new(
                    "Decoding artwork",
                    self.position as u64,
                    self.bytes.len() as u64,
                    "bytes",
                ))
                .map_err(std::io::Error::other)?;
            }
            Ok(n)
        }
    }
    let document: Document = serde_json::from_reader(DecodeReader {
        bytes: &data,
        position: 0,
        report: &mut *report,
    })
    .map_err(|error| format!("This file is not a valid Ada3D project: {error}"))?;
    report(Progress::new("Decoding artwork", total, total, "bytes"))?;
    report(Progress::new("Checking artwork", 0, 1, "checks"))?;
    document.validate()?;
    report(Progress::new("Checking artwork", 1, 1, "checks"))?;
    Ok(document)
}

fn validate_project_path(path: &Path) -> Result<(), String> {
    if path
        .extension()
        .and_then(|s| s.to_str())
        .is_some_and(|s| s.eq_ignore_ascii_case(PROJECT_EXTENSION))
    {
        Ok(())
    } else {
        Err(format!("Ada3D projects use .{PROJECT_EXTENSION} files."))
    }
}

pub fn import_image(path: &Path) -> Result<Raster, String> {
    let data = read_bounded(path, MAX_IMAGE_FILE_BYTES)?;
    let mut reader = ImageReader::new(Cursor::new(data))
        .with_guessed_format()
        .map_err(|error| format!("Could not identify this image: {error}"))?;
    if !matches!(reader.format(), Some(ImageFormat::Png | ImageFormat::Jpeg)) {
        return Err("Import supports PNG and JPEG images.".into());
    }
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_DIMENSION);
    limits.max_image_height = Some(MAX_DIMENSION);
    limits.max_alloc = Some(MAX_IMAGE_FILE_BYTES);
    reader.limits(limits);
    let decoder = reader
        .into_decoder()
        .map_err(|error| format!("Could not read image headers: {error}"))?;
    let (width, height) = decoder.dimensions();
    if width == 0 || height == 0 || width > MAX_DIMENSION || height > MAX_DIMENSION {
        return Err(format!(
            "Images must be between 1 and {MAX_DIMENSION} pixels on each side."
        ));
    }
    let image = DynamicImage::from_decoder(decoder)
        .map_err(|error| format!("Could not decode the image: {error}"))?
        .into_rgba8();
    let raster = Raster {
        width,
        height,
        pixels: image.into_raw(),
    };
    raster.validate()?;
    Ok(raster)
}

pub fn export_png(path: &Path, raster: &Raster) -> Result<(), String> {
    raster.validate()?;
    let mut data = Vec::new();
    image::codecs::png::PngEncoder::new(&mut data)
        .write_image(
            &raster.pixels,
            raster.width,
            raster.height,
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|error| format!("Could not encode the PNG image: {error}"))?;
    atomic_write(path, &data)
}

fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    let file =
        File::open(path).map_err(|error| format!("Could not open {}: {error}", path.display()))?;
    let metadata = file
        .metadata()
        .map_err(|error| format!("Could not inspect the file: {error}"))?;
    if !metadata.is_file() {
        return Err("Choose a regular file.".into());
    }
    if metadata.len() > limit {
        return Err(format!(
            "This file exceeds the {} MiB size limit.",
            limit / 1024 / 1024
        ));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("Could not read the file: {error}"))?;
    if bytes.len() as u64 > limit {
        return Err("The file grew beyond the size limit while being read.".into());
    }
    Ok(bytes)
}

pub(crate) fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    atomic_write_with_progress(path, bytes, &mut |_| Ok(()))
}

pub(crate) fn atomic_write_with_progress(
    path: &Path,
    bytes: &[u8],
    report: &mut Reporter<'_>,
) -> Result<(), String> {
    report(
        Progress::new("Writing file", 0, bytes.len() as u64, "bytes")
            .with_item(path.display().to_string()),
    )?;
    let name = path.file_name().ok_or("Choose a destination file name.")?;
    let parent = path
        .parent()
        .filter(|value| !value.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut created: Option<(PathBuf, File)> = None;
    for _ in 0..64 {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let mut temporary_name = OsString::from(".");
        temporary_name.push(name);
        temporary_name.push(format!(".tmp-{}-{sequence}", std::process::id()));
        let temporary_path = parent.join(temporary_name);
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path)
        {
            Ok(file) => {
                created = Some((temporary_path, file));
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(format!("Could not create the destination file: {error}")),
        }
    }
    let (temporary_path, mut file) =
        created.ok_or("Could not allocate a temporary destination file.")?;
    let written = (|| -> Result<(), String> {
        let mut completed = 0;
        for chunk in bytes.chunks(65536) {
            file.write_all(chunk).map_err(|e| e.to_string())?;
            completed += chunk.len();
            report(
                Progress::new(
                    "Writing file",
                    completed as u64,
                    bytes.len() as u64,
                    "bytes",
                )
                .with_item(path.display().to_string()),
            )?;
        }
        report(Progress::new("Syncing file", 0, 0, "").with_item(path.display().to_string()))?;
        file.sync_all().map_err(|e| e.to_string())
    })();
    drop(file);
    if let Err(error) = written {
        let _ = fs::remove_file(&temporary_path);
        return Err(format!("Could not finish writing the file: {error}"));
    }
    if let Err(error) = fs::rename(&temporary_path, path) {
        let _ = fs::remove_file(&temporary_path);
        return Err(format!("Could not replace the destination file: {error}"));
    }
    Ok(())
}
