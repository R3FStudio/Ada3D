fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=ADA3D_UPDATE_REPOSITORY");
    println!("cargo:rerun-if-changed=Cargo.toml");
    if std::env::var("TARGET").is_ok_and(|target| target.ends_with("windows-msvc")) {
        println!("cargo:rerun-if-changed=assets/brand/app.ico");
        let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
        let icon = std::env::current_dir()
            .unwrap()
            .join("assets/brand/app.ico")
            .display()
            .to_string()
            .replace('\\', "/");
        let version = std::env::var("CARGO_PKG_VERSION").unwrap();
        let numbers = version.split('.').take(3).collect::<Vec<_>>().join(",") + ",0";
        let resource_text = format!(
            r#"1 ICON "{icon}"
1 VERSIONINFO
FILEVERSION {numbers}
PRODUCTVERSION {numbers}
FILEFLAGSMASK 0x3fL
FILEFLAGS 0x0L
FILEOS 0x40004L
FILETYPE 0x1L
BEGIN
 BLOCK "StringFileInfo"
 BEGIN
  BLOCK "040904b0"
  BEGIN
   VALUE "CompanyName", "R3F Studio\0"
   VALUE "FileDescription", "Ada3D\0"
   VALUE "FileVersion", "{version}\0"
   VALUE "ProductName", "Ada3D\0"
   VALUE "ProductVersion", "{version}\0"
   VALUE "OriginalFilename", "Ada3D.exe\0"
   VALUE "LegalCopyright", "Copyright (C) 2026 R3F Studio\0"
  END
 END
 BLOCK "VarFileInfo"
 BEGIN
  VALUE "Translation", 0x0409, 1200
 END
END
"#
        );
        std::fs::write(out.join("icon.rc"), resource_text).unwrap();
        let sdk = std::env::var_os("WindowsSdkDir")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from("C:/Program Files (x86)/Windows Kits/10"));
        let mut versions: Vec<_> = std::fs::read_dir(sdk.join("bin"))
            .expect("Windows SDK is required")
            .filter_map(Result::ok)
            .map(|e| e.path().join("x64/rc.exe"))
            .filter(|p| p.exists())
            .collect();
        versions.sort();
        let rc = std::env::var_os("ADA_RC")
            .map(std::path::PathBuf::from)
            .or_else(|| versions.pop())
            .expect("Install Windows SDK resource compiler or set ADA_RC");
        let resource = out.join("icon.res");
        assert!(
            std::process::Command::new(rc)
                .arg("/nologo")
                .arg("/fo")
                .arg(&resource)
                .arg(out.join("icon.rc"))
                .status()
                .expect("Run Windows resource compiler")
                .success(),
            "Icon resource compilation failed"
        );
        println!("cargo:rustc-link-arg-bin=Ada3D={}", resource.display());
        println!("cargo:rustc-link-arg-bin=Ada3D=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg-bin=Ada3D=/MANIFESTDEPENDENCY:type='win32' name='Microsoft.Windows.Common-Controls' version='6.0.0.0' processorArchitecture='*' publicKeyToken='6595b64144ccf1df' language='*'"
        );
    }
}
