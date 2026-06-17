use nih_plug::prelude::Enum;
use std::f32::consts::{PI, TAU};

/// Oversampling factor for the waveshaper stage.
#[derive(Enum, PartialEq, Eq, Clone, Copy)]
pub enum OsFactor {
    #[id = "off"]
    #[name = "Off"]
    Off,
    #[id = "x2"]
    #[name = "2x"]
    X2,
    #[id = "x4"]
    #[name = "4x"]
    X4,
}

// Prototype lowpass: 32 taps, split into two 16-tap polyphase components.
const PHASE_LEN: usize = 16;
const N_TAPS: usize = PHASE_LEN * 2;

/// A single 2× oversampling stage: a polyphase FIR interpolator (1×→2×) and a
/// matching polyphase FIR decimator (2×→1×). Two stages cascade for 4×.
///
/// All state is fixed-size and stack-allocated — no heap in the audio path.
struct Stage2x {
    up_hist: [f32; PHASE_LEN],
    dn_even: [f32; PHASE_LEN],
    dn_odd: [f32; PHASE_LEN],
}

impl Stage2x {
    fn new() -> Self {
        Self {
            up_hist: [0.0; PHASE_LEN],
            dn_even: [0.0; PHASE_LEN],
            dn_odd: [0.0; PHASE_LEN],
        }
    }

    fn reset(&mut self) {
        self.up_hist = [0.0; PHASE_LEN];
        self.dn_even = [0.0; PHASE_LEN];
        self.dn_odd = [0.0; PHASE_LEN];
    }

    /// Interpolate one input sample into two output samples at twice the rate.
    #[inline]
    fn upsample(&mut self, x: f32, p0: &[f32; PHASE_LEN], p1: &[f32; PHASE_LEN]) -> (f32, f32) {
        self.up_hist.copy_within(0..PHASE_LEN - 1, 1);
        self.up_hist[0] = x;

        let mut o0 = 0.0;
        let mut o1 = 0.0;
        for k in 0..PHASE_LEN {
            o0 += p0[k] * self.up_hist[k];
            o1 += p1[k] * self.up_hist[k];
        }
        (o0, o1)
    }

    /// Decimate the two oversampled samples back into one at the base rate.
    #[inline]
    fn downsample(&mut self, y0: f32, y1: f32, p0: &[f32; PHASE_LEN], p1: &[f32; PHASE_LEN]) -> f32 {
        self.dn_even.copy_within(0..PHASE_LEN - 1, 1);
        self.dn_even[0] = y0;
        self.dn_odd.copy_within(0..PHASE_LEN - 1, 1);
        self.dn_odd[0] = y1;

        let mut o = 0.0;
        for k in 0..PHASE_LEN {
            o += p0[k] * self.dn_even[k] + p1[k] * self.dn_odd[k];
        }
        o
    }
}

/// Anti-aliasing oversampler that runs a nonlinearity at 2× or 4× the base
/// sample rate. The waveshaper generates harmonics above Nyquist that fold back
/// as inharmonic aliasing; processing at a higher rate pushes those products up
/// so the decimation lowpass can remove them before they fold.
pub struct Oversampler {
    // Polyphase coefficients for interpolation (gain ×2) and decimation (gain ×1).
    up_phase0: [f32; PHASE_LEN],
    up_phase1: [f32; PHASE_LEN],
    dn_phase0: [f32; PHASE_LEN],
    dn_phase1: [f32; PHASE_LEN],
    stage_a: Stage2x,
    stage_b: Stage2x,
}

impl Default for Oversampler {
    fn default() -> Self {
        // Design a windowed-sinc lowpass with cutoff at the base-rate Nyquist
        // (0.25 of the 2× rate), Blackman-windowed for strong stopband rejection.
        let mut proto = [0.0_f32; N_TAPS];
        let m = (N_TAPS - 1) as f32;
        let fc = 0.25;
        let mut sum = 0.0;
        for (i, tap) in proto.iter_mut().enumerate() {
            let n = i as f32 - m / 2.0;
            let sinc = if n.abs() < 1e-6 {
                2.0 * fc
            } else {
                (TAU * fc * n).sin() / (PI * n)
            };
            let phase = i as f32 / m;
            let window =
                0.42 - 0.5 * (TAU * phase).cos() + 0.08 * (2.0 * TAU * phase).cos();
            *tap = sinc * window;
            sum += *tap;
        }
        // Normalise so the passband (DC) gain is exactly unity.
        for tap in proto.iter_mut() {
            *tap /= sum;
        }

        let mut up_phase0 = [0.0; PHASE_LEN];
        let mut up_phase1 = [0.0; PHASE_LEN];
        let mut dn_phase0 = [0.0; PHASE_LEN];
        let mut dn_phase1 = [0.0; PHASE_LEN];
        for m in 0..PHASE_LEN {
            // ×2 on the interpolator compensates for the zero-stuffing energy loss.
            up_phase0[m] = proto[2 * m] * 2.0;
            up_phase1[m] = proto[2 * m + 1] * 2.0;
            dn_phase0[m] = proto[2 * m];
            dn_phase1[m] = proto[2 * m + 1];
        }

        Self {
            up_phase0,
            up_phase1,
            dn_phase0,
            dn_phase1,
            stage_a: Stage2x::new(),
            stage_b: Stage2x::new(),
        }
    }
}

impl Oversampler {
    pub fn reset(&mut self) {
        self.stage_a.reset();
        self.stage_b.reset();
    }

    /// Run `f` (the nonlinearity) at the selected oversampling factor.
    ///
    /// At `Off` the function is applied directly. At 2× / 4× the input is
    /// upsampled, `f` is evaluated at every subsample, and the result is
    /// decimated back to the base rate.
    #[inline]
    pub fn process<F: FnMut(f32) -> f32>(&mut self, x: f32, factor: OsFactor, mut f: F) -> f32 {
        let (up0, up1) = (&self.up_phase0, &self.up_phase1);
        let (dn0, dn1) = (&self.dn_phase0, &self.dn_phase1);

        match factor {
            OsFactor::Off => f(x),
            OsFactor::X2 => {
                let (s0, s1) = self.stage_a.upsample(x, up0, up1);
                let y0 = f(s0);
                let y1 = f(s1);
                self.stage_a.downsample(y0, y1, dn0, dn1)
            }
            OsFactor::X4 => {
                // Outer stage: 1×→2×. Inner stage: 2×→4×.
                let (a0, a1) = self.stage_a.upsample(x, up0, up1);
                let (b0, b1) = self.stage_b.upsample(a0, up0, up1);
                let (b2, b3) = self.stage_b.upsample(a1, up0, up1);
                let c0 = f(b0);
                let c1 = f(b1);
                let c2 = f(b2);
                let c3 = f(b3);
                let d0 = self.stage_b.downsample(c0, c1, dn0, dn1);
                let d1 = self.stage_b.downsample(c2, c3, dn0, dn1);
                self.stage_a.downsample(d0, d1, dn0, dn1)
            }
        }
    }
}
