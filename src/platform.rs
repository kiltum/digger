//! Communication between the winit/UI thread, the game thread and the
//! audio callback.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use winit::event_loop::EventLoopProxy;

use crate::constants::{FB_H, FB_W};
use crate::input::SharedInput;

#[derive(Clone, Copy, Debug)]
pub enum UserEvent {
    /// A new frame was published by the game thread.
    Frame,
}

pub struct Shared {
    pub input: Arc<SharedInput>,
    frame: Mutex<Vec<u32>>,
    pub running: AtomicBool,
    proxy: Option<EventLoopProxy<UserEvent>>,
}

impl Shared {
    pub fn new(proxy: EventLoopProxy<UserEvent>) -> Arc<Self> {
        Self::with_proxy(Some(proxy))
    }

    /// A frame sink for headless runs (no window/event loop).
    pub fn headless() -> Arc<Self> {
        Self::with_proxy(None)
    }

    fn with_proxy(proxy: Option<EventLoopProxy<UserEvent>>) -> Arc<Self> {
        Arc::new(Self {
            input: SharedInput::new(),
            frame: Mutex::new(vec![0; FB_W * FB_H]),
            running: AtomicBool::new(true),
            proxy,
        })
    }

    /// Publish a composed 0x00RRGGBB frame.
    pub fn publish(&self, rgba: &[u32]) {
        if let Ok(mut frame) = self.frame.lock() {
            let n = frame.len().min(rgba.len());
            frame[..n].copy_from_slice(&rgba[..n]);
        }
        if let Some(proxy) = &self.proxy {
            let _ = proxy.send_event(UserEvent::Frame);
        }
    }

    pub fn snapshot(&self) -> Vec<u32> {
        self.frame
            .lock()
            .map(|f| f.clone())
            .unwrap_or_else(|_| vec![0; FB_W * FB_H])
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}
