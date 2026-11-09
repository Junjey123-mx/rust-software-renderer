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
    TogglePause,
    ToggleCamera,
    ToggleLocal,
}

/// Held keys and the action they produce.
pub const HELD: [(KeyboardKey, SolarAction); 6] = [
    (KeyboardKey::KEY_W, SolarAction::Forward),
    (KeyboardKey::KEY_S, SolarAction::Backward),
    (KeyboardKey::KEY_LEFT, SolarAction::YawLeft),
    (KeyboardKey::KEY_RIGHT, SolarAction::YawRight),
    (KeyboardKey::KEY_UP, SolarAction::PitchUp),
    (KeyboardKey::KEY_DOWN, SolarAction::PitchDown),
];

/// Keys that fire once per press.
pub const PRESSED: [(KeyboardKey, SolarAction); 3] = [
    (KeyboardKey::KEY_SPACE, SolarAction::TogglePause),
    (KeyboardKey::KEY_V, SolarAction::ToggleCamera),
    (KeyboardKey::KEY_L, SolarAction::ToggleLocal),
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
