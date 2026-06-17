use nih_plug::prelude::*;
use nih_plug_egui::EguiState;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

mod dsp;
mod editor;
mod params;

use dsp::{shape, DcBlocker, Envelope, SineOsc};
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
    velocity: f32,
    beat_phase: f32,
    was_playing: bool,
    sample_rate: f32,
    declick_counter: i32,
    pending_trigger: bool,
    pending_velocity: f32,
    dc_blocker: DcBlocker,
}

impl Default for HardKick {
    fn default() -> Self {
        Self {
            params: Arc::new(HardKickParams::default()),
            editor_state: EguiState::from_size(340, 600),
            trigger: Arc::new(AtomicBool::new(false)),
            playing: Arc::new(AtomicBool::new(false)),
            osc: SineOsc::new(44_100.0),
            pitch_env: Envelope::default(),
            amp_env: Envelope::default(),
            velocity: 1.0,
            beat_phase: 0.0,
            was_playing: false,
            sample_rate: 44_100.0,
            declick_counter: 0,
            pending_trigger: false,
            pending_velocity: 1.0,
            dc_blocker: DcBlocker::default(),
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
            self.velocity = velocity;
            self.osc.reset();
            self.pitch_env.trigger();
            self.amp_env.trigger();
        }
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
        self.pitch_env = Envelope::default();
        self.amp_env = Envelope::default();
        self.declick_counter = 0;
        self.pending_trigger = false;
        self.dc_blocker.reset();
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

            // Synthesize.
            let raw = if self.amp_env.is_active() {
                let pitch_t = self
                    .pitch_env
                    .tick(self.params.decay.value() * self.sample_rate);
                let freq = lerp(
                    self.params.pitch_start.value(),
                    self.params.pitch_end.value(),
                    pitch_t.powf(self.params.curve.value()),
                );

                // Waveshape the full-scale oscillator so the harmonic content stays
                // consistent across the kick, then apply the amplitude envelope.
                let osc = self.osc.tick(freq);
                let shaped = shape(
                    osc,
                    self.params.shaper.value(),
                    self.params.drive.value(),
                    self.params.bias.value(),
                );
                let mix = self.params.dist_mix.value();
                let body = self.dc_blocker.process(lerp(osc, shaped, mix));

                let amp_t = self
                    .amp_env
                    .tick(self.params.amp_decay.value() * self.sample_rate);
                let amp = (1.0 - amp_t).powf(self.params.amp_curve.value())
                    * self.velocity
                    * self.params.level.value();

                // Final safety clip; the shapers are already bounded, so this only
                // catches extreme parameter combinations. A proper limiter comes later.
                (body * amp).clamp(-1.0, 1.0)
            } else {
                0.0
            };

            // Declick: linearly fade to zero before firing a pending retrigger.
            let value = if self.declick_counter > 0 {
                let gain = self.declick_counter as f32 / DECLICK_LEN as f32;
                self.declick_counter -= 1;
                if self.declick_counter == 0 && self.pending_trigger {
                    self.pending_trigger = false;
                    self.velocity = self.pending_velocity;
                    self.osc.reset();
                    self.pitch_env.trigger();
                    self.amp_env.trigger();
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
