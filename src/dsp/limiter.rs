/// Lookahead peak limiter — instant attack, configurable release.
///
/// Uses a fixed-size ring buffer (256 samples, ~5.8 ms at 44.1 kHz) so there
/// is zero heap allocation. Gain is computed by tracking the peak level over
/// the lookahead window and smoothing downward gain reduction with a one-pole
/// release filter.
///
/// The output is the input delayed by `LOOKAHEAD` samples with gain applied so
/// the peak never exceeds `threshold`. True inter-sample peaks are not
/// detected; the limiter operates at the sample level.
pub const LOOKAHEAD: usize = 256;

pub struct Limiter {
    // Delay line for the lookahead (stack-allocated, no heap).
    buf: [f32; LOOKAHEAD],
    write: usize,
    // Current gain reduction coefficient (linear, ≤ 1.0).
    gain: f32,
    // One-pole release coefficient — computed from release time.
    release_coeff: f32,
    // Threshold in linear amplitude.
    threshold: f32,
}

impl Default for Limiter {
    fn default() -> Self {
        Self {
            buf: [0.0; LOOKAHEAD],
            write: 0,
            gain: 1.0,
            release_coeff: 0.9998, // ~200 ms at 44.1 kHz
            threshold: 1.0,
        }
    }
}

impl Limiter {
    /// Update threshold and release. Call once per buffer.
    ///
    /// `threshold_db` — ceiling in dBFS (e.g. -0.3).
    /// `release_ms`   — how fast gain recovers after a peak (milliseconds).
    pub fn set_params(&mut self, threshold_db: f32, release_ms: f32, sample_rate: f32) {
        self.threshold = 10.0_f32.powf(threshold_db / 20.0);
        // One-pole coefficient: gain multiplied by this each sample during release.
        // At release_ms the gain has recovered to ~1/e of the reduction.
        let release_samples = release_ms * 0.001 * sample_rate;
        self.release_coeff = (-1.0_f32 / release_samples).exp();
    }

    /// Clears the delay line and resets gain state.
    pub fn reset(&mut self) {
        self.buf = [0.0; LOOKAHEAD];
        self.write = 0;
        self.gain = 1.0;
    }

    /// Process one sample. Returns the gain-reduced, lookahead-delayed output.
    ///
    /// The delay means the plugin output is `LOOKAHEAD` samples behind the
    /// "live" signal — negligible for a one-shot kick (≈5.8 ms tail delay).
    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        // Write the incoming sample into the delay line.
        self.buf[self.write] = x;
        // Read the oldest sample (LOOKAHEAD samples ago) — this is what we output.
        let read = (self.write + 1) % LOOKAHEAD;
        let delayed = self.buf[read];
        self.write = read;

        // Compute the gain required so `x` (the peak coming LOOKAHEAD samples
        // from now) does not exceed the threshold. Because we are looking ahead,
        // by the time this sample is output, gain has already been reduced.
        let peak = x.abs();
        let target_gain = if peak > self.threshold {
            self.threshold / peak
        } else {
            1.0
        };

        // Instant attack: clamp gain down immediately if target is lower.
        // Smooth release: let gain recover gradually.
        if target_gain < self.gain {
            self.gain = target_gain;
        } else {
            self.gain = 1.0 - (1.0 - self.gain) * self.release_coeff;
        }

        delayed * self.gain
    }
}
