use crate::dsp::{OsFactor, Shaper};
use crate::params::HardKickParams;
use nih_plug::context::gui::ParamSetter;

pub struct Preset {
    pub name: &'static str,
    pub pitch_start: f32,
    pub pitch_end: f32,
    pub decay: f32,
    pub curve: f32,
    pub amp_attack: f32,
    pub amp_decay: f32,
    pub amp_curve: f32,
    pub level: f32,
    pub shaper: Shaper,
    pub drive: f32,
    pub bias: f32,
    pub dist_mix: f32,
    pub crossover_freq: f32,
    pub pre_eq_freq: f32,
    pub pre_eq_q: f32,
    pub pre_eq_gain: f32,
    pub tone: f32,
    pub click_level: f32,
    pub click_decay: f32,
    pub click_tone: f32,
    pub oversample: OsFactor,
    pub limiter_threshold: f32,
    pub limiter_release: f32,
    pub bpm: f32,
    pub punch_level: f32,
    pub punch_freq: f32,
    pub punch_decay: f32,
    pub punch_curve: f32,
}

pub fn apply(preset: &Preset, params: &HardKickParams, setter: &ParamSetter) {
    macro_rules! set {
        ($param:expr, $val:expr) => {{
            setter.begin_set_parameter(&$param);
            setter.set_parameter(&$param, $val);
            setter.end_set_parameter(&$param);
        }};
    }
    set!(params.pitch_start, preset.pitch_start);
    set!(params.pitch_end, preset.pitch_end);
    set!(params.decay, preset.decay);
    set!(params.curve, preset.curve);
    set!(params.amp_attack, preset.amp_attack);
    set!(params.amp_decay, preset.amp_decay);
    set!(params.amp_curve, preset.amp_curve);
    set!(params.level, preset.level);
    set!(params.shaper, preset.shaper);
    set!(params.drive, preset.drive);
    set!(params.bias, preset.bias);
    set!(params.dist_mix, preset.dist_mix);
    set!(params.crossover_freq, preset.crossover_freq);
    set!(params.pre_eq_freq, preset.pre_eq_freq);
    set!(params.pre_eq_q, preset.pre_eq_q);
    set!(params.pre_eq_gain, preset.pre_eq_gain);
    set!(params.tone, preset.tone);
    set!(params.click_level, preset.click_level);
    set!(params.click_decay, preset.click_decay);
    set!(params.click_tone, preset.click_tone);
    set!(params.oversample, preset.oversample);
    set!(params.limiter_threshold, preset.limiter_threshold);
    set!(params.limiter_release, preset.limiter_release);
    set!(params.bpm, preset.bpm);
    set!(params.punch_level, preset.punch_level);
    set!(params.punch_freq, preset.punch_freq);
    set!(params.punch_decay, preset.punch_decay);
    set!(params.punch_curve, preset.punch_curve);
}

pub const PRESETS: &[Preset] = &[
    // Classic Hardstyle — deep exponential pitch drop, tube saturation for body,
    // moderate click for definition. Sub below 120 Hz stays clean.
    Preset {
        name: "Classic Hard",
        pitch_start: 180.0,
        pitch_end: 45.0,
        decay: 0.5,
        curve: 3.0,
        amp_attack: 1.5,
        amp_decay: 0.6,
        amp_curve: 1.2,
        level: 0.85,
        shaper: Shaper::Tube,
        drive: 0.30,
        bias: 0.15,
        dist_mix: 1.0,
        crossover_freq: 120.0,
        pre_eq_freq: 2000.0,
        pre_eq_q: 3.0,
        pre_eq_gain: 0.0,
        tone: -2.0,
        click_level: 0.40,
        click_decay: 5.0,
        click_tone: 3000.0,
        oversample: OsFactor::X2,
        limiter_threshold: -0.5,
        limiter_release: 100.0,
        bpm: 150.0,
        punch_level: 0.35,
        punch_freq: 220.0,
        punch_decay: 14.0,
        punch_curve: 2.5,
    },

    // Raw Screech — narrow pre-EQ peak boosted into the tube saturator generates
    // the rawstyle screech. High bias adds asymmetric even harmonics for thickness.
    // 4x oversampling keeps the top end clean at this extreme drive.
    Preset {
        name: "Raw Screech",
        pitch_start: 200.0,
        pitch_end: 50.0,
        decay: 0.35,
        curve: 4.5,
        amp_attack: 1.0,
        amp_decay: 0.45,
        amp_curve: 1.5,
        level: 0.80,
        shaper: Shaper::Tube,
        drive: 0.65,
        bias: 0.30,
        dist_mix: 1.0,
        crossover_freq: 100.0,
        pre_eq_freq: 2800.0,
        pre_eq_q: 6.0,
        pre_eq_gain: 16.0,
        tone: -4.0,
        click_level: 0.50,
        click_decay: 3.0,
        click_tone: 4000.0,
        oversample: OsFactor::X4,
        limiter_threshold: -0.5,
        limiter_release: 80.0,
        bpm: 150.0,
        punch_level: 0.45,
        punch_freq: 280.0,
        punch_decay: 10.0,
        punch_curve: 3.0,
    },

    // Minimal Tek — short, punchy, hard-clipped. Almost all attack, minimal tail.
    // Strong click gives the percussive character of tek/hard techno kicks.
    Preset {
        name: "Minimal Tek",
        pitch_start: 130.0,
        pitch_end: 55.0,
        decay: 0.18,
        curve: 2.0,
        amp_attack: 0.5,
        amp_decay: 0.22,
        amp_curve: 2.0,
        level: 0.88,
        shaper: Shaper::Hard,
        drive: 0.55,
        bias: 0.10,
        dist_mix: 1.0,
        crossover_freq: 150.0,
        pre_eq_freq: 1500.0,
        pre_eq_q: 2.0,
        pre_eq_gain: 4.0,
        tone: 1.0,
        click_level: 0.80,
        click_decay: 2.0,
        click_tone: 5000.0,
        oversample: OsFactor::X2,
        limiter_threshold: -0.5,
        limiter_release: 60.0,
        bpm: 160.0,
        punch_level: 0.55,
        punch_freq: 320.0,
        punch_decay: 6.0,
        punch_curve: 3.5,
    },

    // Sub Pressure — maximum low-end weight. Crossover at 80 Hz keeps almost the
    // entire sub range clean; only the high band gets light tube saturation.
    // Long decay, gentle onset, no screech.
    Preset {
        name: "Sub Pressure",
        pitch_start: 150.0,
        pitch_end: 38.0,
        decay: 0.70,
        curve: 2.5,
        amp_attack: 3.0,
        amp_decay: 0.80,
        amp_curve: 1.0,
        level: 0.86,
        shaper: Shaper::Tube,
        drive: 0.15,
        bias: 0.0,
        dist_mix: 0.6,
        crossover_freq: 80.0,
        pre_eq_freq: 2000.0,
        pre_eq_q: 3.0,
        pre_eq_gain: 0.0,
        tone: -4.0,
        click_level: 0.15,
        click_decay: 8.0,
        click_tone: 2000.0,
        oversample: OsFactor::X2,
        limiter_threshold: -0.5,
        limiter_release: 150.0,
        bpm: 145.0,
        punch_level: 0.20,
        punch_freq: 180.0,
        punch_decay: 20.0,
        punch_curve: 2.0,
    },

    // Industrial — asymmetric hard-clip at high bias creates a thick stack of even
    // harmonics. Sharp click, bright tone, aggressive pitch drop. Sounds like it
    // was recorded through a blown speaker.
    Preset {
        name: "Industrial",
        pitch_start: 220.0,
        pitch_end: 55.0,
        decay: 0.28,
        curve: 5.0,
        amp_attack: 1.0,
        amp_decay: 0.32,
        amp_curve: 1.8,
        level: 0.82,
        shaper: Shaper::Hard,
        drive: 0.80,
        bias: 0.40,
        dist_mix: 1.0,
        crossover_freq: 130.0,
        pre_eq_freq: 3500.0,
        pre_eq_q: 4.0,
        pre_eq_gain: 8.0,
        tone: 2.0,
        click_level: 0.70,
        click_decay: 1.5,
        click_tone: 6000.0,
        oversample: OsFactor::X4,
        limiter_threshold: -0.5,
        limiter_release: 50.0,
        bpm: 155.0,
        punch_level: 0.60,
        punch_freq: 350.0,
        punch_decay: 8.0,
        punch_curve: 3.0,
    },

    // Fold Metal — sine wavefolder with ADAA. The folding carves dense metallic
    // overtones; at this drive the kick has a synth-like, almost acid quality.
    // Heavy pre-EQ into the folder for extra screech character.
    Preset {
        name: "Fold Metal",
        pitch_start: 190.0,
        pitch_end: 48.0,
        decay: 0.32,
        curve: 3.5,
        amp_attack: 2.0,
        amp_decay: 0.40,
        amp_curve: 1.6,
        level: 0.80,
        shaper: Shaper::Fold,
        drive: 0.60,
        bias: 0.0,
        dist_mix: 1.0,
        crossover_freq: 110.0,
        pre_eq_freq: 3200.0,
        pre_eq_q: 5.0,
        pre_eq_gain: 12.0,
        tone: -2.0,
        click_level: 0.45,
        click_decay: 4.0,
        click_tone: 3500.0,
        oversample: OsFactor::X4,
        limiter_threshold: -0.5,
        limiter_release: 90.0,
        bpm: 150.0,
        punch_level: 0.40,
        punch_freq: 260.0,
        punch_decay: 12.0,
        punch_curve: 2.8,
    },
];
