use nih_plug::prelude::Enum;
use std::f32::consts::PI;

/// Waveshaping model for the distortion stage.
#[derive(Enum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shaper {
    /// Asymmetric triode saturation — generates both even and odd harmonics for the
    /// thick, aggressive rawstyle body. Unlike plain tanh (odd-only), even harmonics
    /// add warmth and weight that cuts through a dense mix.
    #[name = "Tube"]
    Tube,
    /// Hard clip — sharp brickwall distortion, strong odd harmonics only.
    #[name = "Hard"]
    Hard,
    /// Sine wavefolder with first-order ADAA — folds the signal back to carve
    /// dense metallic overtones without the aliasing of naive folding.
    #[name = "Fold"]
    Fold,
}

/// Stateless shaper for the editor waveform preview.
/// Does not run ADAA (no state available); accuracy is "good enough" for display.
pub fn shape(x: f32, mode: Shaper, drive: f32, bias: f32) -> f32 {
    let gain = 1.0 + drive * 20.0;
    let driven = x * gain;
    match mode {
        Shaper::Tube => triode(driven, drive, bias),
        Shaper::Hard => (driven + bias * 0.9).clamp(-1.0, 1.0) - (bias * 0.9).clamp(-1.0, 1.0),
        Shaper::Fold => (driven * PI * 0.5).sin(),
    }
}

/// Asymmetric triode saturation model.
///
/// `f(x) = (x-q)/(1-exp(-k(x-q))) + q/(1-exp(k*q))`
///
/// The second term is a DC correction so f(0) = 0 exactly.
/// With q > 0 the curve is asymmetric: positive peaks are amplified more than negative
/// peaks, generating even harmonics alongside the odd ones that any saturator produces.
/// The final tanh bounds the output to (-1, 1).
///
/// `drive` → k  (saturation intensity)
/// `bias`  → q  (operating-point shift, 0 = symmetric)
#[inline]
fn triode(x: f32, drive: f32, bias: f32) -> f32 {
    let k = 1.0 + drive * 14.0;
    let q = bias * 0.7;
    let xq = x - q;

    // Numerically stable body: L'Hopital limit as xq→0 is 1/k.
    let body = if xq.abs() < 1e-5 {
        1.0 / k
    } else {
        xq / (1.0 - (-k * xq).exp())
    };

    // DC correction: limit as q→0 of q/(1-exp(kq)) = -1/k.
    let dc = if q.abs() < 1e-5 {
        -1.0 / k
    } else {
        q / (1.0 - (k * q).exp())
    };

    (body + dc).tanh()
}

/// Antiderivative of the sine wavefolder f(x) = sin(x * π/2):
/// F(x) = -(2/π) * cos(x * π/2)
#[inline]
fn fold_antiderivative(x: f32) -> f32 {
    -(2.0 / PI) * (x * PI * 0.5).cos()
}

/// Stateful waveshaper that applies the selected mode per sample.
///
/// The Fold mode uses first-order Antiderivative Antialiasing (ADAA): instead of
/// evaluating f(x) directly, it computes `(F(x) - F(x_prev)) / (x - x_prev)` where
/// F is the closed-form antiderivative of the folder. This eliminates the aliasing
/// products that would otherwise fold back from above Nyquist, giving cleaner
/// metallic harmonics than naive folding or oversampling, at near-zero extra CPU cost.
pub struct WaveShaper {
    prev_x: f32,
    prev_f: f32,
}

impl Default for WaveShaper {
    fn default() -> Self {
        Self {
            prev_x: 0.0,
            prev_f: fold_antiderivative(0.0),
        }
    }
}

impl WaveShaper {
    pub fn reset(&mut self) {
        self.prev_x = 0.0;
        self.prev_f = fold_antiderivative(0.0);
    }

    #[inline]
    pub fn process(&mut self, x: f32, mode: Shaper, drive: f32, bias: f32) -> f32 {
        let gain = 1.0 + drive * 20.0;
        let driven = x * gain;
        let out = match mode {
            Shaper::Tube => triode(driven, drive, bias),
            Shaper::Hard => {
                let b = bias * 0.9;
                (driven + b).clamp(-1.0, 1.0) - b.clamp(-1.0, 1.0)
            }
            Shaper::Fold => {
                // First-order ADAA.
                let cur_f = fold_antiderivative(driven);
                let dx = driven - self.prev_x;
                let y = if dx.abs() > 1e-5 {
                    (cur_f - self.prev_f) / dx
                } else {
                    // Midpoint fallback when consecutive samples are too close.
                    ((driven + self.prev_x) * PI * 0.25).sin()
                };
                self.prev_x = driven;
                self.prev_f = cur_f;
                y
            }
        };
        if mode != Shaper::Fold {
            self.prev_x = driven;
            self.prev_f = fold_antiderivative(driven);
        }
        out
    }
}

/// First-order DC-blocking high-pass (~20 Hz).
/// Removes the DC offset introduced by asymmetric distortion.
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
    pub fn reset(&mut self) {
        self.x1 = 0.0;
        self.y1 = 0.0;
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        const R: f32 = 0.9995;
        let y = x - self.x1 + R * self.y1;
        self.x1 = x;
        self.y1 = y;
        y
    }
}
