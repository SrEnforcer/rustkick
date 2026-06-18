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

    /// Resets the oscillator phase to π/2 (peak) so the first output sample has
    /// maximum amplitude — this is "phase locking", ensuring every trigger delivers
    /// identical punch regardless of when in the previous cycle the retrigger fires.
    pub fn reset(&mut self) {
        self.phase = 0.25; // 0.25 cycles = π/2 radians = sin peak
    }

    pub fn tick(&mut self, freq: f32) -> f32 {
        let value = (self.phase * TAU).sin();
        self.phase = (self.phase + freq / self.sample_rate).fract();
        value
    }
}
