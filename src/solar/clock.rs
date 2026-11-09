//! The single source of simulated time of the solar scene.

/// Longest real step applied per frame, so a stall never jumps the simulation.
pub const MAX_STEP: f32 = 0.1;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SimulationClock {
    /// Accumulated simulated seconds.
    pub time: f32,
    /// Simulated seconds added by the last `advance` (0 while paused).
    pub delta: f32,
    /// Clamped real seconds of the last `advance`, for flight controls that
    /// keep working while the simulation is paused.
    pub real_delta: f32,
    pub paused: bool,
    /// Simulated seconds per real second; finite and non-negative.
    pub speed: f32,
}

impl Default for SimulationClock {
    fn default() -> Self {
        Self::new(0.0)
    }
}

impl SimulationClock {
    pub fn new(time: f32) -> Self {
        Self {
            time,
            delta: 0.0,
            real_delta: 0.0,
            paused: false,
            speed: 1.0,
        }
    }

    /// Advances by a real frame delta: non-finite or negative deltas count
    /// as zero, and the step is clamped to `MAX_STEP` before scaling.
    pub fn advance(&mut self, real_dt: f32) {
        let real = if real_dt.is_finite() {
            real_dt.clamp(0.0, MAX_STEP)
        } else {
            0.0
        };
        self.real_delta = real;
        self.delta = if self.paused { 0.0 } else { real * self.speed };
        self.time += self.delta;
    }

    pub fn toggle_pause(&mut self) -> bool {
        self.paused = !self.paused;
        self.paused
    }

    /// Sets the speed; invalid values are ignored.
    pub fn set_speed(&mut self, speed: f32) {
        if speed.is_finite() && speed >= 0.0 {
            self.speed = speed;
        }
    }
}
