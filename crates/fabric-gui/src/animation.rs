//! Animation utilities for smooth UI transitions and liquid glass effects.

/// Persistent animation state carried across frames.
pub struct AnimationState {
    /// Current phase of the pulse animation (radians).
    pub pulse_phase: f32,
    /// Fade-in progress 0.0 -> 1.0.
    pub fade_in: f32,
    /// Smooth interpolation factor for tab transitions.
    pub transition_progress: f32,
    /// Target tab index for smooth switching.
    pub transition_target: usize,
    /// Phase for gradient hue rotation.
    pub gradient_phase: f32,
}

impl AnimationState {
    pub fn new() -> Self {
        Self {
            pulse_phase: 0.0,
            fade_in: 0.0,
            transition_progress: 1.0,
            transition_target: 0,
            gradient_phase: 0.0,
        }
    }

    /// Advance all animations by `dt` seconds.
    pub fn update(&mut self, dt: f32) {
        self.pulse_phase += dt * 3.0;
        if self.pulse_phase > std::f32::consts::TAU {
            self.pulse_phase -= std::f32::consts::TAU;
        }
        self.gradient_phase += dt * 0.5;
        if self.gradient_phase > std::f32::consts::TAU {
            self.gradient_phase -= std::f32::consts::TAU;
        }
        self.fade_in = (self.fade_in + dt * 4.0).min(1.0);
        let speed = dt * 6.0;
        if (self.transition_progress - 1.0).abs() > 0.01 {
            self.transition_progress = (self.transition_progress + speed).min(1.0);
        }
    }

    /// Sinusoidal pulse value in 0.0..=1.0.
    pub fn pulse(&self) -> f32 {
        (self.pulse_phase.sin() * 0.5 + 0.5) * 0.3 + 0.7
    }

    /// Pulsing glow intensity in 0.0..=1.0.
    /// Returns a soft sine wave useful for glow opacity on active elements.
    pub fn pulse_glow(&self) -> f32 {
        self.pulse_phase.sin() * 0.5 + 0.5
    }

    /// Hue rotation factor for animated gradient bars.
    /// Returns a value in 0.0..=1.0 that shifts over time.
    pub fn gradient_shift(&self) -> f32 {
        self.gradient_phase.sin() * 0.5 + 0.5
    }

    /// Ease-in fade value in 0.0..=1.0.
    pub fn fade(&self) -> f32 {
        self.fade_in * self.fade_in
    }

    /// Reset fade-in (e.g. on tab switch).
    pub fn reset_fade(&mut self) {
        self.fade_in = 0.0;
    }

    /// Set a new tab transition target.
    pub fn start_transition(&mut self, target_index: usize) {
        if self.transition_target != target_index {
            self.transition_target = target_index;
            self.transition_progress = 0.0;
        }
    }
}

impl Default for AnimationState {
    fn default() -> Self {
        Self::new()
    }
}
