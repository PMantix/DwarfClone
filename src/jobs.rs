//! Long jobs (embarking, saving, loading) run on a worker
//! thread so the screen can keep drawing a loading screen with progress.

use crate::game::Game;
use crate::saves::SaveMeta;
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// What to do once a save has been written.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AfterSave {
    Resume,
    Quit,
    Title,
}

pub enum JobDone {
    Embarked(Game),
    Saved(Game, SaveMeta, AfterSave),
    Loaded(Game),
}

/// Reports the current stage and how far along the job is (0..1).
pub type Report<'a> = &'a dyn Fn(&str, f32);

pub struct Job {
    pub title: String,
    pub started: Instant,
    progress: Arc<Mutex<(String, f32)>>,
    rx: Receiver<Result<JobDone, String>>,
}

impl Job {
    pub fn spawn<F>(title: &str, work: F) -> Job
    where
        F: FnOnce(Report) -> Result<JobDone, String> + Send + 'static,
    {
        let progress = Arc::new(Mutex::new(("Starting".to_string(), 0.0)));
        let (tx, rx) = channel();
        let p = Arc::clone(&progress);
        std::thread::spawn(move || {
            let report = |stage: &str, frac: f32| {
                if let Ok(mut g) = p.lock() {
                    *g = (stage.to_string(), frac.clamp(0.0, 1.0));
                }
            };
            let _ = tx.send(work(&report));
        });
        Job { title: title.to_string(), started: Instant::now(), progress, rx }
    }

    /// Stage name and fraction done.
    pub fn status(&self) -> (String, f32) {
        self.progress.lock().map(|g| g.clone()).unwrap_or_else(|_| ("Working".into(), 0.0))
    }

    /// The result, once the worker has finished.
    pub fn poll(&self) -> Option<Result<JobDone, String>> {
        match self.rx.try_recv() {
            Ok(r) => Some(r),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Err("The job stopped unexpectedly.".into())),
        }
    }
}
