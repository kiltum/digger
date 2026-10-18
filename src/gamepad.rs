//! Player-1 gamepad support via `gilrs` (MIT/Apache-2.0).
//!
//! A background thread polls the first connected controller and publishes a
//! small `PadState` to `SharedInput`.  The game thread ORs it into player 1's
//! direction and fire latches, so keyboard and pad can be used together.
//! Player 2 gamepad support is intentionally not implemented yet.

use std::sync::Arc;
use std::time::Duration;

use gilrs::{Axis, Button, Gilrs};

use crate::input::{PadState, SharedInput};

/// Start the gamepad polling thread.  Failure to initialise the backend is
/// logged once and never affects the game.
pub fn spawn(shared: Arc<SharedInput>) {
    let _ = std::thread::Builder::new()
        .name("digger-gamepad".into())
        .spawn(move || {
            let mut gilrs = match Gilrs::new() {
                Ok(g) => g,
                Err(e) => {
                    eprintln!("gamepad: backend unavailable: {e}");
                    return;
                }
            };
            let mut last = PadState::default();
            loop {
                // Drain events so gilrs's internal gamepad state is current.
                while gilrs.next_event().is_some() {}
                let state = read_pad(&gilrs);
                if state != last {
                    shared.set_pad(state);
                    last = state;
                }
                std::thread::sleep(Duration::from_millis(8));
            }
        });
}

fn read_pad(gilrs: &Gilrs) -> PadState {
    let mut pad = PadState::default();
    let Some((_id, gamepad)) = gilrs.gamepads().find(|(_, g)| g.is_connected()) else {
        return pad;
    };

    // Analogue sticks with a generous dead zone, plus the D-pad.
    const DEAD: f32 = 0.45;
    let x = gamepad
        .axis_data(Axis::LeftStickX)
        .map(|a| a.value())
        .unwrap_or(0.0);
    let y = gamepad
        .axis_data(Axis::LeftStickY)
        .map(|a| a.value())
        .unwrap_or(0.0);

    pad.left = x < -DEAD || gamepad.is_pressed(Button::DPadLeft);
    pad.right = x > DEAD || gamepad.is_pressed(Button::DPadRight);
    // gilrs uses the conventional -1 = up, +1 = down for stick Y.
    pad.up = y < -DEAD || gamepad.is_pressed(Button::DPadUp);
    pad.down = y > DEAD || gamepad.is_pressed(Button::DPadDown);

    pad.fire = gamepad.is_pressed(Button::South)
        || gamepad.is_pressed(Button::East)
        || gamepad.is_pressed(Button::RightTrigger2)
        || gamepad.is_pressed(Button::LeftTrigger2);
    pad.start = gamepad.is_pressed(Button::Start);
    pad
}
