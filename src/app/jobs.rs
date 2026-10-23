//! One bounded background task per workflow; UI reads counters and completion.
use adars::progress::{Progress, Reporter};
use eframe::egui;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
    mpsc,
};
static ACTIVE_TASKS: AtomicUsize = AtomicUsize::new(0);
struct Permit;
impl Drop for Permit {
    fn drop(&mut self) {
        ACTIVE_TASKS.fetch_sub(1, Ordering::AcqRel);
    }
}

pub struct Job<T> {
    receiver: mpsc::Receiver<Result<T, String>>,
    progress: Arc<Mutex<Progress>>,
    cancelled: Arc<AtomicBool>,
}
impl<T: Send + 'static> Job<T> {
    pub fn start(
        ctx: &egui::Context,
        viewport: egui::ViewportId,
        work: impl FnOnce(&mut Reporter<'_>) -> Result<T, String> + Send + 'static,
    ) -> Result<Self, String> {
        let mut active = ACTIVE_TASKS.load(Ordering::Acquire);
        loop {
            if active >= 2 {
                return Err("Wait for the current file tasks to finish.".into());
            }
            match ACTIVE_TASKS.compare_exchange_weak(
                active,
                active + 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => break,
                Err(current) => active = current,
            }
        }
        let permit = Permit;
        let (send, receiver) = mpsc::sync_channel(1);
        let progress = Arc::new(Mutex::new(Progress::new("Starting", 0, 0, "")));
        let cancelled = Arc::new(AtomicBool::new(false));
        let state = progress.clone();
        let cancel = cancelled.clone();
        let ctx = ctx.clone();
        std::thread::Builder::new()
            .name("ada-file-task".into())
            .spawn(move || {
                let _permit = permit;
                let mut last_paint = std::time::Instant::now();
                let mut report = |value: Progress| {
                    if cancel.load(Ordering::Relaxed) {
                        return Err("Cancelled".into());
                    }
                    let mut progress = state.lock().unwrap_or_else(|e| e.into_inner());
                    let stage_changed = progress.stage != value.stage;
                    *progress = value;
                    drop(progress);
                    if stage_changed || last_paint.elapsed() >= std::time::Duration::from_millis(30)
                    {
                        ctx.request_repaint_of(viewport);
                        last_paint = std::time::Instant::now();
                    }
                    Ok(())
                };
                let result = work(&mut report);
                let result = if cancel.load(Ordering::Relaxed) {
                    Err("Cancelled".into())
                } else {
                    result
                };
                let _ = send.send(result);
                ctx.request_repaint_of(viewport);
            })
            .map_err(|e| format!("Could not start file task: {e}"))?;
        Ok(Self {
            receiver,
            progress,
            cancelled,
        })
    }
    pub fn poll(&self) -> Option<Result<T, String>> {
        match self.receiver.try_recv() {
            Ok(value) => Some(value),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                Some(Err("File worker stopped unexpectedly.".into()))
            }
        }
    }
    pub fn progress(&self) -> Progress {
        self.progress
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
}
impl<T> Drop for Job<T> {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
}
pub(super) fn active_count() -> usize {
    ACTIVE_TASKS.load(Ordering::Acquire)
}

pub fn progress_ui(ui: &mut egui::Ui, progress: &Progress) {
    ui.label(progress.stage);
    if let Some(item) = &progress.item {
        ui.add(egui::Label::new(item).truncate());
    }
    if let Some(fraction) = progress.fraction() {
        ui.add(egui::ProgressBar::new(fraction).text(format!(
            "{} / {} {}",
            progress.completed, progress.total, progress.unit
        )));
    } else {
        ui.add(egui::ProgressBar::new(0.0).text(if progress.completed > 0 {
            format!("{} {} produced", progress.completed, progress.unit)
        } else {
            "Working…".into()
        }));
    }
}
