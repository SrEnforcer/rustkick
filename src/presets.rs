use crate::dsp::{OsFactor, Shaper};
use crate::params::HardKickParams;
use nih_plug::context::gui::ParamSetter;

/// A complete snapshot of every plugin parameter.
pub struct Preset {
    pub name: &'static str,
    // PITCH
    pub pitch_start: f32,
    pub pitch_end: f32,
    pub decay: f32,
    pub curve: f32,
    // AMPLITUDE
    pub amp_decay: f32,
    pub amp_curve: f32,
    pub level: f32,
    // SHAPING
    pub shaper: Shaper,
    pub drive: f32,
    pub bias: f32,
    pub dist_mix: f32,
    pub crossover_freq: f32,
    // EQ
    pub pre_eq_freq: f32,
    pub pre_eq_q: f32,
    pub pre_eq_gain: f32,
    pub tone: f32,
    // TRANSIENT
    pub click_level: f32,
    pub click_decay: f32,
    pub click_tone: f32,
    // OUTPUT
    pub oversample: OsFactor,
    pub limiter_threshold: f32,
    pub limiter_release: f32,
    // SEQUENCER
    pub bpm: f32,
}

/// Apply all values from a preset to the live parameter set.
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
}

pub const PRESETS: &[Preset] = &[
    // 1. Classic Hardstyle — deep pitch drop, clean sub, soft saturation, punchy click.
    //    The reference point: heavy low end, smooth decay, no screech.
    Preset {
        name: "Classic Hard",
        pitch_start: 180.0,
        pitch_end: 45.0,
        decay: 0.5,
        curve: 3.0,
        amp_decay: 0.6,
        amp_curve: 1.2,
        level: 0.85,
        shaper: Shaper::Soft,
        drive: 0.35,
        bias: 0.0,
        dist_mix: 1.0,
        crossover_freq: 120.0,
        pre_eq_freq: 2000.0,
        pre_eq_q: 3.0,
        pre_eq_gain: 0.0,
        tone: -2.0,
        click_level: 0.4,
        click_decay: 5.0,
        click_tone: 3000.0,
        oversample: OsFactor::X2,
        limiter_threshold: -0.3,
        limiter_release: 100.0,
        bpm: 150.0,
    },

    // 2. Raw Screech — the rawstyle "laser" tone: extreme pre-EQ boost into the
    //    folder, asymmetric bias for even harmonics, crossover low so the sub
    //    survives the carnage.
    Preset {
        name: "Raw Screech",
        pitch_start: 200.0,
        pitch_end: 50.0,
        decay: 0.35,
        curve: 4.5,
        amp_decay: 0.45,
        amp_curve: 1.5,
        level: 0.80,
        shaper: Shaper::Fold,
        drive: 0.75,
        bias: 0.25,
        dist_mix: 1.0,
        crossover_freq: 100.0,
        pre_eq_freq: 2800.0,
        pre_eq_q: 6.0,
        pre_eq_gain: 18.0,
        tone: -3.0,
        click_level: 0.55,
        click_decay: 3.0,
        click_tone: 4000.0,
        oversample: OsFactor::X4,
        limiter_threshold: -0.3,
        limiter_release: 80.0,
        bpm: 150.0,
    },

    // 3. Minimal Tek — short, punchy, lo-fi. Hard clipper, almost no body, all
    //    attack. Works well at higher BPMs.
    Preset {
        name: "Minimal Tek",
        pitch_start: 130.0,
        pitch_end: 55.0,
        decay: 0.18,
        curve: 2.0,
        amp_decay: 0.22,
        amp_curve: 2.0,
        level: 0.90,
        shaper: Shaper::Hard,
        drive: 0.60,
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
    },

    // 4. Sub Pressure — massive sub, very low crossover so almost the whole
    //    signal stays clean, gentle drive only. Designed for maximum low-end
    //    weight in a club system.
    Preset {
        name: "Sub Pressure",
        pitch_start: 150.0,
        pitch_end: 38.0,
        decay: 0.70,
        curve: 2.5,
        amp_decay: 0.80,
        amp_curve: 1.0,
        level: 0.88,
        shaper: Shaper::Soft,
        drive: 0.20,
        bias: 0.0,
        dist_mix: 0.6,
        crossover_freq: 80.0,
        pre_eq_freq: 2000.0,
        pre_eq_q: 3.0,
        pre_eq_gain: 0.0,
        tone: -4.0,
        click_level: 0.20,
        click_decay: 8.0,
        click_tone: 2000.0,
        oversample: OsFactor::X2,
        limiter_threshold: -0.3,
        limiter_release: 150.0,
        bpm: 145.0,
    },

    // 5. Industrial Stomp — asymmetric hard clip, strong bias for a thick,
    //    even-harmonic stack, sharp metallic click. More aggressive than rawstyle.
    Preset {
        name: "Industrial",
        pitch_start: 220.0,
        pitch_end: 55.0,
        decay: 0.28,
        curve: 5.0,
        amp_decay: 0.32,
        amp_curve: 1.8,
        level: 0.82,
        shaper: Shaper::Hard,
        drive: 0.85,
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
    },

    // 6. Floating Bounce — longer decay, slow curve, very soft saturation.
    //    Warmer and more melodic than a typical hard kick; useful as a "colour"
    //    kick underneath a main one or in slower uptempo tracks.
    Preset {
        name: "Floating",
        pitch_start: 160.0,
        pitch_end: 42.0,
        decay: 0.90,
        curve: 1.5,
        amp_decay: 1.10,
        amp_curve: 0.8,
        level: 0.75,
        shaper: Shaper::Soft,
        drive: 0.15,
        bias: 0.0,
        dist_mix: 0.5,
        crossover_freq: 100.0,
        pre_eq_freq: 2000.0,
        pre_eq_q: 3.0,
        pre_eq_gain: 0.0,
        tone: -1.0,
        click_level: 0.10,
        click_decay: 10.0,
        click_tone: 1500.0,
        oversample: OsFactor::X2,
        limiter_threshold: -0.3,
        limiter_release: 200.0,
        bpm: 138.0,
    },
];
