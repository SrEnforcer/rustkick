use std::f32::consts::TAU;

/// Biquad filter in Direct Form II Transposed — numerically stable under
/// high-Q and high-gain settings.
///
/// Supports peaking EQ and high-shelf modes. Coefficients are recomputed
/// lazily via `set_peaking` / `set_highshelf` and cached until changed.
pub struct BiquadFilter {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    // Delay elements.
    z1: f32,
    z2: f32,
}

impl Default for BiquadFilter {
    fn default() -> Self {
        // Identity (pass-through) on construction.
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            z1: 0.0,
            z2: 0.0,
        }
    }
}

impl BiquadFilter {
    /// Clears the delay line (call on plugin reset / retrigger if needed).
    pub fn reset(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }

    /// Peaking EQ band — boosts/cuts `gain_db` around `freq` with bandwidth `q`.
    ///
    /// A high Q (e.g. 3–8) with a strong boost fed into a distortion stage is
    /// what produces the rawstyle "screech": the saturator amplifies only that
    /// narrow band nonlinearly.
    pub fn set_peaking(&mut self, freq: f32, q: f32, gain_db: f32, sample_rate: f32) {
        let a = 10.0_f32.powf(gain_db / 40.0);
        let w0 = TAU * freq / sample_rate;
        let alpha = w0.sin() / (2.0 * q);

        let b0 = 1.0 + alpha * a;
        let b1 = -2.0 * w0.cos();
        let b2 = 1.0 - alpha * a;
        let a0 = 1.0 + alpha / a;
        let a1 = -2.0 * w0.cos();
        let a2 = 1.0 - alpha / a;

        self.b0 = b0 / a0;
        self.b1 = b1 / a0;
        self.b2 = b2 / a0;
        self.a1 = a1 / a0;
        self.a2 = a2 / a0;
    }

    /// High-shelf filter — boosts/cuts everything above `freq` by `gain_db`.
    ///
    /// Used as a post-distortion "Tone" control to rein in harshness.
    pub fn set_highshelf(&mut self, freq: f32, gain_db: f32, sample_rate: f32) {
        let a = 10.0_f32.powf(gain_db / 40.0);
        let w0 = TAU * freq / sample_rate;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        // S = 1 (shelf slope = 1 gives a smooth transition).
        let alpha = sin_w0 / 2.0 * ((a + 1.0 / a) * (1.0 - 1.0) + 2.0).sqrt();
        // Fallback to a gentle slope when alpha comes out ≤ 0.
        let alpha = if alpha > 0.0 { alpha } else { sin_w0 * 0.5 };

        let b0 = a * ((a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * a.sqrt() * alpha);
        let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0);
        let b2 = a * ((a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * a.sqrt() * alpha);
        let a0 = (a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * a.sqrt() * alpha;
        let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_w0);
        let a2 = (a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * a.sqrt() * alpha;

        self.b0 = b0 / a0;
        self.b1 = b1 / a0;
        self.b2 = b2 / a0;
        self.a1 = a1 / a0;
        self.a2 = a2 / a0;
    }

    /// High-pass filter (RBJ cookbook) at `freq` with resonance `q`.
    ///
    /// Used to shape the transient click so only its high-frequency "tok"
    /// character remains and it doesn't muddy the sub.
    pub fn set_highpass(&mut self, freq: f32, q: f32, sample_rate: f32) {
        let w0 = TAU * freq / sample_rate;
        let cos_w0 = w0.cos();
        let alpha = w0.sin() / (2.0 * q);

        let b0 = (1.0 + cos_w0) / 2.0;
        let b1 = -(1.0 + cos_w0);
        let b2 = (1.0 + cos_w0) / 2.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;

        self.b0 = b0 / a0;
        self.b1 = b1 / a0;
        self.b2 = b2 / a0;
        self.a1 = a1 / a0;
        self.a2 = a2 / a0;
    }

    /// Processes a single sample through the biquad.
    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.z1;
        self.z1 = self.b1 * x - self.a1 * y + self.z2;
        self.z2 = self.b2 * x - self.a2 * y;
        y
    }

    /// 2nd-order Butterworth lowpass (RBJ cookbook, Q = 1/√2 ≈ 0.7071).
    pub fn set_lowpass_butterworth(&mut self, freq: f32, sample_rate: f32) {
        let w0 = TAU * freq / sample_rate;
        let cos_w0 = w0.cos();
        let alpha = w0.sin() / (2.0_f32.sqrt());

        let b0 = (1.0 - cos_w0) / 2.0;
        let b1 = 1.0 - cos_w0;
        let b2 = (1.0 - cos_w0) / 2.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;

        self.b0 = b0 / a0;
        self.b1 = b1 / a0;
        self.b2 = b2 / a0;
        self.a1 = a1 / a0;
        self.a2 = a2 / a0;
    }

    /// 2nd-order Butterworth highpass (RBJ cookbook, Q = 1/√2 ≈ 0.7071).
    pub fn set_highpass_butterworth(&mut self, freq: f32, sample_rate: f32) {
        let w0 = TAU * freq / sample_rate;
        let cos_w0 = w0.cos();
        let alpha = w0.sin() / (2.0_f32.sqrt());

        let b0 = (1.0 + cos_w0) / 2.0;
        let b1 = -(1.0 + cos_w0);
        let b2 = (1.0 + cos_w0) / 2.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;

        self.b0 = b0 / a0;
        self.b1 = b1 / a0;
        self.b2 = b2 / a0;
        self.a1 = a1 / a0;
        self.a2 = a2 / a0;
    }
}

/// 4th-order Linkwitz-Riley crossover — two cascaded 2nd-order Butterworth
/// sections per band. The outputs sum to unity gain at all frequencies, with
/// exactly 6 dB of attenuation at the crossover point for each band.
pub struct LRCrossover {
    lp1: BiquadFilter,
    lp2: BiquadFilter,
    hp1: BiquadFilter,
    hp2: BiquadFilter,
}

impl Default for LRCrossover {
    fn default() -> Self {
        Self {
            lp1: BiquadFilter::default(),
            lp2: BiquadFilter::default(),
            hp1: BiquadFilter::default(),
            hp2: BiquadFilter::default(),
        }
    }
}

impl LRCrossover {
    /// Update coefficients for the given crossover frequency. Call once per
    /// buffer whenever the frequency parameter changes.
    pub fn set_freq(&mut self, freq: f32, sample_rate: f32) {
        self.lp1.set_lowpass_butterworth(freq, sample_rate);
        self.lp2.set_lowpass_butterworth(freq, sample_rate);
        self.hp1.set_highpass_butterworth(freq, sample_rate);
        self.hp2.set_highpass_butterworth(freq, sample_rate);
    }

    /// Clears all delay lines.
    pub fn reset(&mut self) {
        self.lp1.reset();
        self.lp2.reset();
        self.hp1.reset();
        self.hp2.reset();
    }

    /// Split one sample into `(sub, high)` bands.
    #[inline]
    pub fn process(&mut self, x: f32) -> (f32, f32) {
        let sub = self.lp2.process(self.lp1.process(x));
        let high = self.hp2.process(self.hp1.process(x));
        (sub, high)
    }
}
