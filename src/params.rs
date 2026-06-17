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
