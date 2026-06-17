use nih_plug::prelude::*;
use nih_plug_egui::EguiState;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

mod dsp;
mod editor;
mod params;

use dsp::{Envelope, SineOsc};
use params::HardKickParams;

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
}

impl Default for HardKick {
    fn default() -> Self {
        Self {
            params: Arc::new(HardKickParams::default()),
            editor_state: EguiState::from_size(340, 380),
            trigger: Arc::new(AtomicBool::new(false)),
            playing: Arc::new(AtomicBool::new(false)),
            osc: SineOsc::new(44_100.0),
            pitch_env: Envelope::default(),
            amp_env: Envelope::default(),
            velocity: 1.0,
            beat_phase: 0.0,
            was_playing: false,
            sample_rate: 44_100.0,
        }
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
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
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        if self.trigger.swap(false, Ordering::Relaxed) {
            self.velocity = 1.0;
            self.osc.reset();
            self.pitch_env.trigger();
            self.amp_env.trigger();
        }

        let playing = self.playing.load(Ordering::Relaxed);
        let beat_samples = 60.0 / self.params.bpm.value() * self.sample_rate;

        // Rising edge: fire immediately when play starts.
        if playing && !self.was_playing {
            self.velocity = 1.0;
            self.osc.reset();
            self.pitch_env.trigger();
            self.amp_env.trigger();
            self.beat_phase = 0.0;
        }
        if !playing {
            self.beat_phase = 0.0;
        }
        self.was_playing = playing;

        let mut next_event = context.next_event();

        for (sample_id, channel_samples) in buffer.iter_samples().enumerate() {
            // Sequencer beat clock.
            if playing {
                self.beat_phase += 1.0;
                if self.beat_phase >= beat_samples {
                    self.beat_phase -= beat_samples;
                    self.velocity = 1.0;
                    self.osc.reset();
                    self.pitch_env.trigger();
                    self.amp_env.trigger();
                }
            }

            while let Some(event) = next_event {
                if event.timing() != sample_id as u32 {
                    break;
                }

                if let NoteEvent::NoteOn { velocity, .. } = event {
                    self.velocity = velocity;
                    self.osc.reset();
                    self.pitch_env.trigger();
                    self.amp_env.trigger();
                }

                next_event = context.next_event();
            }

            let value = if self.amp_env.is_active() {
                let pitch_t = self.pitch_env.tick(self.params.decay.value() * self.sample_rate);
                let freq = lerp(
                    self.params.pitch_start.value(),
                    self.params.pitch_end.value(),
                    pitch_t.powf(self.params.curve.value()),
                );

                let amp_t = self.amp_env.tick(self.params.amp_decay.value() * self.sample_rate);
                let amp = (1.0 - amp_t).powf(self.params.amp_curve.value())
                    * self.velocity
                    * self.params.level.value();

                self.osc.tick(freq) * amp
            } else {
                0.0
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
