//! Restart helpers install a verified package after the editor exits.
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
#[cfg(any(not(target_os = "windows"), test))]
fn quote(value: &Path) -> String {
    format!("'{}'", value.to_string_lossy().replace('\'', "'\\''"))
}
#[cfg(target_os = "windows")]
fn ps_quote(value: &Path) -> String {
    format!("'{}'", value.to_string_lossy().replace('\'', "''"))
}
pub(super) fn launch(stage: &Path) -> Result<(), String> {
    let stage = stage.canonicalize().map_err(|e| e.to_string())?;
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let target = install_target(&executable)?;
    let parent = target.parent().ok_or("No installation directory")?;
    let probe = parent.join(format!(".Ada3D-write-test-{}", std::process::id()));
    let file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
        .map_err(|_| {
            "The installation directory is not writable. Install the release manually.".to_string()
        })?;
    drop(file);
    fs::remove_file(probe).map_err(|e| e.to_string())?;
    let payload = stage.join("payload");
    if !payload.is_dir() {
        return Err("The staged update is missing.".into());
    }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let script = stage.join("install.ps1");
        fs::write(
            &script,
            windows_script(&target, &payload, std::process::id()),
        )
        .map_err(|e| e.to_string())?;
        Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-WindowStyle",
                "Hidden",
                "-File",
            ])
            .arg(&script)
            .creation_flags(0x08000000)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        let script = stage.join("install.sh");
        fs::write(
            &script,
            unix_script(
                &target,
                &payload,
                std::process::id(),
                cfg!(target_os = "macos"),
            ),
        )
        .map_err(|e| e.to_string())?;
        Command::new("/bin/sh")
            .arg(&script)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
fn install_target(executable: &Path) -> Result<PathBuf, String> {
    if cfg!(target_os = "macos") {
        let bundle = executable
            .ancestors()
            .nth(3)
            .ok_or("Ada3D is not running from an app bundle")?;
        if bundle.extension().and_then(|s| s.to_str()) != Some("app") {
            return Err("Install Ada3D.app to enable automatic installation.".into());
        }
        Ok(bundle.to_path_buf())
    } else {
        Ok(executable.to_path_buf())
    }
}
#[cfg(target_os = "windows")]
fn windows_script(target: &Path, payload: &Path, pid: u32) -> String {
    format!(
        r#"$ErrorActionPreference = 'Stop'
$destination = {target}
$payload = {payload}
$backup = $destination + '.previous-' + [Guid]::NewGuid().ToString('N')
$movedOld = $false
$replacement = $destination + '.new-{pid}'
try {{
    $limit = (Get-Date).AddMinutes(2)
    while (Get-Process -Id {pid} -ErrorAction SilentlyContinue) {{
        if ((Get-Date) -gt $limit) {{ throw 'Ada3D did not exit in time.' }}
        Start-Sleep -Milliseconds 200
    }}
    Copy-Item -LiteralPath (Join-Path $payload 'Ada3D.exe') -Destination $replacement
    Move-Item -LiteralPath $destination -Destination $backup
    $movedOld = $true
    try {{ Move-Item -LiteralPath $replacement -Destination $destination }}
    catch {{ Move-Item -LiteralPath $backup -Destination $destination; throw }}
    foreach ($name in @('LICENSE','COPYRIGHT','THIRD_PARTY_NOTICES.md','SOURCE.txt','licenses')) {{
        $source = Join-Path $payload $name
        if (Test-Path -LiteralPath $source) {{ Copy-Item -LiteralPath $source -Destination (Split-Path -Parent $destination) -Recurse -Force }}
    }}
    Start-Process -FilePath $destination
}} catch {{
    if ($movedOld -and (Test-Path -LiteralPath $backup)) {{
        if (Test-Path -LiteralPath $destination) {{ Remove-Item -LiteralPath $destination -Force }}
        Move-Item -LiteralPath $backup -Destination $destination
    }}
    $_ | Out-String | Set-Content -LiteralPath (Join-Path (Split-Path -Parent $payload) 'update-error.log')
    if (Test-Path -LiteralPath $destination) {{ Start-Process -FilePath $destination }}
}}
"#,
        target = ps_quote(target),
        payload = ps_quote(payload)
    )
}
#[cfg(any(not(target_os = "windows"), test))]
fn unix_script(target: &Path, payload: &Path, pid: u32, mac: bool) -> String {
    let source = payload.join(if mac { "Ada3D.app" } else { "Ada3D" });
    let restart = if mac {
        "open -n \"$target\""
    } else {
        "\"$target\" >/dev/null 2>&1 &"
    };
    let copy = if mac {
        "cp -R \"$source\" \"$replacement\""
    } else {
        "cp \"$source\" \"$replacement\"\nchmod 755 \"$replacement\""
    };
    let notices = if mac {
        String::new()
    } else {
        format!(
            "for name in LICENSE COPYRIGHT THIRD_PARTY_NOTICES.md SOURCE.txt licenses; do\n  if [ -e \"$payload/$name\" ]; then cp -R \"$payload/$name\" {} ; fi\ndone",
            quote(target.parent().unwrap_or(Path::new(".")))
        )
    };
    format!(
        r#"#!/bin/sh
set -eu
target={target}
source={source}
payload={payload}
backup="$target.previous-{pid}"
replacement="$target.new-{pid}"
attempt=0
while kill -0 {pid} 2>/dev/null; do
    attempt=$((attempt + 1))
    [ "$attempt" -lt 600 ] || exit 1
    sleep 0.2
done
{copy}
mv "$target" "$backup"
if ! mv "$replacement" "$target"; then mv "$backup" "$target"; exit 1; fi
{notices}
{restart}
"#,
        target = quote(target),
        source = quote(&source),
        payload = quote(payload)
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn helper_quotes_paths_and_waits_without_killing_processes() {
        assert_eq!(quote(Path::new("Ada's folder")), "'Ada'\\''s folder'");
        let script = unix_script(
            Path::new("/apps/Ada3D"),
            Path::new("/cache/update"),
            1234,
            false,
        );
        assert!(script.contains("kill -0 1234"));
        assert!(!script.contains("kill -9"));
        assert!(script.contains("mv \"$backup\" \"$target\""));
        #[cfg(target_os = "windows")]
        {
            assert_eq!(ps_quote(Path::new("Ada's folder")), "'Ada''s folder'");
            let script = windows_script(Path::new("C:/Ada3D.exe"), Path::new("C:/cache"), 1234);
            assert!(script.contains("Get-Process -Id 1234"));
            assert!(!script.contains("Stop-Process"));
        }
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_installer_waits_replaces_fixture_and_restarts_new_binary() {
        use std::os::windows::process::CommandExt;
        use std::{
            io::Write,
            process::Stdio,
            time::{Duration, Instant, SystemTime, UNIX_EPOCH},
        };
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("Ada3D updater's test-{nonce}"));
        let payload = root.join("payload");
        fs::create_dir_all(&payload).unwrap();
        let target = root.join("Ada3D.exe");
        for (name, destination) in [("old", target.clone()), ("new", payload.join("Ada3D.exe"))] {
            let source = root.join(format!("{name}.rs"));
            fs::write(&source,format!(r##"#![windows_subsystem="windows"]
fn main() {{
 if std::env::args().any(|s|s=="--hold") {{ std::thread::sleep(std::time::Duration::from_secs(2)); }}
 else {{ let file=std::env::current_exe().unwrap().parent().unwrap().join("restarted.txt"); std::fs::write(file,"{name}").unwrap(); }}
}}
"##)).unwrap();
            let result = Command::new("rustc")
                .arg(&source)
                .args(["--crate-name", "update_fixture"])
                .arg("-o")
                .arg(destination)
                .output()
                .unwrap();
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
        }
        fs::write(root.join("notes.txt"), "user document").unwrap();
        fs::write(payload.join("LICENSE"), "new license").unwrap();
        let old_bytes = fs::read(&target).unwrap();
        let new_bytes = fs::read(payload.join("Ada3D.exe")).unwrap();
        let mut old = Command::new(&target)
            .arg("--hold")
            .creation_flags(0x08000000)
            .spawn()
            .unwrap();
        let pid = old.id();
        let script = root.join("install.ps1");
        fs::File::create(&script)
            .unwrap()
            .write_all(windows_script(&target, &payload, pid).as_bytes())
            .unwrap();
        let mut helper = Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-File"])
            .arg(&script)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(0x08000000)
            .spawn()
            .unwrap();
        std::thread::sleep(Duration::from_millis(150));
        assert_eq!(fs::read(&target).unwrap(), old_bytes);
        assert!(old.wait().unwrap().success());
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            if helper.try_wait().unwrap().is_some() {
                break;
            }
            if Instant::now() > deadline {
                let _ = helper.kill();
                panic!("Update helper did not finish");
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        while !root.join("restarted.txt").is_file() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(25));
        }
        assert_eq!(fs::read(&target).unwrap(), new_bytes);
        assert_eq!(
            fs::read(
                fs::read_dir(&root)
                    .unwrap()
                    .filter_map(Result::ok)
                    .find(|entry| entry
                        .file_name()
                        .to_string_lossy()
                        .starts_with("Ada3D.exe.previous-"))
                    .unwrap()
                    .path()
            )
            .unwrap(),
            old_bytes
        );
        assert_eq!(
            fs::read_to_string(root.join("restarted.txt")).unwrap(),
            "new"
        );
        assert_eq!(
            fs::read_to_string(root.join("notes.txt")).unwrap(),
            "user document"
        );
        assert_eq!(
            fs::read_to_string(root.join("LICENSE")).unwrap(),
            "new license"
        );
        std::thread::sleep(Duration::from_millis(100));
        fs::remove_dir_all(root).unwrap();
    }
}
