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

    /// Resets the oscillator to a zero-crossing, preventing clicks on retrigger.
    pub fn reset(&mut self) {
        self.phase = 0.0;
    }

    pub fn tick(&mut self, freq: f32) -> f32 {
        let value = (self.phase * TAU).sin();
        self.phase = (self.phase + freq / self.sample_rate).fract();
        value
    }
}
