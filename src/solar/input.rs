//! Keyboard and mouse of the solar scene, private to it so the
//! orbit-camera keys of the earlier scenes keep their meaning there. The
//! captured mouse steers (the ship under the follow rig, the camera under
//! the orbit rig); the arrows no longer turn anything here, so the two never
//! compete.

use raylib::RaylibHandle;
use raylib::consts::KeyboardKey;
use raylib::math::Vector2;

/// One frame of solar-scene input. Held keys repeat every frame; toggles
/// fire once per press.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolarAction {
    Forward,
    Backward,
    /// Sideways along the ship's right, without turning.
    StrafeLeft,
    StrafeRight,
    /// Scripted turns at the ship's turn rate (recordings and tests); no
    /// key is bound to them, the mouse steers instead.
    YawLeft,
    YawRight,
    PitchUp,
    PitchDown,
    /// Local flight: away from the planet.
    Ascend,
    /// Local flight: towards the planet.
    Descend,
    /// Space flight: light speed, ramped up while held and down after.
    Boost,
    TogglePause,
    ToggleCamera,
    ToggleLocal,
    /// Quick travel next to the body with this index (F1, F2, F3).
    Warp(usize),
}

/// Held keys and the action they produce.
pub const HELD: [(KeyboardKey, SolarAction); 7] = [
    (KeyboardKey::KEY_W, SolarAction::Forward),
    (KeyboardKey::KEY_S, SolarAction::Backward),
    (KeyboardKey::KEY_A, SolarAction::StrafeLeft),
    (KeyboardKey::KEY_D, SolarAction::StrafeRight),
    (KeyboardKey::KEY_E, SolarAction::Ascend),
    (KeyboardKey::KEY_Q, SolarAction::Descend),
    (KeyboardKey::KEY_SPACE, SolarAction::Boost),
];

/// Keys that fire once per press.
pub const PRESSED: [(KeyboardKey, SolarAction); 6] = [
    (KeyboardKey::KEY_P, SolarAction::TogglePause),
    (KeyboardKey::KEY_V, SolarAction::ToggleCamera),
    (KeyboardKey::KEY_L, SolarAction::ToggleLocal),
    (KeyboardKey::KEY_F1, SolarAction::Warp(0)),
    (KeyboardKey::KEY_F2, SolarAction::Warp(1)),
    (KeyboardKey::KEY_F3, SolarAction::Warp(2)),
];

/// Frees the captured mouse or captures a free one.
pub const CAPTURE_KEY: KeyboardKey = KeyboardKey::KEY_TAB;
/// Frees the captured mouse; on a free mouse it quits the viewer.
pub const RELEASE_KEY: KeyboardKey = KeyboardKey::KEY_ESCAPE;

/// Reads this frame's keyboard state into actions.
pub fn poll_solar(rl: &RaylibHandle) -> Vec<SolarAction> {
    let mut actions: Vec<SolarAction> = HELD
        .iter()
        .filter(|(k, _)| rl.is_key_down(*k))
        .map(|(_, a)| *a)
        .collect();
    actions.extend(
        PRESSED
            .iter()
            .filter(|(k, _)| rl.is_key_pressed(*k))
            .map(|(_, a)| *a),
    );
    actions
}

/// Mouse turn of one frame, radians: positive yaw turns left, positive
/// pitch lifts the nose (the same senses as the scripted turns).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Look {
    pub yaw: f32,
    pub pitch: f32,
}

/// How a relative mouse movement becomes a turn: radians per pixel on each
/// axis (applied once per frame and never scaled by the frame time, since
/// the delta already measures the whole movement) and an optional inverted
/// vertical axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MouseLook {
    pub sensitivity_x: f32,
    pub sensitivity_y: f32,
    pub invert_y: bool,
}

impl Default for MouseLook {
    fn default() -> Self {
        Self {
            sensitivity_x: 0.003,
            sensitivity_y: 0.0025,
            invert_y: false,
        }
    }
}

impl MouseLook {
    /// Moving right turns right, moving up lifts the nose (down with
    /// `invert_y`). A non-finite delta turns nothing.
    pub fn turn(&self, delta: Vector2) -> Look {
        if !(delta.x.is_finite() && delta.y.is_finite()) {
            return Look::default();
        }
        let flip = if self.invert_y { -1.0 } else { 1.0 };
        Look {
            yaw: -delta.x * self.sensitivity_x,
            pitch: -delta.y * self.sensitivity_y * flip,
        }
    }
}

/// Whether the mouse steers. The first delta after a capture or after the
/// window regains focus is dropped: it holds the cursor's jump back to the
/// window, not a movement of the player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MouseCapture {
    captured: bool,
    settle: bool,
}

impl MouseCapture {
    pub fn captured(&self) -> bool {
        self.captured
    }

    /// `TAB`: captures a free mouse or frees a captured one; returns
    /// whether it is now captured.
    pub fn toggle(&mut self) -> bool {
        self.captured = !self.captured;
        self.settle = true;
        self.captured
    }

    /// `ESC`: frees the mouse; returns whether it was captured (`ESC` on
    /// a free mouse is the caller's to handle).
    pub fn release(&mut self) -> bool {
        std::mem::replace(&mut self.captured, false)
    }

    /// Turn of this frame from the relative `delta`: nothing while free,
    /// unfocused or settling after a capture or a focus loss.
    pub fn turn(&mut self, delta: Vector2, focused: bool, look: &MouseLook) -> Look {
        if !focused {
            self.settle = true;
            return Look::default();
        }
        if !self.captured || std::mem::take(&mut self.settle) {
            return Look::default();
        }
        look.turn(delta)
    }
}
