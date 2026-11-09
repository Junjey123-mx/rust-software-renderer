//! Keyboard of the solar scene, private to it so the orbit-camera keys of
//! the earlier scenes keep their meaning there.

use raylib::RaylibHandle;
use raylib::consts::KeyboardKey;

/// One frame of solar-scene input. Held keys repeat every frame; toggles
/// fire once per press.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolarAction {
    Forward,
    Backward,
    YawLeft,
    YawRight,
    PitchUp,
    PitchDown,
    /// Local flight: away from the planet.
    Ascend,
    /// Local flight: towards the planet.
    Descend,
    /// Space flight: light speed while held.
    Boost,
    TogglePause,
    ToggleCamera,
    ToggleLocal,
    /// Quick travel next to the body with this index (F1, F2, F3).
    Warp(usize),
}

/// Held keys and the action they produce.
pub const HELD: [(KeyboardKey, SolarAction); 9] = [
    (KeyboardKey::KEY_W, SolarAction::Forward),
    (KeyboardKey::KEY_S, SolarAction::Backward),
    (KeyboardKey::KEY_LEFT, SolarAction::YawLeft),
    (KeyboardKey::KEY_RIGHT, SolarAction::YawRight),
    (KeyboardKey::KEY_UP, SolarAction::PitchUp),
    (KeyboardKey::KEY_DOWN, SolarAction::PitchDown),
    (KeyboardKey::KEY_E, SolarAction::Ascend),
    (KeyboardKey::KEY_Q, SolarAction::Descend),
    (KeyboardKey::KEY_LEFT_SHIFT, SolarAction::Boost),
];

/// Keys that fire once per press.
pub const PRESSED: [(KeyboardKey, SolarAction); 6] = [
    (KeyboardKey::KEY_SPACE, SolarAction::TogglePause),
    (KeyboardKey::KEY_V, SolarAction::ToggleCamera),
    (KeyboardKey::KEY_L, SolarAction::ToggleLocal),
    (KeyboardKey::KEY_F1, SolarAction::Warp(0)),
    (KeyboardKey::KEY_F2, SolarAction::Warp(1)),
    (KeyboardKey::KEY_F3, SolarAction::Warp(2)),
];

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
