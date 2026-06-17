# hardkick — milestone 1: sine oscillator met toonhoogte-envelope

Agentinstructie voor het opzetten van de eerste werkende versie van een
hardstyle-kick-synthesizer in Rust met `nih-plug`, draaiend in een Windows
WSL2 (Ubuntu) omgeving.

Doel van deze milestone: een triggerbare, dalende sinussweep. Geen distortion,
geen GUI-maatwerk, geen extra lagen — alleen een schone gepitchte sinus die op
een MIDI-noot reageert. Alles wat later komt (waveshaping, click-laag, limiter)
bouwt hierop voort.

---

## Omgeving voorbereiden (WSL2 Ubuntu)

```bash
# Build-tools en audio-headers
sudo apt update
sudo apt install build-essential pkg-config libssl-dev libasound2-dev -y

# Rust via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Controleren
rustc --version
cargo --version
```

**Let op audio in WSL2:** geluid via de standalone-binary werkt alleen met een
recente WSLg-audio-passthrough. Controleer in PowerShell met `wsl --version` of
je WSL 2.0+ draait; werk de omgeving zo nodig bij met `wsl --update`. Compileert
de binary wel maar hoor je niets, dan is dit vrijwel altijd de oorzaak — niet de
code.

---

## Project opzetten

```bash
cargo new --lib hardkick
cd hardkick
```

### `Cargo.toml`

```toml
[package]
name = "hardkick"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "lib"]

[[bin]]
name = "hardkick-standalone"
path = "src/main.rs"

[dependencies]
nih_plug = { git = "https://github.com/robbert-vdh/nih-plug", features = [
    "assert_process_alloc",
    "standalone",
] }

[profile.release]
opt-level = 3
lto = "thin"
```

---

## Bestandsstructuur

```
hardkick/
├── Cargo.toml
└── src/
    ├── lib.rs          # Plugin-entrypoint en process-lus
    ├── main.rs         # Standalone-binary
    ├── params.rs       # Parameterdefinities
    └── dsp/
        ├── mod.rs
        ├── osc.rs      # Sinusoscillator
        └── envelope.rs # Toonhoogte-envelope
```

---

## Code

### `src/dsp/osc.rs`

```rust
use std::f32::consts::TAU;

/// Een eenvoudige sinusoscillator die fase bijhoudt tussen samples.
///
/// De fase loopt van 0.0 tot 1.0 en wordt pas bij het uitlezen
/// vermenigvuldigd met `TAU`, zodat afronding minimaal blijft.
pub struct SineOsc {
    phase: f32,
    sample_rate: f32,
}

impl SineOsc {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            phase: 0.0,
            sample_rate,
        }
    }

    /// Herstart de oscillator op een nuldoorgang.
    ///
    /// Het resetten van de fase bij elke trigger voorkomt klikken
    /// aan het begin van de kick.
    pub fn reset(&mut self) {
        self.phase = 0.0;
    }

    /// Genereert het volgende sample voor de opgegeven frequentie.
    pub fn tick(&mut self, freq: f32) -> f32 {
        let value = (self.phase * TAU).sin();
        self.phase = (self.phase + freq / self.sample_rate).fract();
        value
    }
}
```

### `src/dsp/envelope.rs`

```rust
/// Lineair voortschrijdende positie van 0.0 naar 1.0 over een vaste decaytijd.
///
/// De envelope is alleen verantwoordelijk voor het ruwe verloop; de
/// vormgeving (exponent voor de toonhoogtecurve) gebeurt in de process-lus,
/// zodat deze module herbruikbaar blijft.
#[derive(Default)]
pub struct PitchEnvelope {
    position: f32,
    active: bool,
}

impl PitchEnvelope {
    /// Start de envelope opnieuw vanaf het begin.
    pub fn trigger(&mut self) {
        self.position = 0.0;
        self.active = true;
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Schuift de positie op en geeft de genormaliseerde voortgang terug
    /// (0.0 = begin, 1.0 = einde). Bij het bereiken van het einde wordt
    /// de envelope inactief.
    pub fn tick(&mut self, decay_samples: f32) -> f32 {
        if !self.active {
            return 1.0;
        }

        self.position += 1.0 / decay_samples;

        if self.position >= 1.0 {
            self.position = 1.0;
            self.active = false;
        }

        self.position
    }
}
```

### `src/dsp/mod.rs`

```rust
pub mod envelope;
pub mod osc;

pub use envelope::PitchEnvelope;
pub use osc::SineOsc;
```

### `src/params.rs`

```rust
use nih_plug::prelude::*;

#[derive(Params)]
pub struct HardKickParams {
    /// Startfrequentie van de sweep, in Hz.
    #[id = "pitch_start"]
    pub pitch_start: FloatParam,

    /// Eindfrequentie van de sweep, in Hz.
    #[id = "pitch_end"]
    pub pitch_end: FloatParam,

    /// Decaytijd van de envelope, in seconden.
    #[id = "decay"]
    pub decay: FloatParam,

    /// Exponent voor de toonhoogtecurve.
    ///
    /// Waarden boven 1.0 geven een snelle initiële val (hardstylekarakter);
    /// waarden onder 1.0 een tragere val.
    #[id = "curve"]
    pub curve: FloatParam,
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

            curve: FloatParam::new(
                "Curve",
                2.0,
                FloatRange::Linear {
                    min: 0.1,
                    max: 8.0,
                },
            )
            .with_value_to_string(formatters::v2s_f32_rounded(2)),
        }
    }
}
```

### `src/lib.rs`

```rust
use nih_plug::prelude::*;
use std::sync::Arc;

mod dsp;
mod params;

use dsp::{PitchEnvelope, SineOsc};
use params::HardKickParams;

pub struct HardKick {
    params: Arc<HardKickParams>,
    osc: SineOsc,
    envelope: PitchEnvelope,
    sample_rate: f32,
}

impl Default for HardKick {
    fn default() -> Self {
        Self {
            params: Arc::new(HardKickParams::default()),
            osc: SineOsc::new(44_100.0),
            envelope: PitchEnvelope::default(),
            sample_rate: 44_100.0,
        }
    }
}

/// Lineaire interpolatie tussen `a` en `b` op positie `t` (0.0..=1.0).
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
        self.envelope = PitchEnvelope::default();
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let mut next_event = context.next_event();

        for (sample_id, channel_samples) in buffer.iter_samples().enumerate() {
            // Verwerk alle events die op dit sample vallen.
            while let Some(event) = next_event {
                if event.timing() != sample_id as u32 {
                    break;
                }

                if let NoteEvent::NoteOn { .. } = event {
                    self.osc.reset();
                    self.envelope.trigger();
                }

                next_event = context.next_event();
            }

            let value = if self.envelope.is_active() {
                let decay_samples = self.params.decay.value() * self.sample_rate;
                let t = self.envelope.tick(decay_samples);

                let curved = t.powf(self.params.curve.value());
                let freq = lerp(
                    self.params.pitch_start.value(),
                    self.params.pitch_end.value(),
                    curved,
                );

                // Amplitude volgt het complement van de envelope.
                self.osc.tick(freq) * (1.0 - t)
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
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("Hardstyle-kick-synthesizer");
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
```

### `src/main.rs`

```rust
fn main() {
    nih_plug::nih_export_standalone::<hardkick::HardKick>();
}
```

> Let op: afhankelijk van de `nih-plug`-versie heet het standalone-entrypoint
> `nih_export_standalone` (macro/functie). Werkt bovenstaande niet, gebruik dan
> in `lib.rs` de macro `nih_export_standalone!(HardKick);` en houd `main.rs`
> leeg op een aanroep van de gegenereerde main na. Controleer de actuele
> standalone-voorbeelden in de `nih-plug`-repository.

---

## Bouwen en draaien

```bash
# Standalone draaien (snelste dev-lus, geen DAW nodig)
cargo run --bin hardkick-standalone --release

# Plugin-bibliotheek bouwen (CLAP/VST3)
cargo build --release
```

De standalone-binary opent een venster met:

- automatisch gedetecteerde audio-uitvoer via CPAL;
- een virtueel toetsenbord om noten te triggeren;
- alle parameters uit `HardKickParams` als instelbare regelaars.

---

## Definition of done

- `cargo run --bin hardkick-standalone --release` opent zonder fouten een venster.
- Een MIDI-noot (virtueel toetsenbord) levert een hoorbare, dalende sinussweep.
- De parameters reageren in realtime op verstelling.
- `cargo build --release` compileert de `cdylib` zonder fouten naast de standalone.
- Geen klikken bij het triggeren (fasereset volstaat in deze fase).
- Geen heap-allocatie in de process-lus (`assert_process_alloc` bewaakt dit in debug).

---

## Vervolgmilestones (niet nu implementeren)

1. Waveshaping/distortion (hard clip, soft clip, wavefold) — onderscheid hardstyle/rawstyle.
2. Click-/attack-laag (ruisburst of losse oscillator).
3. Tone stack (biquad-EQ) en transient shaper.
4. Brickwall-limiter op de uitgang.
5. Eigen GUI via `vizia`.
6. Headless offline-render naar `.wav` voor previewgeneratie in bcktrck.