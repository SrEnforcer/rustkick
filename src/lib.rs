use nih_plug::prelude::*;
use nih_plug_egui::EguiState;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

mod dsp;
mod editor;
mod params;
mod presets;
mod render;

use dsp::{
    BiquadFilter, DcBlocker, Envelope, LRCrossover, Limiter, Noise, Oversampler, SineOsc,
    WaveShaper,
};
use params::HardKickParams;

// ~1.5 ms at 44.1 kHz — long enough to be click-free, short enough to be inaudible.
const DECLICK_LEN: i32 = 64;

pub struct HardKick {
    params: Arc<HardKickParams>,
    editor_state: Arc<EguiState>,
    trigger: Arc<AtomicBool>,
    playing: Arc<AtomicBool>,
    osc: SineOsc,
    pitch_env: Envelope,
    amp_env: Envelope,
    punch_osc: SineOsc,
    punch_env: Envelope,
    velocity: f32,
    beat_phase: f32,
    was_playing: bool,
    sample_rate: f32,
    declick_counter: i32,
    pending_trigger: bool,
    pending_velocity: f32,
    dc_blocker: DcBlocker,
    pre_eq: BiquadFilter,
    post_eq: BiquadFilter,
    noise: Noise,
    click_env: Envelope,
    click_hp: BiquadFilter,
    crossover: LRCrossover,
    limiter: Limiter,
    oversampler: Oversampler,
    wave_shaper: WaveShaper,
    // Onset ramp — fades in amplitude over `onset_total` samples on each trigger,
    // preventing the click caused by an instantaneous jump from silence to full gain.
    onset_pos: u32,
    onset_total: u32,
}

impl Default for HardKick {
    fn default() -> Self {
        Self {
            params: Arc::new(HardKickParams::default()),
            editor_state: EguiState::from_size(780, 480),
            trigger: Arc::new(AtomicBool::new(false)),
            playing: Arc::new(AtomicBool::new(false)),
            osc: SineOsc::new(44_100.0),
            pitch_env: Envelope::default(),
            amp_env: Envelope::default(),
            punch_osc: SineOsc::new(44_100.0),
            punch_env: Envelope::default(),
            velocity: 1.0,
            beat_phase: 0.0,
            was_playing: false,
            sample_rate: 44_100.0,
            declick_counter: 0,
            pending_trigger: false,
            pending_velocity: 1.0,
            dc_blocker: DcBlocker::default(),
            pre_eq: BiquadFilter::default(),
            post_eq: BiquadFilter::default(),
            noise: Noise::default(),
            click_env: Envelope::default(),
            click_hp: BiquadFilter::default(),
            crossover: LRCrossover::default(),
            limiter: Limiter::default(),
            oversampler: Oversampler::default(),
            wave_shaper: WaveShaper::default(),
            onset_pos: 0,
            onset_total: 0,
        }
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

impl HardKick {
    /// Fires a trigger, or — if the envelope is still active — starts a short linear fade-out
    /// first to prevent the click caused by an abrupt amplitude discontinuity on retrigger.
    fn schedule_trigger(&mut self, velocity: f32) {
        if self.amp_env.is_active() {
            if self.declick_counter == 0 {
                self.declick_counter = DECLICK_LEN;
            }
            self.pending_trigger = true;
            self.pending_velocity = velocity;
        } else {
            self.fire(velocity);
        }
    }

    /// Resets all DSP state and (re)triggers all envelopes for a new kick.
    fn fire(&mut self, velocity: f32) {
        self.velocity = velocity;
        self.osc.reset();
        self.punch_osc.reset();
        self.pitch_env.trigger();
        self.amp_env.trigger();
        self.click_env.trigger();
        self.punch_env.trigger();
        // Clear filter delay lines so stale state from the previous note's tail
        // doesn't cause a discontinuity (pop/crack) at the start of the new note.
        self.crossover.reset();
        self.pre_eq.reset();
        self.dc_blocker.reset();
        self.post_eq.reset();
        self.click_hp.reset();
        self.wave_shaper.reset();
        self.oversampler.reset();
        // Capture the onset ramp length at trigger time so a mid-note param change
        // doesn't corrupt an in-progress ramp.
        let attack_ms = self.params.amp_attack.value();
        self.onset_total = (attack_ms * 0.001 * self.sample_rate).max(1.0) as u32;
        self.onset_pos = 0;
    }
}

impl Plugin for HardKick {
    const NAME: &'static str = "HardKick";
    const VENDOR: &'static str = "bcktrck";
    const URL: &'static str = "https://bcktrck.nl";
    const EMAIL: &'static str = "info@bcktrck.nl";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: None,
        main_output_channels: Some(new_nonzero_u32(2)),
        ..AudioIOLayout::const_default()
    }];

    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        editor::create(
            self.params.clone(),
            self.editor_state.clone(),
            self.trigger.clone(),
            self.playing.clone(),
        )
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.sample_rate = buffer_config.sample_rate;
        self.osc = SineOsc::new(self.sample_rate);
        true
    }

    fn reset(&mut self) {
        self.osc.reset();
        self.punch_osc.reset();
        self.pitch_env = Envelope::default();
        self.amp_env = Envelope::default();
        self.punch_env = Envelope::default();
        self.declick_counter = 0;
        self.pending_trigger = false;
        self.dc_blocker.reset();
        self.pre_eq.reset();
        self.post_eq.reset();
        self.click_env = Envelope::default();
        self.click_hp.reset();
        self.crossover.reset();
        self.limiter.reset();
        self.oversampler.reset();
        self.wave_shaper.reset();
        self.onset_pos = 0;
        self.onset_total = 0;
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let playing = self.playing.load(Ordering::Relaxed);
        let beat_samples = 60.0 / self.params.bpm.value() * self.sample_rate;

        let manual_trigger = self.trigger.swap(false, Ordering::Relaxed);
        let rising_edge = playing && !self.was_playing;

        // Recompute EQ coefficients once per buffer — cheap and avoids per-sample branches.
        self.pre_eq.set_peaking(
            self.params.pre_eq_freq.value(),
            self.params.pre_eq_q.value(),
            self.params.pre_eq_gain.value(),
            self.sample_rate,
        );
        self.post_eq
            .set_highshelf(4000.0, self.params.tone.value(), self.sample_rate);
        self.click_hp
            .set_highpass(self.params.click_tone.value(), 0.707, self.sample_rate);
        self.crossover
            .set_freq(self.params.crossover_freq.value(), self.sample_rate);
        self.limiter.set_params(
            self.params.limiter_threshold.value(),
            self.params.limiter_release.value(),
            self.sample_rate,
        );

        if !playing {
            self.beat_phase = 0.0;
        }
        self.was_playing = playing;

        let mut next_event = context.next_event();

        for (sample_id, channel_samples) in buffer.iter_samples().enumerate() {
            // Block-level triggers are applied at the first sample of each buffer.
            if sample_id == 0 {
                if manual_trigger {
                    self.schedule_trigger(1.0);
                }
                if rising_edge {
                    self.schedule_trigger(1.0);
                    self.beat_phase = 0.0;
                }
            }

            // Sequencer beat clock.
            if playing {
                self.beat_phase += 1.0;
                if self.beat_phase >= beat_samples {
                    self.beat_phase -= beat_samples;
                    self.schedule_trigger(1.0);
                }
            }

            // MIDI events.
            while let Some(event) = next_event {
                if event.timing() != sample_id as u32 {
                    break;
                }
                if let NoteEvent::NoteOn { velocity, .. } = event {
                    self.schedule_trigger(velocity);
                }
                next_event = context.next_event();
            }

            // Tonal body.
            let tonal = if self.amp_env.is_active() {
                let pitch_t = self
                    .pitch_env
                    .tick(self.params.decay.value() * self.sample_rate);
                // Exponential (log-domain) pitch sweep: interpolating geometrically
                // rather than linearly in Hz gives a natural, non-floaty glide that
                // matches how pitch is perceived (octaves are ratios, not offsets).
                let shaped_t = pitch_t.powf(self.params.curve.value());
                let start = self.params.pitch_start.value();
                let end = self.params.pitch_end.value();
                let freq = start * (end / start).powf(shaped_t);

                // Punch layer — short tonal burst summed into the oscillator
                // before the crossover so it can ride through the distortion
                // chain together with the upper band of the body.
                let punch = if self.punch_env.is_active() {
                    let pd = self.params.punch_decay.value() * 0.001 * self.sample_rate;
                    let pt = self.punch_env.tick(pd);
                    let env = (1.0 - pt).powf(self.params.punch_curve.value());
                    self.punch_osc.tick(self.params.punch_freq.value())
                        * env
                        * self.params.punch_level.value()
                } else {
                    0.0
                };

                // LR crossover splits the oscillator into sub and high bands.
                // The sub band is passed clean; only the high band goes through
                // the distortion chain so the sub-bass stays tight and undistorted.
                let osc = self.osc.tick(freq) + punch;
                let (sub, high) = self.crossover.process(osc);
                let pre = self.pre_eq.process(high);
                let shaper = self.params.shaper.value();
                let drive = self.params.drive.value();
                let bias = self.params.bias.value();
                // Fold mode uses ADAA inside WaveShaper (no oversampling needed there).
                // Tube and Hard still benefit from oversampling at extreme drive settings.
                let shaped = self.oversampler.process(
                    pre,
                    self.params.oversample.value(),
                    |s| self.wave_shaper.process(s, shaper, drive, bias),
                );
                let mix = self.params.dist_mix.value();
                let driven = self
                    .post_eq
                    .process(self.dc_blocker.process(lerp(pre, shaped, mix)));
                let body = sub + driven;

                let amp_t = self
                    .amp_env
                    .tick(self.params.amp_decay.value() * self.sample_rate);
                let amp = (1.0 - amp_t).powf(self.params.amp_curve.value())
                    * self.velocity
                    * self.params.level.value();

                body * amp
            } else {
                0.0
            };

            // Transient "tok" click — a short filtered noise burst mixed in parallel
            // *after* the distortion (mixdown-after), so it keeps its mechanical
            // definition instead of being smeared by the saturator.
            let click = if self.click_env.is_active() {
                let click_samples = self.params.click_decay.value() * 0.001 * self.sample_rate;
                let ct = self.click_env.tick(click_samples);
                let n = self.click_hp.process(self.noise.next_sample());
                n * (1.0 - ct)
                    * self.velocity
                    * self.params.click_level.value()
                    * self.params.level.value()
            } else {
                0.0
            };

            // Onset ramp — linear fade-in over amp_attack samples to prevent
            // the click caused by an instantaneous jump from silence to full amplitude.
            let onset_gain = if self.onset_pos < self.onset_total {
                let g = self.onset_pos as f32 / self.onset_total as f32;
                self.onset_pos += 1;
                g
            } else {
                1.0
            };

            // Brickwall limiter — lookahead peak detection, instant attack,
            // configurable release.
            let raw = self.limiter.process((tonal + click) * onset_gain);

            // Declick: linearly fade to zero before firing a pending retrigger.
            let value = if self.declick_counter > 0 {
                let gain = self.declick_counter as f32 / DECLICK_LEN as f32;
                self.declick_counter -= 1;
                if self.declick_counter == 0 && self.pending_trigger {
                    self.pending_trigger = false;
                    self.fire(self.pending_velocity);
                }
                raw * gain
            } else {
                raw
            };

            for sample in channel_samples {
                *sample = value;
            }
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for HardKick {
    const CLAP_ID: &'static str = "nl.bcktrck.hardkick";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Hardstyle kick synthesizer");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::Instrument,
        ClapFeature::Synthesizer,
        ClapFeature::Mono,
    ];
}

impl Vst3Plugin for HardKick {
    const VST3_CLASS_ID: [u8; 16] = *b"bcktrckHardKick1";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Instrument, Vst3SubCategory::Synth];
}

nih_export_clap!(HardKick);
nih_export_vst3!(HardKick);
