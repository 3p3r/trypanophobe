use crate::detector::Detector;
use anyhow::{anyhow, Result};
use std::sync::{Arc, Condvar, Mutex};

enum SlotState {
    Idle,
    Loading,
    Ready(Detector),
    Failed(String),
}

/// Thread-safe model holder: background prewarm, blocking wait for REST inference.
pub struct DetectorSlot {
    state: Mutex<SlotState>,
    ready: Condvar,
}

pub type SharedDetector = Arc<DetectorSlot>;

impl DetectorSlot {
    pub fn new() -> SharedDetector {
        Arc::new(Self {
            state: Mutex::new(SlotState::Idle),
            ready: Condvar::new(),
        })
    }

    /// Start loading in the background; returns immediately (does not block `serve`).
    pub fn start_prewarm(self: &SharedDetector) {
        let spawn = {
            let mut guard = self.state.lock().expect("lock");
            match &*guard {
                SlotState::Ready(_) => {
                    tracing::info!("prewarm skipped — model already loaded");
                    return;
                }
                SlotState::Loading => {
                    tracing::info!("prewarm already in progress");
                    return;
                }
                SlotState::Failed(msg) => {
                    tracing::warn!(error = %msg, "retrying prewarm after failure");
                    *guard = SlotState::Loading;
                    true
                }
                SlotState::Idle => {
                    *guard = SlotState::Loading;
                    true
                }
            }
        };

        if spawn {
            tracing::info!(
                "prewarm started in background (listening now; English /api/check waits until ready)"
            );
            let slot = Arc::clone(self);
            std::thread::spawn(move || slot.run_load());
        }
    }

    fn run_load(&self) {
        tracing::info!("loading model into memory");

        let loaded = Detector::new();
        let mut guard = self.state.lock().expect("lock");
        match loaded {
            Ok(detector) => *guard = SlotState::Ready(detector),
            Err(e) => *guard = SlotState::Failed(e.to_string()),
        }
        self.ready.notify_all();
    }

    /// Block until the model is ready (or return load error). Safe under concurrent REST calls.
    pub fn wait_ready(self: &SharedDetector) -> Result<()> {
        if self.try_begin_lazy_load() {
            let slot = Arc::clone(self);
            std::thread::spawn(move || slot.run_load());
        }

        let mut guard = self.state.lock().map_err(|e| anyhow!("lock: {e}"))?;
        loop {
            match &*guard {
                SlotState::Ready(_) => return Ok(()),
                SlotState::Failed(msg) => return Err(anyhow!("model load failed: {msg}")),
                SlotState::Loading | SlotState::Idle => {
                    guard = self
                        .ready
                        .wait(guard)
                        .map_err(|e| anyhow!("condvar: {e}"))?;
                }
            }
        }
    }

    fn try_begin_lazy_load(&self) -> bool {
        let mut guard = self.state.lock().expect("lock");
        match &*guard {
            SlotState::Idle => {
                *guard = SlotState::Loading;
                tracing::info!("loading model into memory (first English /api/check)");
                true
            }
            _ => false,
        }
    }

    pub fn with_detector<F, T>(self: &SharedDetector, f: F) -> Result<T>
    where
        F: FnOnce(&mut Detector) -> Result<T>,
    {
        self.wait_ready()?;
        let mut guard = self.state.lock().map_err(|e| anyhow!("lock: {e}"))?;
        match &mut *guard {
            SlotState::Ready(detector) => f(detector),
            _ => Err(anyhow!("model not ready after wait")),
        }
    }
}
