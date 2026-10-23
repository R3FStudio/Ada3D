//! User preferences, separate from editable documents and history.
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeMode {
    #[default]
    System,
    Light,
    Dark,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Accent {
    #[default]
    Blue,
    Violet,
    Teal,
}
impl Accent {
    pub fn rgb(self) -> [u8; 3] {
        match self {
            Self::Blue => [45, 104, 238],
            Self::Violet => [124, 76, 230],
            Self::Teal => [0, 145, 136],
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Backdrop {
    #[cfg_attr(not(target_os = "windows"), default)]
    Acrylic,
    #[cfg_attr(target_os = "windows", default)]
    Mica,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlphaBackground {
    #[default]
    Workspace,
    Checkerboard,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Preferences {
    pub recent_projects: Vec<PathBuf>,
    pub version: u32,
    pub theme: ThemeMode,
    pub accent: Accent,
    pub checkerboard: bool,
    pub alpha_background: AlphaBackground,
    pub workspace_grid: bool,
    pub brush_cursor: bool,
    pub reduce_motion: bool,
    pub show_welcome: bool,
    pub check_updates_on_startup: bool,
    pub glass_effects: bool,
    pub backdrop: Backdrop,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            recent_projects: Vec::new(),
            version: 1,
            theme: ThemeMode::System,
            accent: Accent::Blue,
            checkerboard: true,
            alpha_background: AlphaBackground::Workspace,
            workspace_grid: true,
            brush_cursor: true,
            reduce_motion: false,
            show_welcome: true,
            check_updates_on_startup: true,
            glass_effects: true,
            backdrop: Backdrop::default(),
        }
    }
}
impl Preferences {
    pub fn validate(&self) -> Result<(), String> {
        if self.recent_projects.len() > 12
            || self.recent_projects.iter().any(|p| {
                p.as_os_str().len() > 4096
                    || p.extension().and_then(|s| s.to_str()).is_none_or(|s| {
                        !s.eq_ignore_ascii_case(crate::persistence::PROJECT_EXTENSION)
                    })
            })
        {
            return Err("Recent project list is invalid.".into());
        }
        if self.version != 1 {
            Err("Unsupported settings version.".into())
        } else {
            Ok(())
        }
    }
    pub fn remember_project(&mut self, path: &Path) {
        if path
            .extension()
            .and_then(|s| s.to_str())
            .is_none_or(|s| !s.eq_ignore_ascii_case(crate::persistence::PROJECT_EXTENSION))
            || path.as_os_str().len() > 4096
        {
            return;
        }
        self.recent_projects.retain(|p| p != path);
        self.recent_projects.insert(0, path.to_owned());
        self.recent_projects.truncate(12);
    }
}
pub fn default_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    #[cfg(target_os = "macos")]
    let base =
        std::env::var_os("HOME").map(|p| PathBuf::from(p).join("Library/Application Support"));
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".config")));
    base.map(|p| p.join("Ada3D").join("settings.json"))
}
pub fn load(path: &Path) -> Result<Preferences, String> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            if path.file_name().and_then(|s| s.to_str()) == Some("settings.json")
                && let Some(parent) = path.parent()
                && parent.file_name().and_then(|s| s.to_str()) == Some("Ada3D")
                && let Some(base) = parent.parent()
            {
                let previous = base.join("AdaRS/settings.json");
                if previous.is_file() {
                    let preferences = load(&previous)?;
                    save(path, &preferences)?;
                    return Ok(preferences);
                }
            }
            return Ok(Preferences::default());
        }
        Err(e) => return Err(format!("Couldn't read settings: {e}")),
    };
    let meta = file.metadata().map_err(|e| e.to_string())?;
    if !meta.is_file() || meta.len() > 65536 {
        return Err("Settings file is invalid or too large.".into());
    }
    let mut bytes = Vec::new();
    file.take(65537)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 65536 {
        return Err("Settings file is too large.".into());
    }
    let preferences: Preferences =
        serde_json::from_slice(&bytes).map_err(|e| format!("Couldn't read settings: {e}"))?;
    preferences.validate()?;
    Ok(preferences)
}
pub fn save(path: &Path, preferences: &Preferences) -> Result<(), String> {
    preferences.validate()?;
    let bytes = serde_json::to_vec_pretty(preferences).map_err(|e| e.to_string())?;
    if bytes.len() > 65536 {
        return Err("Settings exceed the file size limit.".into());
    }
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Couldn't create settings folder: {e}"))?;
    }
    crate::persistence::atomic_write(path, &bytes)
}
