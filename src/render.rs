use crate::dsp::{shape, BiquadFilter, DcBlocker, Envelope, LRCrossover, Limiter, Oversampler};
use crate::params::HardKickParams;

const SR: f32 = 44_100.0;

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Render a single kick shot from `params` at 44.1 kHz / 32-bit float and
/// write it to `path` as a WAV file.
///
/// Runs entirely outside the process loop — no locks, no real-time constraints.
pub fn export_wav(params: &HardKickParams, path: &str) -> Result<(), String> {
    let amp_samples = (params.amp_decay.value() * SR) as usize;
    // Give 20 ms of tail + limiter lookahead headroom.
    let tail = (SR * 0.02) as usize + crate::dsp::limiter::LOOKAHEAD + 64;
    let n_samples = amp_samples + tail;

    // Fresh DSP state — independent of the live plugin instance.
    let mut phase = 0.0_f32;
    let mut pitch_env = Envelope::default();
    let mut amp_env = Envelope::default();
    let mut pre_eq = BiquadFilter::default();
    let mut post_eq = BiquadFilter::default();
    let mut dc_blocker = DcBlocker::default();
    let mut crossover = LRCrossover::default();
    let mut oversampler = Oversampler::default();
    let mut limiter = Limiter::default();

    // Recompute coefficients (same logic as lib.rs process()).
    pre_eq.set_peaking(
        params.pre_eq_freq.value(),
        params.pre_eq_q.value(),
        params.pre_eq_gain.value(),
        SR,
    );
    post_eq.set_highshelf(4000.0, params.tone.value(), SR);
    crossover.set_freq(params.crossover_freq.value(), SR);
    limiter.set_params(
        params.limiter_threshold.value(),
        params.limiter_release.value(),
        SR,
    );

    pitch_env.trigger();
    amp_env.trigger();

    let shaper = params.shaper.value();
    let drive = params.drive.value();
    let bias = params.bias.value();
    let mix = params.dist_mix.value();
    let level = params.level.value();
    let os = params.oversample.value();

    let mut buf = Vec::with_capacity(n_samples);

    for _ in 0..n_samples {
        let tonal = if amp_env.is_active() {
            let pitch_t = pitch_env.tick(params.decay.value() * SR);
            let shaped_t = pitch_t.powf(params.curve.value());
            let start = params.pitch_start.value();
            let end = params.pitch_end.value();
            let freq = start * (end / start).powf(shaped_t);

            let osc = (phase * std::f32::consts::TAU).sin();
            phase = (phase + freq / SR).fract();

            let (sub, high) = crossover.process(osc);
            let pre = pre_eq.process(high);
            let shaped = oversampler.process(pre, os, |s| shape(s, shaper, drive, bias));
            let driven = post_eq.process(dc_blocker.process(lerp(pre, shaped, mix)));
            let body = sub + driven;

            let amp_t = amp_env.tick(params.amp_decay.value() * SR);
            let amp = (1.0 - amp_t).powf(params.amp_curve.value()) * level;
            body * amp
        } else {
            0.0
        };

        buf.push(limiter.process(tonal));
    }

    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: SR as u32,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };

    let mut writer =
        hound::WavWriter::create(path, spec).map_err(|e| format!("WAV create: {e}"))?;
    for s in &buf {
        writer
            .write_sample(*s)
            .map_err(|e| format!("WAV write: {e}"))?;
    }
    writer.finalize().map_err(|e| format!("WAV finalize: {e}"))?;

    Ok(())
}
