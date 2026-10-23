use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

use adars::raster::Raster;
use adars::render_worker::{RenderReply, RenderRequest, RenderWorker};
use adars::scene::{Camera, Primitive, SceneObject};

fn request(generation: u64) -> RenderRequest {
    RenderRequest {
        generation,
        width: 64,
        height: 48,
        objects: vec![SceneObject::new(7, Primitive::Cube, [220, 130, 70, 255])],
        camera: Camera {
            yaw: 0.0,
            pitch: 0.0,
            distance: 7.0,
        },
        background: Raster::new(4, 2, [35, 45, 55, 255]),
        selected: None,
        wireframe: false,
    }
}

fn wait_until(mut predicate: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(1);
    while !predicate() {
        assert!(
            Instant::now() < deadline,
            "background worker did not respond within one second"
        );
        std::thread::yield_now();
    }
}

fn await_reply(worker: &RenderWorker) -> RenderReply {
    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        if let Some(reply) = worker.poll() {
            return reply;
        }
        assert!(
            Instant::now() < deadline,
            "background render did not arrive within one second"
        );
        std::thread::yield_now();
    }
}

#[test]
fn worker_returns_the_requested_generation_and_render() {
    let repaints = Arc::new(AtomicUsize::new(0));
    let callback_repaints = Arc::clone(&repaints);
    let worker = RenderWorker::new(move || {
        callback_repaints.fetch_add(1, Ordering::Release);
    });
    assert!(worker.poll().is_none());
    worker.submit(request(77)).unwrap();
    let reply = await_reply(&worker);
    assert_eq!(reply.generation, 77);
    assert_eq!(
        (reply.scene.raster.width, reply.scene.raster.height),
        (64, 48)
    );
    assert_eq!(&reply.scene.raster.pixels[0..4], &[35, 45, 55, 255]);
    assert_eq!(reply.scene.object_ids[24 * 64 + 32], Some(7));
    wait_until(|| repaints.load(Ordering::Acquire) == 1);
    assert!(worker.is_alive());
}

#[test]
fn only_one_pending_job_is_accepted_and_rejected_requests_are_preserved() {
    let (entered_tx, entered_rx) = mpsc::sync_channel::<()>(1);
    let (release_tx, release_rx) = mpsc::sync_channel::<()>(1);
    let first = AtomicBool::new(true);
    let worker = RenderWorker::new(move || {
        // Hold the worker after its first reply, without depending on render
        // speed or timing, so its one-slot request queue can be inspected.
        if first.swap(false, Ordering::AcqRel) {
            entered_tx.send(()).unwrap();
            let _ = release_rx.recv_timeout(Duration::from_secs(1));
        }
    });
    worker.submit(request(1)).unwrap();
    entered_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    worker.submit(request(2)).unwrap();
    let newest = request(3);
    let recovered = worker
        .submit(newest.clone())
        .expect_err("the one-slot job queue must be full");
    assert_eq!(recovered, newest);
    assert!(
        worker.is_alive(),
        "a full queue must not indicate worker failure"
    );
    assert_eq!(await_reply(&worker).generation, 1);
    release_tx.send(()).unwrap();
    assert_eq!(await_reply(&worker).generation, 2);
    worker.submit(recovered).unwrap();
    assert_eq!(await_reply(&worker).generation, 3);
}

struct DropSignal(Arc<AtomicBool>);

impl Drop for DropSignal {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

#[test]
fn dropping_owner_releases_worker_with_a_full_reply_queue() {
    let finished = Arc::new(AtomicBool::new(false));
    let lifecycle = DropSignal(Arc::clone(&finished));
    let (ready_tx, ready_rx) = mpsc::sync_channel::<()>(1);
    let worker = RenderWorker::new(move || {
        let _keep_lifecycle = &lifecycle;
        let _ = ready_tx.try_send(());
    });
    worker.submit(request(1)).unwrap();
    ready_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    // Do not poll the reply. A second send may be waiting behind that full
    // slot when the UI drops its receiver, which must release the thread.
    worker.submit(request(2)).unwrap();
    drop(worker);
    wait_until(|| finished.load(Ordering::Acquire));
}

#[test]
fn terminated_worker_is_detectable_and_returns_unsent_requests() {
    let worker = RenderWorker::new(|| panic!("intentional worker-failure regression test"));
    worker.submit(request(1)).unwrap();
    wait_until(|| !worker.is_alive());
    let newest = request(2);
    let recovered = worker
        .submit(newest.clone())
        .expect_err("a stopped worker must disconnect its request receiver");
    assert_eq!(recovered, newest);
    // The already completed reply remains readable even after the callback
    // terminated the worker. The UI may choose whether to display it.
    assert_eq!(worker.poll().unwrap().generation, 1);
}
