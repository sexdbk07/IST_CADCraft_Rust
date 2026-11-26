//! Saving without freezing the window. QSAVE of a drawing that already has its file is encoded
//! and written on a worker thread ([`cadcraft_engine::cmd::file::SaveJob`]); the UI shows a
//! "Saving ..." card and keeps painting, and the drawing is marked saved as of the moment the
//! save started. Edits made meanwhile stay unsaved. The web build has no threads and saves in one
//! step.

use std::sync::mpsc::{Receiver, TryRecvError, channel};
use std::time::{Duration, Instant};

use cadcraft_engine::cmd::file::{SaveJob, SaveOutcome};

use crate::theme::Tokens;

struct Running {
    name: String,
    started: Instant,
    rx: Receiver<(SaveJob, SaveOutcome)>,
}

/// What [`Saving::poll`] found.
pub enum Polled {
    /// Nothing is being saved, or the save is still running.
    Nothing,
    /// The save ended: finish it with [`cadcraft_engine::cmd::file::finish_qsave`].
    Done(SaveJob, SaveOutcome),
    /// The worker stopped without an answer; the file may not have been written.
    Lost(String),
}

/// The save running in the background, if any.
#[derive(Default)]
pub struct Saving {
    running: Option<Running>,
}

impl Saving {
    /// Whether a save is running.
    pub fn busy(&self) -> bool {
        self.running.is_some()
    }

    /// Run `job` on a worker thread. False when no thread could be started (nothing was
    /// written): the caller saves in one step instead.
    pub fn start(&mut self, job: SaveJob) -> bool {
        let name = std::path::Path::new(job.path()).file_name().map_or_else(|| job.path().to_string(), |n| n.to_string_lossy().into_owned());
        let (tx, rx) = channel();
        let spawned = std::thread::Builder::new().name("cadcraft-save".into()).spawn(move || {
            let outcome = job.run();
            let _ = tx.send((job, outcome));
        });
        if spawned.is_err() {
            return false;
        }
        self.running = Some(Running { name, started: Instant::now(), rx });
        true
    }

    /// The save that ended since the last call, if any. Called every frame.
    pub fn poll(&mut self) -> Polled {
        let Some(r) = &self.running else { return Polled::Nothing };
        match r.rx.try_recv() {
            Ok((job, outcome)) => {
                self.running = None;
                Polled::Done(job, outcome)
            }
            Err(TryRecvError::Empty) => Polled::Nothing,
            Err(TryRecvError::Disconnected) => {
                let name = r.name.clone();
                self.running = None;
                Polled::Lost(crate::tf!("Saving {name} stopped unexpectedly; the file may not have been written.", name = name))
            }
        }
    }

    /// Wait for the running save to end (closing the window, quitting), up to `limit`.
    pub fn wait(&mut self, limit: Duration) -> Polled {
        let Some(r) = &self.running else { return Polled::Nothing };
        match r.rx.recv_timeout(limit) {
            Ok((job, outcome)) => {
                self.running = None;
                Polled::Done(job, outcome)
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => Polled::Nothing,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                let name = r.name.clone();
                self.running = None;
                Polled::Lost(crate::tf!("Saving {name} stopped unexpectedly; the file may not have been written.", name = name))
            }
        }
    }

    /// Paint the card at the bottom of the window and keep frames coming while the file is
    /// written.
    pub fn show(&self, ctx: &egui::Context) {
        let Some(r) = &self.running else { return };
        let t = Tokens::get();
        let label = crate::tf!("Saving {name}... {secs} s", name = r.name, secs = format!("{:.0}", r.started.elapsed().as_secs_f32()));
        egui::Area::new(egui::Id::unique("cc_saving"))
            .order(egui::Order::Foreground)
            .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -72.0))
            .interactable(false)
            .show(ctx, |ui| {
                egui::Frame::NONE
                    .fill(t.panel)
                    .stroke(egui::Stroke::new(1.0, t.accent))
                    .corner_radius(6.0)
                    .inner_margin(egui::Margin::symmetric(14, 8))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.add(egui::Spinner::new().color(t.accent));
                            ui.label(egui::RichText::new(&label).color(t.text));
                        });
                    });
            });
        ctx.request_repaint_after(Duration::from_millis(100));
    }
}
