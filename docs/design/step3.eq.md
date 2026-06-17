# hardkick — milestone 3: pre/post EQ + UI segmented switch

Building on milestone 2 (waveshaping), this step adds the frequency-sculpting
layer that turns a merely-distorted sine into a genuine rawstyle kick.

---

## Goal

- A biquad **peaking EQ before the distortion** (the "screech" generator).
- A **high-shelf tone control after the distortion** to tame harshness.
- A custom **segmented switch** in the UI for the shaper model instead of a slider.
- A consistent **dark themed** egui style applied globally.

---

## DSP design

### Full signal path (per sample)

```
osc → pre-EQ (peak) → shaper → DC blocker → post-EQ (shelf) → × amp env → clip
```

### Pre-EQ — the screech mechanism

A peaking biquad (Q = 0.5–10, gain 0–24 dB) sits immediately before the
saturator. Because the distortion is non-linear, only the boosted frequency band
is clipped hard — generating dense harmonics *around* that specific frequency.
This is the mechanism behind the iconic rawstyle screech.

- **Screech Hz** (200–8000 Hz): sets which overtone is excited.
- **Screech Q**: narrower = more surgical / more intense.
- **Screech dB**: 0 = neutral (pre-EQ is acoustically bypassed at 0 dB gain).

Typical starting point: 2–3 kHz, Q 4–6, gain 12–18 dB with Hard or Fold shaper.

### Post-EQ — tone control

A high-shelf filter fixed at 4 kHz, ±18/+6 dB range. Negative values pull back
harshness from the distortion; positive values add top-end brightness.
Set to 0 dB by default (identity).

### Biquad implementation (`src/dsp/filter.rs`)

Direct Form II Transposed — stable under high-Q and high-gain settings.
Coefficients are recomputed once per buffer (not per sample), which is both cheap
and avoids branching in the hot path.

---

## UI design

### Segmented switch (`shaper_switch` in `editor.rs`)

The shaper previously used an opaque slider over an enum. Three discrete options
deserve a three-way hardware-style toggle:

- Pill-shaped container with a dim outline.
- Each segment can be clicked to select. Active segment has:
  - A filled accent-coloured background.
  - A small glow dot at the top.
  - Bright text label.
- Inactive segments are dark with muted text.

### Dark theme

A global egui style override sets panel background, widget fills, and accent
colours consistently. All custom-painted widgets share the same `ACCENT` /
`ACCENT_DIM` / `PANEL_BG` / `SECTION_BG` constants.

---

## Parameters added

| Param         | Range           | Default | Notes                              |
|---------------|-----------------|---------|------------------------------------|
| `pre_eq_freq` | 200–8000 Hz     | 2000 Hz | Screech centre frequency           |
| `pre_eq_q`    | 0.5–10          | 3.0     | Peak bandwidth                     |
| `pre_eq_gain` | 0–24 dB         | 0 dB    | 0 = pre-EQ is neutral              |
| `tone`        | −18 to +6 dB   | 0 dB    | Post-distortion high-shelf         |

---

## Definition of done

- `cargo build --release` compiles without errors.
- With `pre_eq_gain = 0` and `tone = 0` the sound is identical to milestone 2.
- Raising `Screech dB` + a Hard/Fold shaper + Drive > 0.5 produces an audible
  resonant screech character.
- The shaper switch displays correctly: active segment glows, clicking changes mode.
- The dark theme applies uniformly across all UI sections.

---

## Next milestones

1. Exponential pitch envelope (logarithmic Hz interpolation — see design doc p.4).
2. Click/attack layer — a short noise burst or separate high-frequency oscillator for the "tok".
3. Linkwitz-Riley crossover to protect the sub from the distortion.
4. Brickwall limiter on the output.
5. Oversampling (2×/4×) around the waveshaper to suppress aliasing.
