//! One-shot GitHub release checks and verified, staged updates.
mod archive;
mod install;
mod network;
use std::path::PathBuf;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Phase {
    #[default]
    Idle,
    NotConfigured,
    Checking,
    Current,
    Downloading,
    Verifying,
    Ready,
    Error,
}
#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    pub phase: Phase,
    pub message: String,
    pub progress: Option<f32>,
    pub version: Option<String>,
    pub(crate) staged: Option<PathBuf>,
}
impl Snapshot {
    pub fn busy(&self) -> bool {
        matches!(
            self.phase,
            Phase::Checking | Phase::Downloading | Phase::Verifying
        )
    }
}
#[derive(Clone, Default)]
pub struct Updater {
    state: Arc<Mutex<Snapshot>>,
    running: Arc<AtomicBool>,
}
impl Updater {
    pub fn snapshot(&self) -> Snapshot {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
    pub fn set_error(&self, message: String) {
        eprintln!("Ada3D update: {message}");
        self.set(
            Phase::Error,
            "Couldn’t finish the update. Please try again later.".into(),
            None,
        );
    }
    fn set(&self, phase: Phase, message: String, progress: Option<f32>) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.phase = phase;
        state.message = message;
        state.progress = progress;
    }
    pub fn check(
        &self,
        repository: &str,
        automatic: bool,
        notify: impl Fn() + Send + Sync + 'static,
    ) {
        if self.running.swap(true, Ordering::AcqRel) {
            return;
        }
        if repository.is_empty() {
            self.set(
                Phase::NotConfigured,
                "Updates are unavailable right now. Please try again later.".into(),
                None,
            );
            self.running.store(false, Ordering::Release);
            notify();
            return;
        }
        if !valid_repository(repository) {
            self.set_error("Use a GitHub repository in owner/name format.".into());
            self.running.store(false, Ordering::Release);
            notify();
            return;
        }
        self.set(Phase::Checking, "Checking for updates...".into(), None);
        notify();
        let updater = self.clone();
        let repository = repository.to_owned();
        std::thread::spawn(move || {
            let report = |phase, message, progress| {
                updater.set(phase, message, progress);
                notify();
            };
            match network::check_and_download(&repository, automatic, &report) {
                Ok(Some((version, staged))) => {
                    let mut state = updater.state.lock().unwrap_or_else(|e| e.into_inner());
                    *state = Snapshot {
                        phase: Phase::Ready,
                        message: format!("Ada3D {version} is ready to install"),
                        progress: Some(1.0),
                        version: Some(version),
                        staged: Some(staged),
                    };
                }
                Ok(None) => updater.set(Phase::Current, "You're up to date".into(), Some(1.0)),
                Err(message) => updater.set_error(message),
            }
            updater.running.store(false, Ordering::Release);
            notify();
        });
    }
    pub fn install_on_exit(&self) -> Result<(), String> {
        let state = self.snapshot();
        if state.phase != Phase::Ready {
            return Err("No verified update is ready.".into());
        }
        let staged = state.staged.as_deref().ok_or("No staged update")?;
        let attempt = staged
            .parent()
            .ok_or("Missing update directory")?
            .join("last-install-attempt.json");
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs();
        let marker = serde_json::json!({"version": state.version, "time": timestamp});
        std::fs::write(
            attempt,
            serde_json::to_vec(&marker).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        install::launch(staged)
    }
}
pub fn repository() -> &'static str {
    option_env!("ADA3D_UPDATE_REPOSITORY").unwrap_or("R3FStudio/Ada3D")
}
pub fn valid_repository(value: &str) -> bool {
    let parts: Vec<_> = value.split('/').collect();
    parts.len() == 2
        && parts.iter().all(|part| {
            !part.is_empty()
                && part.len() <= 100
                && *part != "."
                && *part != ".."
                && part
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
        })
}
pub fn platform() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repository_rejects_urls_and_traversal() {
        assert!(valid_repository("R3F-Studio/Ada3D"));
        for repo in [
            "",
            "../Ada3D",
            "https://github.com/a/b",
            "a/b/c",
            "a/",
            "a/b?token=x",
        ] {
            assert!(!valid_repository(repo));
        }
    }
    #[test]
    fn missing_repository_does_not_start_network_work() {
        let updater = Updater::default();
        updater.check("", false, || {});
        assert_eq!(updater.snapshot().phase, Phase::NotConfigured);
        assert!(!updater.snapshot().busy());
    }
}
