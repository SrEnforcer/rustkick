use std::f32::consts::TAU;

pub struct SineOsc {
    phase: f32,
    sample_rate: f32,
}

impl SineOsc {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            phase: 0.0,
            sample_rate,
        }
    }

    /// Resets the oscillator to a zero-crossing.
    ///
    /// Phase 0 means the first output sample is 0 — no discontinuity, no click.
    /// The onset ramp in the host (lib.rs) handles the initial amplitude ramp
    /// so the attack still feels immediate without a transient spike.
    pub fn reset(&mut self) {
        self.phase = 0.0;
    }

    pub fn tick(&mut self, freq: f32) -> f32 {
        let value = (self.phase * TAU).sin();
        self.phase = (self.phase + freq / self.sample_rate).fract();
        value
    }
}
