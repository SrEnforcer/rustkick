# hardkick — milestone 4: exponential pitch + transient click layer

Two changes that close the biggest remaining gap to a real hardstyle kick:
a perceptually-correct pitch sweep, and the mechanical "tok" attack.

---

## Goal

- Replace the linear-in-Hz pitch sweep with an **exponential (log-domain)** sweep.
- Add a **transient click layer** — a short filtered noise burst for the "tok".
- Mix the click in parallel **after** the distortion (mixdown-after routing).

---

## 1. Exponential pitch sweep

### Problem

Milestone 1–3 interpolated frequency linearly in Hz:

```
freq = lerp(start, end, t^curve)
```

The design report (p.4) flags this: linear-in-Hz modulation sounds floaty and
unnatural in the low end, because pitch is perceived logarithmically (an octave
is a *ratio*, not a fixed Hz offset).

### Solution

Interpolate geometrically between start and end:

```
shaped_t = t^curve
freq      = start · (end / start)^shaped_t
```

At `shaped_t = 0` this is `start`; at `1` it is `end`; in between it glides
through equal frequency *ratios* per unit time. The `curve` parameter still
shapes how fast the sweep moves. Both `start` and `end` are ≥ 20 Hz, so the
division is always safe.

The waveform preview uses the same formula so the display stays accurate.

## 2. Transient click layer

The "tok" is a short, bright, mechanical attack sitting on top of the tonal body.

### Implementation

- **`src/dsp/noise.rs`** — a xorshift32 white-noise source (allocation-free,
  a few integer ops per sample).
- A dedicated short **click envelope** (`click_env`) triggered together with the
  main envelopes via the new `fire()` helper.
- A **high-pass biquad** (`click_hp`, Q ≈ 0.707) shapes the burst so only its
  high-frequency character remains and it doesn't muddy the sub.

### Routing — mixdown-after

The click is summed **in parallel, after** the distortion / DC blocker / post-EQ
(design report recommendation #4, PunchBox-style). This keeps the click crisp and
defined instead of being smeared by the saturator:

```
tonal = osc → pre-EQ → shaper → DC block → post-EQ → × amp env
click = noise → high-pass → × click env × click level
out   = clamp(tonal + click)
```

### Parameters

| Param         | Range        | Default | Notes                          |
|---------------|--------------|---------|--------------------------------|
| `click_level` | 0.0 – 1.0    | 0.0     | 0 = no click (preserves prior) |
| `click_decay` | 0.5 – 50 ms  | 4 ms    | Very short = sharp tok         |
| `click_tone`  | 200–8000 Hz  | 2000 Hz | High-pass cutoff for the burst |

---

## Refactor

Trigger logic is consolidated into a single `HardKick::fire(velocity)` method
(resets the oscillator and triggers the pitch, amp and click envelopes). Both
the immediate-trigger path and the post-declick pending path now call it, so the
envelopes can never drift out of sync.

---

## Definition of done

- `cargo build --release` compiles without errors or warnings.
- With `click_level = 0` the only audible change vs. milestone 3 is the smoother,
  punchier pitch glide.
- Raising `click_level` adds a distinct attack; `click_decay` and `click_tone`
  shape it from a soft thump to a sharp tick.
- No heap allocation in the process loop.

---

## Next milestones

1. Linkwitz-Riley crossover to protect the sub band from the distortion.
2. Brickwall limiter on the output.
3. Oversampling (2×/4×) around the waveshaper to suppress aliasing.
4. Preset system + headless offline render to `.wav`.
