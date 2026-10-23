use std::{
    fs::{self, File},
    io,
    path::{Component, Path},
};
fn allowed(path: &Path) -> bool {
    if path
        .components()
        .any(|p| !matches!(p, Component::Normal(_)))
    {
        return false;
    }
    let Some(first) = path.components().next() else {
        return false;
    };
    let first = first.as_os_str().to_string_lossy();
    matches!(
        first.as_ref(),
        "Ada3D.exe"
            | "Ada3D"
            | "Ada3D.app"
            | "LICENSE"
            | "COPYRIGHT"
            | "THIRD_PARTY_NOTICES.md"
            | "licenses"
            | "SOURCE.txt"
            | "share"
            | "install.sh"
    )
}
pub(super) fn unpack(archive: &Path, destination: &Path) -> Result<(), String> {
    let mut zip = zip::ZipArchive::new(File::open(archive).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    if zip.len() > 10000 {
        return Err("Too many files in update package.".into());
    }
    let mut total = 0u64;
    for i in 0..zip.len() {
        let entry = zip.by_index(i).map_err(|e| e.to_string())?;
        let path = entry.enclosed_name().ok_or("Unsafe update path")?;
        if !allowed(&path)
            || entry.name().contains('\\')
            || entry.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000)
        {
            return Err("Unsafe file in update package.".into());
        }
        total = total
            .checked_add(entry.size())
            .ok_or("Update is too large")?;
        if total > 1024 * 1024 * 1024 {
            return Err("Update expands beyond its size limit.".into());
        }
    }
    fs::create_dir(destination).map_err(|e| e.to_string())?;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
        let output = destination.join(entry.enclosed_name().ok_or("Unsafe update path")?);
        if entry.is_dir() {
            fs::create_dir_all(&output).map_err(|e| e.to_string())?;
            continue;
        }
        fs::create_dir_all(output.parent().ok_or("Missing update parent")?)
            .map_err(|e| e.to_string())?;
        let mut file = File::create(&output).map_err(|e| e.to_string())?;
        let size = io::copy(&mut entry, &mut file).map_err(|e| e.to_string())?;
        if size != entry.size() {
            return Err("Incomplete update file.".into());
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let executable = entry.unix_mode().is_some_and(|m| m & 0o111 != 0);
            fs::set_permissions(
                &output,
                fs::Permissions::from_mode(if executable { 0o755 } else { 0o644 }),
            )
            .map_err(|e| e.to_string())?;
        }
    }
    let binary = if cfg!(target_os = "windows") {
        destination.join("Ada3D.exe")
    } else if cfg!(target_os = "macos") {
        destination.join("Ada3D.app/Contents/MacOS/Ada3D")
    } else {
        destination.join("Ada3D")
    };
    if !binary.is_file() {
        return Err("The update package does not contain Ada3D.".into());
    }
    use std::io::Read;
    let mut magic = [0u8; 4];
    File::open(&binary)
        .and_then(|mut file| file.read_exact(&mut magic))
        .map_err(|e| e.to_string())?;
    let valid = if cfg!(target_os = "windows") {
        magic[..2] == *b"MZ"
    } else if cfg!(target_os = "macos") {
        matches!(magic, [0xcf, 0xfa, 0xed, 0xfe] | [0xca, 0xfe, 0xba, 0xbe])
    } else {
        magic == *b"\x7fELF"
    };
    if !valid {
        return Err("The update executable is not valid for this platform.".into());
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn update_paths_are_limited_to_release_files() {
        for path in [
            "../Ada3D.exe",
            "/Ada3D.exe",
            "settings.json",
            "docs/log.txt",
            "Ada3D.app/../escape",
        ] {
            assert!(!allowed(Path::new(path)));
        }
        for path in [
            "Ada3D.exe",
            "Ada3D",
            "Ada3D.app/Contents/MacOS/Ada3D",
            "licenses/LICENSE-MIT.txt",
        ] {
            assert!(allowed(Path::new(path)));
        }
    }

    fn fixture(extra: Option<&str>) -> (std::path::PathBuf, std::path::PathBuf) {
        use std::io::Write;
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("Ada3D-archive-{}-{nonce}", std::process::id()));
        fs::create_dir(&root).unwrap();
        let archive = root.join("package.zip");
        let mut writer = zip::ZipWriter::new(File::create(&archive).unwrap());
        let binary = if cfg!(target_os = "windows") {
            "Ada3D.exe"
        } else if cfg!(target_os = "macos") {
            "Ada3D.app/Contents/MacOS/Ada3D"
        } else {
            "Ada3D"
        };
        writer
            .start_file(
                binary,
                zip::write::SimpleFileOptions::default().unix_permissions(0o755),
            )
            .unwrap();
        writer
            .write_all(if cfg!(target_os = "windows") {
                b"MZ00"
            } else if cfg!(target_os = "macos") {
                b"\xcf\xfa\xed\xfe"
            } else {
                b"\x7fELF"
            })
            .unwrap();
        if let Some(path) = extra {
            writer
                .start_file(path, zip::write::SimpleFileOptions::default())
                .unwrap();
            writer.write_all(b"malicious file").unwrap();
        }
        writer.finish().unwrap();
        (root, archive)
    }
    #[test]
    fn valid_package_extracts_the_expected_executable() {
        let (root, archive) = fixture(None);
        let output = root.join("payload");
        unpack(&archive, &output).unwrap();
        assert!(output.is_dir());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn traversal_and_unexpected_files_are_rejected_before_extraction() {
        for path in ["../outside.txt", "settings.json", "../../Ada3D.exe"] {
            let (root, archive) = fixture(Some(path));
            let output = root.join("payload");
            assert!(unpack(&archive, &output).is_err());
            assert!(!output.exists());
            assert!(!root.join("outside.txt").exists());
            fs::remove_dir_all(root).unwrap();
        }
    }
}
