//! Bounded background rendering for the native UI's 3D preview.
//!
//! One worker, one queued request, and one queued reply limit retained work.
//! The UI owns any newest request that cannot yet be submitted and uses the
//! generation field to discard obsolete results. None of this API waits on
//! rendering or joins the worker from the UI thread.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::thread::{self, JoinHandle};

use crate::raster::Raster;
use crate::scene::{self, Camera, RenderedScene, SceneObject};

#[derive(Clone, Debug, PartialEq)]
pub struct RenderRequest {
    pub generation: u64,
    pub width: u32,
    pub height: u32,
    pub objects: Vec<SceneObject>,
    pub camera: Camera,
    pub background: Raster,
    pub selected: Option<u64>,
    pub wireframe: bool,
}

pub struct RenderReply {
    pub generation: u64,
    pub scene: RenderedScene,
}

pub struct RenderWorker {
    requests: SyncSender<RenderRequest>,
    replies: Receiver<RenderReply>,
    thread: JoinHandle<()>,
    active: Arc<AtomicBool>,
}

impl RenderWorker {
    pub fn new(repaint: impl Fn() + Send + 'static) -> Self {
        let (requests, incoming) = mpsc::sync_channel::<RenderRequest>(1);
        let (outgoing, replies) = mpsc::sync_channel::<RenderReply>(1);
        let active = Arc::new(AtomicBool::new(true));
        let worker_active = Arc::clone(&active);
        let thread = thread::Builder::new()
            .name("Ada3D-scene-renderer".into())
            .spawn(move || {
                while let Ok(request) = incoming.recv() {
                    // Dropping the owner also cancels a queued request. A
                    // render already in progress finishes without a UI wait.
                    if !worker_active.load(Ordering::Acquire) {
                        break;
                    }
                    let scene = scene::render(
                        request.width,
                        request.height,
                        &request.objects,
                        &request.camera,
                        Some(&request.background),
                        request.selected,
                        request.wireframe,
                    );
                    if !worker_active.load(Ordering::Acquire) {
                        break;
                    }
                    // Backpressure is confined to this background thread.
                    // The first queued reply already requested a repaint;
                    // polling it permits the next reply to be delivered.
                    // Dropping the UI's receiver immediately unblocks send.
                    if outgoing
                        .send(RenderReply {
                            generation: request.generation,
                            scene,
                        })
                        .is_err()
                    {
                        break;
                    }
                    if worker_active.load(Ordering::Acquire) {
                        repaint();
                    }
                }
            })
            .expect("could not start the scene rendering worker");
        Self {
            requests,
            replies,
            thread,
            active,
        }
    }

    /// Try to queue one render without waiting. On a full or disconnected
    /// channel, return the complete request so the UI can retain its latest
    /// desired state or report a stopped worker.
    pub fn submit(&self, request: RenderRequest) -> Result<(), RenderRequest> {
        self.requests
            .try_send(request)
            .map_err(|error| match error {
                TrySendError::Full(request) | TrySendError::Disconnected(request) => request,
            })
    }

    /// Read one completed render without waiting. Generations are not filtered
    /// here; the UI decides whether the reply still matches its desired state.
    pub fn poll(&self) -> Option<RenderReply> {
        self.replies.try_recv().ok()
    }

    /// Distinguish temporary channel backpressure from worker termination.
    /// No join is performed, including when the worker panicked.
    pub fn is_alive(&self) -> bool {
        !self.thread.is_finished()
    }
}

impl Drop for RenderWorker {
    fn drop(&mut self) {
        self.active.store(false, Ordering::Release);
        // The request sender and reply receiver are then dropped normally.
        // Their disconnects wake recv/send, and dropping JoinHandle detaches
        // rather than joining an in-progress render on the UI thread.
    }
}
