/// Feed-forward peak compressor operating in the log (dB) domain on the
/// gain-reduction value. Smoothed exponentially with separate attack/release
/// time constants.
///
/// Used as a body shaper *after* the waveshaper and post-EQ: squeezes the
/// distorted high band into a denser, more "glued" body — the pumping
/// character commercial kicks rely on for perceived loudness.
pub struct Compressor {
    /// Current smoothed gain reduction in dB (≥ 0).
    reduction_db: f32,
    attack_coeff: f32,
    release_coeff: f32,
    threshold_db: f32,
    ratio: f32,
    makeup_lin: f32,
}

impl Default for Compressor {
    fn default() -> Self {
        Self {
            reduction_db: 0.0,
            attack_coeff: 0.0,
            release_coeff: 0.0,
            threshold_db: 0.0,
            ratio: 1.0,
            makeup_lin: 1.0,
        }
    }
}

impl Compressor {
    pub fn reset(&mut self) {
        self.reduction_db = 0.0;
    }

    /// Recompute coefficients. Call once per buffer.
    pub fn set_params(
        &mut self,
        threshold_db: f32,
        ratio: f32,
        attack_ms: f32,
        release_ms: f32,
        makeup_db: f32,
        sample_rate: f32,
    ) {
        self.threshold_db = threshold_db;
        self.ratio = ratio.max(1.0);
        self.attack_coeff = (-1.0 / (attack_ms.max(0.01) * 0.001 * sample_rate)).exp();
        self.release_coeff = (-1.0 / (release_ms.max(0.01) * 0.001 * sample_rate)).exp();
        self.makeup_lin = 10.0_f32.powf(makeup_db / 20.0);
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        let abs = x.abs().max(1e-9);
        let x_db = 20.0 * abs.log10();
        let over = x_db - self.threshold_db;
        let target = if over > 0.0 {
            over * (1.0 - 1.0 / self.ratio)
        } else {
            0.0
        };
        let coeff = if target > self.reduction_db {
            self.attack_coeff
        } else {
            self.release_coeff
        };
        self.reduction_db = target + (self.reduction_db - target) * coeff;
        let gain = 10.0_f32.powf(-self.reduction_db / 20.0);
        x * gain * self.makeup_lin
    }
}
