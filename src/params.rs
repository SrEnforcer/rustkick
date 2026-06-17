use crate::dsp::Shaper;
use nih_plug::prelude::*;

#[derive(Params)]
pub struct HardKickParams {
    /// Sweep start frequency in Hz.
    #[id = "pitch_start"]
    pub pitch_start: FloatParam,

    /// Sweep end frequency in Hz.
    #[id = "pitch_end"]
    pub pitch_end: FloatParam,

    /// Envelope decay time in seconds.
    #[id = "decay"]
    pub decay: FloatParam,

    /// Pitch curve exponent — values above 1.0 give a fast initial drop (hardstyle character).
    #[id = "curve"]
    pub curve: FloatParam,

    /// Amplitude decay time in seconds — independent from pitch decay.
    #[id = "amp_decay"]
    pub amp_decay: FloatParam,

    /// Amplitude curve exponent — shapes how the volume fades out.
    #[id = "amp_curve"]
    pub amp_curve: FloatParam,

    /// Master output level.
    #[id = "level"]
    pub level: FloatParam,

    /// Waveshaping model: sets the character of the distortion.
    #[id = "shaper"]
    pub shaper: EnumParam<Shaper>,

    /// Distortion intensity (0.0 = clean, 1.0 = fully driven).
    #[id = "drive"]
    pub drive: FloatParam,

    /// Distortion asymmetry — adds even harmonics for a thicker tone.
    #[id = "bias"]
    pub bias: FloatParam,

    /// Dry/wet ratio of the distortion (0.0 = unprocessed, 1.0 = fully distorted).
    #[id = "dist_mix"]
    pub dist_mix: FloatParam,

    /// Pre-distortion peaking EQ frequency in Hz.
    ///
    /// A narrow boost here fed into the saturator produces the rawstyle "screech":
    /// the non-linearity amplifies only that band, generating overtones around it.
    #[id = "pre_eq_freq"]
    pub pre_eq_freq: FloatParam,

    /// Pre-distortion peaking EQ resonance (Q).
    ///
    /// Higher values narrow the peak. Typical rawstyle settings: 3–8.
    #[id = "pre_eq_q"]
    pub pre_eq_q: FloatParam,

    /// Pre-distortion peaking EQ gain in dB (0 = bypassed).
    #[id = "pre_eq_gain"]
    pub pre_eq_gain: FloatParam,

    /// Post-distortion high-shelf tone control in dB.
    ///
    /// Negative values tame harshness after the waveshaper; positive values
    /// add brightness. Shelf frequency is fixed at 4 kHz.
    #[id = "tone"]
    pub tone: FloatParam,

    /// Transient click ("tok") level, mixed in parallel after the distortion.
    #[id = "click_level"]
    pub click_level: FloatParam,

    /// Transient click decay time in milliseconds — very short for a mechanical attack.
    #[id = "click_decay"]
    pub click_decay: FloatParam,

    /// High-pass cutoff that shapes the click; higher = thinner, more "tok".
    #[id = "click_tone"]
    pub click_tone: FloatParam,

    /// Linkwitz-Riley crossover frequency — signal below this is passed clean to
    /// the output; only the band above it goes through the distortion chain.
    /// Keeps the sub-bass tight and undistorted while the upper body gets driven.
    #[id = "crossover_freq"]
    pub crossover_freq: FloatParam,

    /// Internal sequencer tempo in BPM.
    #[id = "bpm"]
    pub bpm: FloatParam,
}

impl Default for HardKickParams {
    fn default() -> Self {
        Self {
            pitch_start: FloatParam::new(
                "Pitch start",
                150.0,
                FloatRange::Skewed {
                    min: 20.0,
                    max: 800.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(1)),

            pitch_end: FloatParam::new(
                "Pitch end",
                50.0,
                FloatRange::Skewed {
                    min: 20.0,
                    max: 200.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(1)),

            decay: FloatParam::new(
                "Decay",
                0.4,
                FloatRange::Skewed {
                    min: 0.05,
                    max: 2.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_unit(" s")
            .with_value_to_string(formatters::v2s_f32_rounded(2)),

            curve: FloatParam::new("Curve", 2.0, FloatRange::Linear { min: 0.1, max: 8.0 })
                .with_value_to_string(formatters::v2s_f32_rounded(2)),

            amp_decay: FloatParam::new(
                "Amp decay",
                0.5,
                FloatRange::Skewed {
                    min: 0.05,
                    max: 2.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_unit(" s")
            .with_value_to_string(formatters::v2s_f32_rounded(2)),

            amp_curve: FloatParam::new("Amp curve", 1.0, FloatRange::Linear { min: 0.1, max: 8.0 })
                .with_value_to_string(formatters::v2s_f32_rounded(2)),

            level: FloatParam::new("Level", 0.8, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_value_to_string(formatters::v2s_f32_rounded(2)),

            // Default Soft + drive 0.0 keeps the step-1 tone (clean sine) intact;
            // the user dials in the hardness deliberately.
            shaper: EnumParam::new("Shaper", Shaper::Soft),

            drive: FloatParam::new("Drive", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_value_to_string(formatters::v2s_f32_rounded(2)),

            bias: FloatParam::new("Bias", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_value_to_string(formatters::v2s_f32_rounded(2)),

            dist_mix: FloatParam::new("Dist mix", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_value_to_string(formatters::v2s_f32_rounded(2)),

            pre_eq_freq: FloatParam::new(
                "Pre EQ freq",
                2000.0,
                FloatRange::Skewed {
                    min: 200.0,
                    max: 8000.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(1)),

            pre_eq_q: FloatParam::new(
                "Pre EQ Q",
                3.0,
                FloatRange::Skewed {
                    min: 0.5,
                    max: 10.0,
                    factor: FloatRange::skew_factor(1.0),
                },
            )
            .with_value_to_string(formatters::v2s_f32_rounded(1)),

            // Default 0 dB = pre-EQ is inactive until the user raises it.
            pre_eq_gain: FloatParam::new(
                "Pre EQ gain",
                0.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 24.0,
                },
            )
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_rounded(1)),

            // Default 0 dB = tone control is neutral.
            tone: FloatParam::new(
                "Tone",
                0.0,
                FloatRange::Linear {
                    min: -18.0,
                    max: 6.0,
                },
            )
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_rounded(1)),

            // Default 0.0 = no click until the user dials it in.
            click_level: FloatParam::new(
                "Click level",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_value_to_string(formatters::v2s_f32_rounded(2)),

            click_decay: FloatParam::new(
                "Click decay",
                4.0,
                FloatRange::Skewed {
                    min: 0.5,
                    max: 50.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_unit(" ms")
            .with_value_to_string(formatters::v2s_f32_rounded(1)),

            click_tone: FloatParam::new(
                "Click tone",
                2000.0,
                FloatRange::Skewed {
                    min: 200.0,
                    max: 8000.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(1)),

            crossover_freq: FloatParam::new(
                "Crossover",
                150.0,
                FloatRange::Skewed {
                    min: 60.0,
                    max: 400.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(1)),

            bpm: FloatParam::new(
                "BPM",
                150.0,
                FloatRange::Linear {
                    min: 60.0,
                    max: 220.0,
                },
            )
            .with_value_to_string(formatters::v2s_f32_rounded(1)),
        }
    }
}
