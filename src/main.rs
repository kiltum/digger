//! Window, presentation and thread bootstrap.

use std::num::NonZeroU32;
use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::PhysicalKey;
use winit::window::{Fullscreen, Window, WindowId};

use digger::audio::Audio;
use digger::constants::{FB_H, FB_W};
use digger::game::Game;
use digger::platform::{Shared, UserEvent};
use digger::sys::Sys;

type DisplayHandle = winit::event_loop::OwnedDisplayHandle;

struct App {
    shared: Arc<Shared>,
    window: Option<Arc<Window>>,
    surface: Option<softbuffer::Surface<DisplayHandle, Arc<Window>>>,
    _context: Option<softbuffer::Context<DisplayHandle>>,
    audio: Option<Audio>,
    game_thread: Option<std::thread::JoinHandle<()>>,
}

impl App {
    fn new(shared: Arc<Shared>) -> Self {
        Self {
            shared,
            window: None,
            surface: None,
            _context: None,
            audio: None,
            game_thread: None,
        }
    }

    fn start(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("Digger")
            .with_inner_size(LogicalSize::new(1280.0, 800.0))
            .with_min_inner_size(LogicalSize::new(320.0, 200.0));
        let window = match event_loop.create_window(attrs) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                eprintln!("failed to create window: {e}");
                event_loop.exit();
                return;
            }
        };
        let context = match softbuffer::Context::new(event_loop.owned_display_handle()) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("failed to create softbuffer context: {e}");
                event_loop.exit();
                return;
            }
        };
        let surface = match softbuffer::Surface::new(&context, window.clone()) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("failed to create softbuffer surface: {e}");
                event_loop.exit();
                return;
            }
        };
        self.window = Some(window.clone());
        self._context = Some(context);
        self.surface = Some(surface);
        window.request_redraw();

        if self.audio.is_none() {
            let audio = digger::audio::start();
            let engine = audio.engine.clone();
            digger::gamepad::spawn(self.shared.input.clone());
            let shared = self.shared.clone();
            match std::thread::Builder::new()
                .name("digger-game".into())
                .spawn(move || {
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        match Sys::new(shared.clone(), Some(engine)) {
                            Ok(sys) => {
                                let mut game = Game::new(sys);
                                game.mainprog();
                            }
                            Err(e) => eprintln!("failed to load assets: {e}"),
                        }
                    }));
                    if let Err(p) = result {
                        eprintln!("game thread panicked: {p:?}");
                    }
                    shared.stop();
                }) {
                Ok(handle) => self.game_thread = Some(handle),
                Err(e) => {
                    eprintln!("failed to start game thread: {e}");
                    event_loop.exit();
                }
            }
            self.audio = Some(audio);
        }
    }

    fn present(&mut self) {
        let Some(window) = &self.window else { return };
        let Some(surface) = &mut self.surface else {
            return;
        };
        let size = window.inner_size();
        let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else {
            return;
        };
        if surface.resize(w, h).is_err() {
            return;
        }
        let Ok(mut buffer) = surface.buffer_mut() else {
            return;
        };
        let frame = self.shared.snapshot();
        buffer.fill(0);

        let sw = w.get() as f64;
        let sh = h.get() as f64;
        let scale = (sw / FB_W as f64).min(sh / FB_H as f64);
        if scale <= 0.0 {
            let _ = buffer.present();
            return;
        }
        // Prefer integer scaling for crisp pixels; fall back to nearest-neighbour
        // when the window is too small for a 1x image.
        let mut dw;
        let mut dh;
        if scale >= 1.0 {
            let s = scale.floor() as u32;
            dw = FB_W as u32 * s;
            dh = FB_H as u32 * s;
        } else {
            dw = (FB_W as f64 * scale) as u32;
            dh = (FB_H as f64 * scale) as u32;
        }
        dw = dw.max(1).min(w.get());
        dh = dh.max(1).min(h.get());
        let offx = (w.get() - dw) / 2;
        let offy = (h.get() - dh) / 2;

        let dst_w = w.get() as usize;
        for dy in 0..dh {
            let sy = (dy as u64 * FB_H as u64 / dh as u64) as usize;
            let src_row = sy.min(FB_H - 1) * FB_W;
            let dst_row = (offy + dy) as usize * dst_w + offx as usize;
            for dx in 0..dw {
                let sx = (dx as u64 * FB_W as u64 / dw as u64) as usize;
                buffer[dst_row + dx as usize] = frame[src_row + sx.min(FB_W - 1)];
            }
        }
        let _ = buffer.present();
    }
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.start(event_loop);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                self.shared.stop();
                event_loop.exit();
            }
            WindowEvent::Resized(_) => {
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            WindowEvent::Focused(false) => {
                self.shared.input.release_all();
            }
            WindowEvent::RedrawRequested => self.present(),
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    match event.state {
                        ElementState::Pressed => {
                            if !event.repeat {
                                if code == winit::keyboard::KeyCode::F11 {
                                    if let Some(w) = &self.window {
                                        if w.fullscreen().is_some() {
                                            w.set_fullscreen(None);
                                        } else {
                                            w.set_fullscreen(Some(Fullscreen::Borderless(None)));
                                        }
                                        w.request_redraw();
                                    }
                                }
                                self.shared.input.key_down(code);
                                if let Some(text) = &event.text {
                                    for c in text.chars() {
                                        self.shared.input.text(c);
                                    }
                                }
                            }
                        }
                        ElementState::Released => self.shared.input.key_up(code),
                    }
                }
            }
            _ => {}
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::Frame => {
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if !self.shared.is_running() {
            event_loop.exit();
        } else {
            event_loop.set_control_flow(ControlFlow::Wait);
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::<UserEvent>::with_user_event().build()?;
    let shared = Shared::new(event_loop.create_proxy());
    let mut app = App::new(shared);
    event_loop.run_app(&mut app)?;
    Ok(())
}
