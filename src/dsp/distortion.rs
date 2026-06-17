use nih_plug::prelude::Enum;

/// Waveshaping model that determines which kind of harmonics the distortion generates.
///
/// The choice between symmetric and asymmetric shaping is the difference between a
/// hollow, square hardstyle tone (odd harmonics) and a richer, more aggressive
/// rawstyle tone (both even and odd harmonics).
#[derive(Enum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shaper {
    /// Symmetric soft clip (`tanh`) — odd harmonics, rounded saturation.
    #[name = "Soft"]
    Soft,
    /// Hard clip — sharp, square distortion with strong odd harmonics.
    #[name = "Hard"]
    Hard,
    /// Wavefolder — folds the signal back to carve metallic overtones (rawstyle).
    #[name = "Fold"]
    Fold,
}

/// Folds the signal back over a threshold an arbitrary number of times.
///
/// Unlike clipping (which caps the peak), a wavefolder mirrors the amplitude,
/// producing a very dense harmonic structure. Adapted from the design report
/// (DSP math of the hardstyle kick).
fn hard_fold(input: f32, threshold: f32) -> f32 {
    let sign = input.signum();
    let x = input.abs();

    if x > threshold {
        let remainder = x % threshold;
        let num_folds = (x / threshold).floor() as i32;
        let y = if num_folds % 2 == 0 {
            remainder
        } else {
            threshold - remainder
        };
        y * sign
    } else {
        input
    }
}

/// Shapes a single sample according to the selected model.
///
/// `drive` (0.0..=1.0) controls the intensity and `bias` (0.0..=1.0) introduces
/// asymmetry. A bias above zero shifts the operating point on the non-linear curve,
/// generating even harmonics for a thicker, warmer tone. The output is always
/// bounded to roughly [-1.0, 1.0].
pub fn shape(x: f32, mode: Shaper, drive: f32, bias: f32) -> f32 {
    // Map drive to a usable input gain.
    let gain = 1.0 + drive * 24.0;
    let driven = x * gain;

    match mode {
        // Asymmetric tanh; the subtraction removes the resting DC offset that the
        // bias introduces (a dedicated DC blocker downstream is still desirable).
        Shaper::Soft => {
            let b = bias * 0.9;
            (driven + b).tanh() - b.tanh()
        }
        // Hard clip with asymmetric thresholds: a positive bias makes the bottom
        // clip sooner than the top, which adds even harmonics.
        Shaper::Hard => (driven + bias).clamp(-1.0, 1.0) - bias.clamp(-1.0, 1.0),
        // Wavefolder around a fixed threshold of 1.0.
        Shaper::Fold => hard_fold(driven, 1.0),
    }
}

/// First-order DC-blocking high-pass (~20 Hz) that removes the DC component
/// introduced by asymmetric distortion.
///
/// Without this filter a static offset would eat headroom and cause floating
/// sub frequencies on a PA system.
pub struct DcBlocker {
    x1: f32,
    y1: f32,
}

impl Default for DcBlocker {
    fn default() -> Self {
        Self { x1: 0.0, y1: 0.0 }
    }
}

impl DcBlocker {
    /// Clears the internal state (on plugin reset).
    pub fn reset(&mut self) {
        self.x1 = 0.0;
        self.y1 = 0.0;
    }

    /// Processes a single sample. The coefficient `R` sets the cutoff frequency;
    /// 0.9995 sits around 20 Hz at 44.1 kHz.
    pub fn process(&mut self, x: f32) -> f32 {
        const R: f32 = 0.9995;
        let y = x - self.x1 + R * self.y1;
        self.x1 = x;
        self.y1 = y;
        y
    }
}
