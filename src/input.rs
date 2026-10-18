//! Keyboard input bridging the winit event thread to the game thread.
//!
//! The original polls a "keys currently down" bitmap (`GetAsyncKeyState`) and
//! additionally buffers ASCII keystrokes so very short taps are not lost.  We
//! reproduce that by keeping a shared held-key set plus an event queue.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use winit::keyboard::{KeyCode, NamedKey};

/// Events queued by the UI thread and drained by the game thread.
#[derive(Clone, Debug)]
pub enum InputEvent {
    KeyDown(KeyCode),
    KeyUp(KeyCode),
    Char(char),
}

/// Simple player-1 gamepad state. gilrs is polled on a background thread and
/// writes this; the game thread ORs it with the keyboard state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PadState {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub fire: bool,
    pub start: bool,
}

#[derive(Default)]
pub struct SharedInput {
    held: Mutex<HashSet<KeyCode>>,
    events: Mutex<Vec<InputEvent>>,
    pad: Mutex<PadState>,
}

impl SharedInput {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn key_down(&self, code: KeyCode) {
        self.held.lock().unwrap().insert(code);
        self.events.lock().unwrap().push(InputEvent::KeyDown(code));
    }

    pub fn key_up(&self, code: KeyCode) {
        self.held.lock().unwrap().remove(&code);
        self.events.lock().unwrap().push(InputEvent::KeyUp(code));
    }

    pub fn text(&self, ch: char) {
        self.events.lock().unwrap().push(InputEvent::Char(ch));
    }

    pub fn is_held(&self, code: KeyCode) -> bool {
        self.held.lock().unwrap().contains(&code)
    }

    pub fn set_pad(&self, pad: PadState) {
        if let Ok(mut slot) = self.pad.lock() {
            *slot = pad;
        }
    }

    pub fn pad(&self) -> PadState {
        self.pad.lock().map(|p| *p).unwrap_or_default()
    }

    /// Release every key (used when the window loses focus) so state cannot
    /// get stuck down.
    pub fn release_all(&self) {
        self.held.lock().unwrap().clear();
    }

    fn drain(&self) -> Vec<InputEvent> {
        let mut q = self.events.lock().unwrap();
        std::mem::take(&mut *q)
    }
}

/// Per-player control mapping.
#[derive(Clone, Copy, Debug)]
pub struct KeyMap {
    pub up: &'static [KeyCode],
    pub down: &'static [KeyCode],
    pub left: &'static [KeyCode],
    pub right: &'static [KeyCode],
    pub fire: &'static [KeyCode],
}

pub const P1_KEYS: KeyMap = KeyMap {
    up: &[KeyCode::ArrowUp],
    down: &[KeyCode::ArrowDown],
    left: &[KeyCode::ArrowLeft],
    right: &[KeyCode::ArrowRight],
    fire: &[KeyCode::Space, KeyCode::ControlLeft, KeyCode::ControlRight],
};

pub const P2_KEYS: KeyMap = KeyMap {
    up: &[KeyCode::KeyW],
    down: &[KeyCode::KeyS],
    left: &[KeyCode::KeyA],
    right: &[KeyCode::KeyD],
    fire: &[KeyCode::ShiftLeft],
};

/// Input state owned by the game thread.
pub struct InputState {
    pub p1: KeyMap,
    pub p2: KeyMap,

    // Original static/dynamic direction latches.
    staticdir: i32,
    dynamicdir: i32,
    staticdir2: i32,
    dynamicdir2: i32,
    oupressed: bool,
    odpressed: bool,
    olpressed: bool,
    orpressed: bool,
    ou2pressed: bool,
    od2pressed: bool,
    ol2pressed: bool,
    or2pressed: bool,
    keydir: i32,
    keydir2: i32,
    pub firepflag: bool,
    pub fire2pflag: bool,

    // Events collected since the previous poll.
    edges: HashSet<KeyCode>,
    pub chars: Vec<char>,
    pub confirm: bool,
    pub any_key: bool,
    /// A key that should start a game from the title screen (everything except
    /// Esc/N/Q, mirroring the original `checkkeyb`).
    pub start_key: bool,
    pub escape: bool,
    pub pausef: bool,
    pub toggle_music: bool,
    pub toggle_sound: bool,
    pub speed_delta: i64,
    pub quit: bool,
    /// Set by a mouse click? Kept for future menu use.
    pub mouse_click: bool,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            p1: P1_KEYS,
            p2: P2_KEYS,
            staticdir: -1,
            dynamicdir: -1,
            staticdir2: -1,
            dynamicdir2: -1,
            oupressed: false,
            odpressed: false,
            olpressed: false,
            orpressed: false,
            ou2pressed: false,
            od2pressed: false,
            ol2pressed: false,
            or2pressed: false,
            keydir: -1,
            keydir2: -1,
            firepflag: false,
            fire2pflag: false,
            edges: HashSet::new(),
            chars: Vec::new(),
            confirm: false,
            any_key: false,
            start_key: false,
            escape: false,
            pausef: false,
            toggle_music: false,
            toggle_sound: false,
            speed_delta: 0,
            quit: false,
            mouse_click: false,
        }
    }

    /// Drain queued events and update global flags. Called once per game tick
    /// (and by blocking UI waits).
    pub fn poll(&mut self, shared: &SharedInput) {
        // Edges only live for one poll; held keys remain authoritative.
        self.edges.clear();
        self.chars.clear();
        self.confirm = false;
        self.any_key = false;
        self.start_key = false;
        self.escape = false;
        self.pausef = false;
        self.toggle_music = false;
        self.toggle_sound = false;
        self.speed_delta = 0;
        self.quit = false;

        for ev in shared.drain() {
            match ev {
                InputEvent::KeyDown(code) => {
                    self.edges.insert(code);
                    self.any_key = true;
                    if !matches!(
                        code,
                        KeyCode::Escape
                            | KeyCode::KeyN
                            | KeyCode::KeyS
                            | KeyCode::KeyQ
                            | KeyCode::F10
                    ) {
                        self.start_key = true;
                    }
                    match code {
                        KeyCode::Escape | KeyCode::KeyQ | KeyCode::F10 => self.escape = true,
                        KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                            self.confirm = true
                        }
                        KeyCode::KeyP => self.pausef = true,
                        KeyCode::F7 => self.toggle_music = true,
                        KeyCode::F9 => self.toggle_sound = true,
                        KeyCode::Equal | KeyCode::NumpadAdd => self.speed_delta = -10_000,
                        KeyCode::Minus | KeyCode::NumpadSubtract => self.speed_delta = 10_000,
                        _ => {}
                    }
                }
                InputEvent::KeyUp(_) => {}
                InputEvent::Char(c) => {
                    self.chars.push(c);
                    self.any_key = true;
                }
            }
        }
        let pad = shared.pad();
        if pad.start {
            // Start acts like "any key" on the title and like Enter elsewhere.
            self.start_key = true;
            self.confirm = true;
            self.any_key = true;
        }
        // Keep escape/quit consistent.
        self.quit = self.escape;
    }

    fn edge(&self, codes: &[KeyCode]) -> bool {
        codes.iter().any(|c| self.edges.contains(c))
    }

    fn held(&self, shared: &SharedInput, codes: &[KeyCode]) -> bool {
        codes.iter().any(|c| shared.is_held(*c))
    }

    fn pressed(&self, shared: &SharedInput, codes: &[KeyCode]) -> bool {
        self.held(shared, codes) || self.edge(codes)
    }

    /// `readdir(n)` — update the latched direction for player `n` (0 or 1).
    pub fn readdir(&mut self, shared: &SharedInput, n: usize) {
        if n == 0 {
            let keys = self.p1;
            let pad = shared.pad();
            let u = self.pressed(shared, keys.up) || pad.up;
            let d = self.pressed(shared, keys.down) || pad.down;
            let l = self.pressed(shared, keys.left) || pad.left;
            let r = self.pressed(shared, keys.right) || pad.right;

            if u && !self.oupressed {
                self.staticdir = 2;
                self.dynamicdir = 2;
            }
            if d && !self.odpressed {
                self.staticdir = 6;
                self.dynamicdir = 6;
            }
            if l && !self.olpressed {
                self.staticdir = 4;
                self.dynamicdir = 4;
            }
            if r && !self.orpressed {
                self.staticdir = 0;
                self.dynamicdir = 0;
            }
            if (self.oupressed && !u && self.dynamicdir == 2)
                || (self.odpressed && !d && self.dynamicdir == 6)
                || (self.olpressed && !l && self.dynamicdir == 4)
                || (self.orpressed && !r && self.dynamicdir == 0)
            {
                self.dynamicdir = -1;
                if u {
                    self.dynamicdir = 2;
                    self.staticdir = 2;
                }
                if d {
                    self.dynamicdir = 6;
                    self.staticdir = 6;
                }
                if l {
                    self.dynamicdir = 4;
                    self.staticdir = 4;
                }
                if r {
                    self.dynamicdir = 0;
                    self.staticdir = 0;
                }
            }
            self.oupressed = u;
            self.odpressed = d;
            self.olpressed = l;
            self.orpressed = r;
            self.keydir = if self.dynamicdir != -1 {
                self.dynamicdir
            } else {
                self.staticdir
            };
            self.staticdir = -1;

            self.firepflag = self.pressed(shared, keys.fire) || pad.fire;
        } else {
            let keys = self.p2;
            let u = self.pressed(shared, keys.up);
            let d = self.pressed(shared, keys.down);
            let l = self.pressed(shared, keys.left);
            let r = self.pressed(shared, keys.right);

            if u && !self.ou2pressed {
                self.staticdir2 = 2;
                self.dynamicdir2 = 2;
            }
            if d && !self.od2pressed {
                self.staticdir2 = 6;
                self.dynamicdir2 = 6;
            }
            if l && !self.ol2pressed {
                self.staticdir2 = 4;
                self.dynamicdir2 = 4;
            }
            if r && !self.or2pressed {
                self.staticdir2 = 0;
                self.dynamicdir2 = 0;
            }
            if (self.ou2pressed && !u && self.dynamicdir2 == 2)
                || (self.od2pressed && !d && self.dynamicdir2 == 6)
                || (self.ol2pressed && !l && self.dynamicdir2 == 4)
                || (self.or2pressed && !r && self.dynamicdir2 == 0)
            {
                self.dynamicdir2 = -1;
                if u {
                    self.dynamicdir2 = 2;
                    self.staticdir2 = 2;
                }
                if d {
                    self.dynamicdir2 = 6;
                    self.staticdir2 = 6;
                }
                if l {
                    self.dynamicdir2 = 4;
                    self.staticdir2 = 4;
                }
                if r {
                    self.dynamicdir2 = 0;
                    self.staticdir2 = 0;
                }
            }
            self.ou2pressed = u;
            self.od2pressed = d;
            self.ol2pressed = l;
            self.or2pressed = r;
            self.keydir2 = if self.dynamicdir2 != -1 {
                self.dynamicdir2
            } else {
                self.staticdir2
            };
            self.staticdir2 = -1;

            self.fire2pflag = self.pressed(shared, keys.fire);
        }
    }

    /// `getdir(n)`
    pub fn getdir(&self, n: usize) -> i32 {
        if n == 0 {
            self.keydir
        } else {
            self.keydir2
        }
    }

    pub fn getfire(&self, n: usize) -> bool {
        if n == 0 {
            self.firepflag
        } else {
            self.fire2pflag
        }
    }

    pub fn clearfire(&mut self, n: usize) {
        if n == 0 {
            self.firepflag = false;
        } else {
            self.fire2pflag = false;
        }
    }

    pub fn take_chars(&mut self) -> Vec<char> {
        std::mem::take(&mut self.chars)
    }

    /// True if the named key went down since the last poll.
    pub fn key_pressed(&self, key: NamedKey) -> bool {
        let code = match key {
            NamedKey::Escape => KeyCode::Escape,
            NamedKey::Enter => KeyCode::Enter,
            NamedKey::Space => KeyCode::Space,
            _ => return false,
        };
        self.edges.contains(&code)
    }
}

/// Cheap helper used for the two-player-per-keyboard "any key starts" title
/// behaviour.
pub fn any_text_char(chars: &[char]) -> Option<char> {
    chars.iter().copied().find(|c| !c.is_control())
}

// Silence an unused-import warning on platforms/builds where HashMap is not
// needed yet; it is kept for future key-remap support.
#[allow(dead_code)]
type _Unused = HashMap<KeyCode, KeyCode>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pad_controls_player_one() {
        let shared = SharedInput::new();
        shared.set_pad(PadState {
            right: true,
            fire: true,
            ..PadState::default()
        });
        let mut input = InputState::new();
        input.readdir(&shared, 0);
        assert_eq!(input.getdir(0), 0, "pad right should latch DIR_RIGHT");
        assert!(input.getfire(0), "pad fire should be visible");

        shared.set_pad(PadState::default());
        input.readdir(&shared, 0);
        assert!(!input.getfire(0));
    }

    #[test]
    fn pad_start_starts_the_title() {
        let shared = SharedInput::new();
        shared.set_pad(PadState {
            start: true,
            ..PadState::default()
        });
        let mut input = InputState::new();
        input.poll(&shared);
        assert!(input.start_key);
        assert!(input.confirm);
    }
}
