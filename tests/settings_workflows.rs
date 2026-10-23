use adars::settings::{self, Accent, Preferences, ThemeMode};

#[test]
fn recent_projects_are_bounded_deduplicated_and_persisted() {
    let file = File::new();
    let mut p = Preferences::default();
    for i in 0..15 {
        p.remember_project(std::path::Path::new(&format!("project-{i}.ada3dproject")));
    }
    assert_eq!(p.recent_projects.len(), 12);
    let prior = p.recent_projects[6].clone();
    p.remember_project(&prior);
    assert_eq!(p.recent_projects[0], prior);
    assert_eq!(p.recent_projects.len(), 12);
    p.remember_project(std::path::Path::new("photo.png"));
    assert_eq!(p.recent_projects.len(), 12);
    settings::save(&file.0, &p).unwrap();
    assert_eq!(settings::load(&file.0).unwrap(), p);
}
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

#[test]
fn oversized_recent_project_settings_preserve_the_existing_file() {
    let file = File::new();
    settings::save(&file.0, &Preferences::default()).unwrap();
    let before = fs::read(&file.0).unwrap();
    let mut p = Preferences::default();
    for i in 0..12 {
        p.remember_project(std::path::Path::new(&format!(
            "{}p-{i}.ada3dproject",
            "a\\".repeat(1900)
        )));
    }
    assert!(p.validate().is_ok());
    assert!(settings::save(&file.0, &p).is_err());
    assert_eq!(fs::read(&file.0).unwrap(), before);
}
static NEXT: AtomicU64 = AtomicU64::new(0);
struct File(PathBuf);
impl File {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!(
            "adars-settings-{}-{}.json",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )))
    }
}
impl Drop for File {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
#[test]
fn missing_settings_follow_system_by_default() {
    let p = File::new();
    assert_eq!(settings::load(&p.0).unwrap(), Preferences::default());
    assert_eq!(Preferences::default().theme, ThemeMode::System);
}
#[test]
fn preference_changes_roundtrip_and_replace_prior_file() {
    let p = File::new();
    let mut prefs = Preferences {
        theme: ThemeMode::Dark,
        accent: Accent::Violet,
        checkerboard: false,
        reduce_motion: true,
        ..Default::default()
    };
    settings::save(&p.0, &prefs).unwrap();
    assert_eq!(settings::load(&p.0).unwrap(), prefs);
    prefs.theme = ThemeMode::Light;
    settings::save(&p.0, &prefs).unwrap();
    assert_eq!(settings::load(&p.0).unwrap(), prefs);
}
#[test]
fn invalid_preferences_do_not_replace_valid_settings() {
    let p = File::new();
    settings::save(&p.0, &Preferences::default()).unwrap();
    let before = fs::read(&p.0).unwrap();
    let invalid = Preferences {
        version: 99,
        ..Default::default()
    };
    assert!(settings::save(&p.0, &invalid).is_err());
    assert_eq!(fs::read(&p.0).unwrap(), before);
}
#[test]
fn malformed_oversized_and_future_settings_are_rejected() {
    let p = File::new();
    for bytes in [
        b"not json".to_vec(),
        vec![b' '; 65537],
        br#"{"version":99}"#.to_vec(),
        br#"{"theme":"unknown"}"#.to_vec(),
    ] {
        fs::write(&p.0, bytes).unwrap();
        assert!(settings::load(&p.0).is_err());
    }
}
#[test]
fn older_settings_receive_defaults_for_new_fields() {
    let p = File::new();
    fs::write(&p.0, br#"{"version":1,"theme":"Light"}"#).unwrap();
    let loaded = settings::load(&p.0).unwrap();
    assert_eq!(loaded.theme, ThemeMode::Light);
    assert!(loaded.checkerboard);
    assert!(loaded.workspace_grid);
}

#[test]
fn previous_preferences_enable_welcome_and_glass_by_default() {
    let preferences: Preferences =
        serde_json::from_str(r#"{"version":1,"theme":"Dark","reduce_motion":true}"#).unwrap();
    assert!(preferences.show_welcome);
    assert!(preferences.glass_effects);
    assert!(preferences.reduce_motion);
}

#[test]
fn material_choice_roundtrips_and_older_settings_use_platform_default() {
    let old: Preferences = serde_json::from_str(r#"{"version":1,"glass_effects":true}"#).unwrap();
    assert_eq!(old.backdrop, adars::settings::Backdrop::default());
    let file = File::new();
    let preferences = Preferences {
        backdrop: adars::settings::Backdrop::Mica,
        ..Preferences::default()
    };
    settings::save(&file.0, &preferences).unwrap();
    assert_eq!(settings::load(&file.0).unwrap(), preferences);
}

#[test]
fn legacy_checkerboard_does_not_override_workspace_transparency_default() {
    let prefs: Preferences = serde_json::from_str(r#"{"version":1,"checkerboard":true}"#).unwrap();
    assert_eq!(
        prefs.alpha_background,
        adars::settings::AlphaBackground::Workspace
    );
}

#[test]
fn renamed_app_migrates_existing_preferences_without_losing_recent_projects() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("Ada3D-settings-migration-{nonce}"));
    let previous = root.join("AdaRS/settings.json");
    let current = root.join("Ada3D/settings.json");
    let mut prefs = adars::settings::Preferences {
        reduce_motion: true,
        ..Default::default()
    };
    prefs.remember_project(std::path::Path::new("drawing.ada3dproject"));
    adars::settings::save(&previous, &prefs).unwrap();
    assert_eq!(adars::settings::load(&current).unwrap(), prefs);
    assert!(current.is_file());
    assert_eq!(adars::settings::load(&previous).unwrap(), prefs);
    std::fs::remove_dir_all(root).unwrap();
}
