# hardkick — milestone 2: waveshaping / distortion

Building on milestone 1 (a triggerable, pitched sine sweep), this step adds the
single most important ingredient for a "hard" kick: **non-linear distortion**.

A pure sine has no harmonics, so no amount of pitch- or amplitude-shaping can
make it sound hard. Hardness *is* harmonic content, and harmonics come from
waveshaping. This milestone introduces a small, real-time-safe distortion stage
plus a DC blocker, and exposes it through the UI and the waveform preview.

No extra layers, no EQ, no oversampling yet — those are later milestones.

---

## Goal

- A selectable waveshaper (`Soft` / `Hard` / `Fold`) applied to the oscillator.
- `Drive`, `Bias` and `Mix` controls to dial the distortion from clean to extreme.
- A DC-blocking high-pass to remove the offset that asymmetric shaping creates.
- The waveform preview reflects the distortion so the shape is visible at a glance.
- Defaults preserve the milestone-1 sound (clean sine) so nothing changes until
  the user turns up `Drive`.

---

## DSP design

### Signal order (per sample)

```
osc → waveshape(drive, bias) → dry/wet mix → DC blocker → × amp envelope → safety clip
```

The oscillator is shaped at full scale *before* the amplitude envelope is
applied, so the harmonic content stays consistent across the whole kick instead
of changing with the decay level.

### Shapers (`src/dsp/distortion.rs`)

- **Soft** — asymmetric `tanh`: `(x·g + b).tanh() − b.tanh()`. Odd harmonics, with
  even harmonics added as `bias` increases. The subtraction removes the resting
  DC offset the bias introduces.
- **Hard** — asymmetric hard clip: `(x·g + b).clamp(−1, 1) − b.clamp(−1, 1)`. Sharp,
  square character.
- **Fold** — a wavefolder (`hard_fold`) that mirrors the signal back over a
  threshold, producing dense, metallic rawstyle overtones.

`drive` (0..1) maps to an input gain of `1 + drive·24`. All three shapers are
inherently bounded to ≈[−1, 1].

### DC blocker

A first-order high-pass (`y = x − x₁ + R·y₁`, `R = 0.9995`, ≈20 Hz at 44.1 kHz)
sits right after shaping. Asymmetric distortion produces a DC offset that would
otherwise eat headroom and cause floating sub frequencies on a PA.

### Output safety

A final `clamp(−1, 1)` guards against extreme parameter combinations. The shapers
are already bounded, so this is only a safety net — a proper brickwall limiter is
milestone 4.

---

## Parameters added (`src/params.rs`)

| Param      | Range      | Default | Notes                                  |
|------------|------------|---------|----------------------------------------|
| `shaper`   | Soft/Hard/Fold | Soft | Waveshaping model                      |
| `drive`    | 0.0 – 1.0  | 0.0     | Distortion intensity (0 = clean)       |
| `bias`     | 0.0 – 1.0  | 0.0     | Asymmetry → even harmonics             |
| `dist_mix` | 0.0 – 1.0  | 1.0     | Dry/wet of the distorted signal        |

---

## Definition of done

- `cargo build --release` compiles the `cdylib` and standalone without errors.
- With `Drive = 0` the kick is identical to milestone 1 (clean sine).
- Raising `Drive` audibly adds harmonics; `Shaper` changes the character and
  `Bias` thickens the tone.
- The waveform preview visibly squares off / folds as distortion increases.
- No heap allocation in the process loop (guarded by `assert_process_alloc`).

---

## Next milestones (not implemented yet)

1. Pre-/post-distortion biquad EQ (the rawstyle "screech" resonance peak).
2. Click/attack layer (noise burst or separate oscillator) for the "tok".
3. Linkwitz-Riley crossover to protect the sub from the distortion.
4. Brickwall limiter on the output.
5. Oversampling (2×/4×) around the shaper to control aliasing.
6. Headless offline render to `.wav` for preview generation.
