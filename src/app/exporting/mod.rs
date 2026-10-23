//! Native export window; snapshot-based work runs outside the UI thread.
mod capture;
mod options;
mod window;
use super::*;
use adars::export::{self, Format, IconFit, Options, Sampling, Source};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use window::draw;

const TITLE: &str = "Ada3D · Export";
fn viewport() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("ada-export")
}

struct State {
    options: Options,
    path: PathBuf,
    path_text: String,
    aspect_lock: bool,
    overwrite_pending: bool,
    job: Option<jobs::Job<Vec<PathBuf>>>,
    message: String,
    outputs: Vec<PathBuf>,
    capture: Option<capture::Capture>,
    auto_export: bool,
}
pub struct ExportDialog {
    doc: Arc<Document>,
    camera: Camera,
    state: Arc<Mutex<State>>,
    closed: Arc<AtomicBool>,
    focus: Arc<AtomicBool>,
}
impl ExportDialog {
    pub fn new(
        doc: &Document,
        camera: &Camera,
        path: Option<&std::path::Path>,
        source: Source,
        format: Format,
    ) -> Self {
        let mut options = Options::for_document(doc);
        options.source = source;
        options.format = format;
        let path = path
            .map(|p| p.with_extension(format.extension()))
            .unwrap_or_else(|| PathBuf::from(format!("Artwork.{}", format.extension())));
        Self {
            doc: Arc::new(doc.clone()),
            camera: camera.clone(),
            state: Arc::new(Mutex::new(State {
                options,
                path_text: path.to_string_lossy().into_owned(),
                path,
                aspect_lock: true,
                overwrite_pending: false,
                job: None,
                message: String::new(),
                outputs: Vec::new(),
                capture: None,
                auto_export: false,
            })),
            closed: Arc::new(AtomicBool::new(false)),
            focus: Arc::new(AtomicBool::new(true)),
        }
    }
    pub fn was_closed(&self) -> bool {
        self.closed.load(Ordering::Relaxed)
    }
    pub fn cancel(&self) {
        if let Some(job) = &self.state.lock().unwrap_or_else(|e| e.into_inner()).job {
            job.cancel();
        }
    }
    pub fn is_running(&self) -> bool {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .job
            .is_some()
    }
    pub fn request_focus(&self) {
        self.focus.store(true, Ordering::Relaxed);
    }
    pub fn show(&self, ctx: &egui::Context, prefs: &Preferences) {
        if self.was_closed() {
            return;
        }
        let state = self.state.clone();
        let closed = self.closed.clone();
        let doc = self.doc.clone();
        let camera = self.camera.clone();
        let prefs = prefs.clone();
        let focus = self.focus.clone();
        let (active, height) = {
            let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            (
                state.capture.is_none(),
                match state.options.format {
                    Format::Ico => 650.0,
                    Format::Obj => 460.0,
                    _ => 520.0,
                },
            )
        };
        ctx.show_viewport_deferred(
            viewport(),
            egui::ViewportBuilder::default()
                .with_title(TITLE)
                .with_active(active)
                .with_inner_size([650.0, height])
                .with_resizable(false)
                .with_decorations(true)
                .with_close_button(true)
                .with_minimize_button(true)
                .with_maximize_button(false)
                .with_icon(icons::window_icon())
                .with_transparent(cfg!(target_os = "windows")),
            move |ctx, _| {
                if focus.swap(false, Ordering::Relaxed) {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                }
                let mut state = state.lock().unwrap_or_else(|e| e.into_inner());
                if ctx.input(|i| i.viewport().close_requested()) {
                    if let Some(job) = &state.job {
                        job.cancel();
                    }
                    closed.store(true, Ordering::Relaxed);
                    ctx.request_repaint_of(egui::ViewportId::ROOT);
                    return;
                }
                if let Some(handle) = native_window::owned_window_key(TITLE) {
                    let key = (
                        handle,
                        prefs.glass_effects,
                        prefs.backdrop,
                        ctx.theme() == egui::Theme::Dark,
                    );
                    let id = egui::Id::new("export-material");
                    if ctx.data(|d| d.get_temp::<(usize, bool, Backdrop, bool)>(id)) != Some(key)
                        && native_window::configure_owned(TITLE, key.1, key.2, key.3).is_ok()
                    {
                        ctx.data_mut(|d| d.insert_temp(id, key));
                    }
                }
                if let Some(result) = state.job.as_ref().and_then(|job| job.poll()) {
                    state.job = None;
                    match result {
                        Ok(paths) => {
                            state.message = "Export complete".into();
                            state.outputs = paths;
                        }
                        Err(error) => state.message = error,
                    }
                }
                if state.auto_export {
                    state.auto_export = false;
                    let path = state.path.clone();
                    let options = state.options.clone();
                    let doc = doc.clone();
                    let camera = camera.clone();
                    match jobs::Job::start(ctx, viewport(), move |report| {
                        export::run(&path, &doc, &camera, &options, report)
                    }) {
                        Ok(job) => state.job = Some(job),
                        Err(e) => state.message = e,
                    }
                }
                draw(ctx, &mut state, &doc, &camera);
                capture::tick(ctx, &mut state);
            },
        );
    }
}
