use crate::dsp::{shape, OsFactor, Shaper};
use crate::params::HardKickParams;
use crate::presets;
use crate::render::export_wav;
use nih_plug::context::gui::ParamSetter;
use nih_plug::prelude::{Editor, Param};
use nih_plug_egui::egui::epaint::StrokeKind;
use nih_plug_egui::{create_egui_editor, egui, EguiState};
use std::f32::consts::{PI, TAU};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

// Mechanical dark-purple palette.
const ACCENT: egui::Color32 = egui::Color32::from_rgb(170, 130, 235);
const ACCENT_DIM: egui::Color32 = egui::Color32::from_rgb(80, 55, 120);
const PANEL_BG: egui::Color32 = egui::Color32::from_rgb(18, 14, 28);
const SECTION_BG: egui::Color32 = egui::Color32::from_rgb(30, 24, 46);
const KNOB_BODY: egui::Color32 = egui::Color32::from_rgb(46, 39, 64);
const LABEL: egui::Color32 = egui::Color32::from_rgb(185, 170, 215);
const LABEL_DIM: egui::Color32 = egui::Color32::from_rgb(110, 95, 150);

// Knob sweep: 270° with the gap at the bottom (lower-left → top → lower-right).
const KNOB_A0: f32 = 0.75 * PI;
const KNOB_A1: f32 = 2.25 * PI;

/// A rotary knob bound to any `Param`. Vertical drag changes the value;
/// double-click resets to default. Hover shows the formatted value.
fn knob<P: Param>(ui: &mut egui::Ui, label: &str, param: &P, setter: &ParamSetter) {
    let cell = egui::vec2(50.0, 56.0);
    let (rect, _) = ui.allocate_exact_size(cell, egui::Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }

    let radius = 17.0;
    let center = egui::pos2(rect.center().x, rect.top() + radius + 5.0);
    let knob_rect = egui::Rect::from_center_size(center, egui::vec2(radius * 2.0, radius * 2.0));
    let resp = ui.interact(knob_rect, ui.id().with(label), egui::Sense::click_and_drag());

    if resp.drag_started() {
        setter.begin_set_parameter(param);
    }
    if resp.dragged() {
        let cur = param.unmodulated_normalized_value();
        // Hold Shift for fine control.
        let speed = if ui.input(|i| i.modifiers.shift) {
            0.0015
        } else {
            0.006
        };
        let new = (cur - resp.drag_delta().y * speed).clamp(0.0, 1.0);
        setter.set_parameter_normalized(param, new);
    }
    if resp.drag_stopped() {
        setter.end_set_parameter(param);
    }
    if resp.double_clicked() {
        setter.begin_set_parameter(param);
        setter.set_parameter_normalized(param, param.default_normalized_value());
        setter.end_set_parameter(param);
    }

    let t = param.unmodulated_normalized_value();
    let painter = ui.painter_at(rect);

    // Knob body with a subtle rim.
    painter.circle_filled(center, radius, KNOB_BODY);
    painter.circle_stroke(center, radius, egui::Stroke::new(1.0, ACCENT_DIM));

    // Value arc around the rim.
    let arc_r = radius + 2.5;
    let segs = 28;
    let mut prev: Option<egui::Pos2> = None;
    for i in 0..=segs {
        let f = i as f32 / segs as f32;
        let ang = KNOB_A0 + (KNOB_A1 - KNOB_A0) * f;
        let p = center + egui::vec2(ang.cos(), ang.sin()) * arc_r;
        if let Some(pp) = prev {
            let col = if f <= t { ACCENT } else { ACCENT_DIM };
            painter.line_segment([pp, p], egui::Stroke::new(2.0, col));
        }
        prev = Some(p);
    }

    // Pointer.
    let ang = KNOB_A0 + (KNOB_A1 - KNOB_A0) * t;
    let dir = egui::vec2(ang.cos(), ang.sin());
    painter.line_segment(
        [center + dir * (radius * 0.3), center + dir * (radius - 2.0)],
        egui::Stroke::new(2.0, ACCENT),
    );

    // Caption.
    painter.text(
        egui::pos2(rect.center().x, rect.bottom() - 5.0),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::monospace(8.5),
        LABEL,
    );

    resp.on_hover_text(param.normalized_value_to_string(t, true));
}

/// Compact vertical 3-position lever switch (up / middle / down) with a small
/// label per position and a caption beneath. Generic over an enum value; the
/// caller supplies a closure to commit the selection.
fn lever_switch<T: PartialEq + Copy>(
    ui: &mut egui::Ui,
    caption: &str,
    current: T,
    options: &[(T, &str)],
    mut select: impl FnMut(T),
) {
    let cell = egui::vec2(56.0, 56.0);
    let (rect, _) = ui.allocate_exact_size(cell, egui::Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }

    let n = options.len();
    let track_w = 14.0;
    let track_h = 42.0;
    let track = egui::Rect::from_min_size(
        egui::pos2(rect.left() + 4.0, rect.top() + 1.0),
        egui::vec2(track_w, track_h),
    );

    let painter = ui.painter_at(rect);
    painter.rect_filled(track, 4.0, PANEL_BG);
    painter.rect_stroke(
        track,
        4.0,
        egui::Stroke::new(1.0, ACCENT_DIM),
        StrokeKind::Inside,
    );

    let active_idx = options.iter().position(|(v, _)| *v == current).unwrap_or(0);

    for (i, &(variant, label)) in options.iter().enumerate() {
        let cy = track.top() + track_h * (i as f32 + 0.5) / n as f32;
        let is_active = i == active_idx;

        if is_active {
            // Illuminated lever cap straddling the track.
            let cap = egui::Rect::from_center_size(
                egui::pos2(track.center().x, cy),
                egui::vec2(track_w + 5.0, 11.0),
            );
            painter.rect_filled(cap, 3.0, ACCENT);
        } else {
            painter.circle_filled(egui::pos2(track.center().x, cy), 1.8, ACCENT_DIM);
        }

        painter.text(
            egui::pos2(track.right() + 6.0, cy),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::monospace(8.5),
            if is_active { ACCENT } else { LABEL_DIM },
        );

        // Click zone covering this third of the cell.
        let zone = egui::Rect::from_min_size(
            egui::pos2(rect.left(), track.top() + track_h * i as f32 / n as f32),
            egui::vec2(rect.width(), track_h / n as f32),
        );
        let resp = ui.interact(zone, ui.id().with((caption, i)), egui::Sense::click());
        if resp.clicked() && !is_active {
            select(variant);
        }
    }

    painter.text(
        egui::pos2(rect.center().x, rect.bottom() - 5.0),
        egui::Align2::CENTER_CENTER,
        caption,
        egui::FontId::monospace(8.5),
        LABEL,
    );
}

/// A framed module panel with a header, laying its content out in a knob row.
fn panel(ui: &mut egui::Ui, title: &str, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::group(ui.style())
        .fill(SECTION_BG)
        .stroke(egui::Stroke::new(1.0, ACCENT_DIM))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new(title)
                        .font(egui::FontId::monospace(8.5))
                        .color(ACCENT),
                );
                ui.add_space(1.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(1.0, 0.0);
                    add(ui);
                });
            });
        });
}

/// Waveform preview mirroring the audio signal path (sans biquads).
fn kick_waveform(ui: &mut egui::Ui, params: &HardKickParams) {
    const N: usize = 512;
    let sr = 44_100.0_f32;

    let amp_samples = params.amp_decay.value() * sr;
    let pitch_samples = params.decay.value() * sr;
    let step = amp_samples / N as f32;

    let mut phase = 0.0_f32;
    let mut wave = Vec::with_capacity(N);

    for i in 0..N {
        let t_amp = i as f32 / (N - 1) as f32;
        let t_pitch = (i as f32 * step / pitch_samples).min(1.0);
        let shaped_t = t_pitch.powf(params.curve.value());
        let start = params.pitch_start.value();
        let end = params.pitch_end.value();
        let freq = start * (end / start).powf(shaped_t);
        let amp = (1.0 - t_amp).powf(params.amp_curve.value()) * params.level.value();

        let osc = (phase * TAU).sin();
        let shaped = shape(
            osc,
            params.shaper.value(),
            params.drive.value(),
            params.bias.value(),
        );
        let mix = params.dist_mix.value();
        let body = osc + (shaped - osc) * mix;
        wave.push((body * amp).clamp(-1.0, 1.0));
        phase = (phase + freq * step / sr).fract();
    }

    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 58.0), egui::Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }

    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, PANEL_BG);
    painter.rect_stroke(
        rect,
        4.0,
        egui::Stroke::new(1.0, ACCENT_DIM),
        StrokeKind::Inside,
    );

    let cy = rect.center().y;
    let h = rect.height() * 0.42;
    let w = rect.width();

    painter.line_segment(
        [egui::pos2(rect.left(), cy), egui::pos2(rect.right(), cy)],
        egui::Stroke::new(0.5, egui::Color32::from_rgb(55, 45, 75)),
    );

    let pts: Vec<egui::Pos2> = wave
        .iter()
        .enumerate()
        .map(|(i, &s)| {
            let x = rect.left() + i as f32 / (N - 1) as f32 * w;
            egui::pos2(x, cy - s * h)
        })
        .collect();
    for win in pts.windows(2) {
        painter.line_segment([win[0], win[1]], egui::Stroke::new(1.5, ACCENT));
    }
}

pub fn create(
    params: Arc<HardKickParams>,
    editor_state: Arc<EguiState>,
    trigger: Arc<AtomicBool>,
    playing: Arc<AtomicBool>,
) -> Option<Box<dyn Editor>> {
    create_egui_editor(
        editor_state,
        (),
        |ctx, _| {
            let mut style = (*ctx.style()).clone();
            style.visuals.window_fill = PANEL_BG;
            style.visuals.panel_fill = PANEL_BG;
            style.visuals.widgets.noninteractive.bg_fill = SECTION_BG;
            style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(35, 28, 52);
            style.visuals.widgets.hovered.bg_fill = ACCENT_DIM;
            style.visuals.widgets.active.bg_fill = ACCENT;
            style.visuals.widgets.noninteractive.fg_stroke.color = ACCENT;
            ctx.set_style(style);
        },
        move |ctx, setter, _state| {
            if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
                trigger.store(true, Ordering::Relaxed);
            }

            egui::CentralPanel::default().show(ctx, |ui| {
                // ── TITLE BAR + TRANSPORT ────────────────────────────────────
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("HARDKICK")
                            .font(egui::FontId::proportional(18.0))
                            .strong()
                            .color(ACCENT),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("💾 WAV").clicked() {
                            let path = format!(
                                "{}/hardkick-export.wav",
                                std::env::var("HOME").unwrap_or_else(|_| ".".into())
                            );
                            match export_wav(&params, &path) {
                                Ok(()) => nih_plug::nih_log!("Exported to {}", path),
                                Err(e) => nih_plug::nih_log!("Export failed: {}", e),
                            }
                        }
                        if ui.button("⚡ Trigger").clicked() {
                            trigger.store(true, Ordering::Relaxed);
                        }
                        let is_playing = playing.load(Ordering::Relaxed);
                        let play_label = if is_playing { "⏹ Stop" } else { "▶ Play" };
                        if ui.button(play_label).clicked() {
                            playing.store(!is_playing, Ordering::Relaxed);
                        }
                    });
                });

                ui.add_space(3.0);
                kick_waveform(ui, &params);
                ui.add_space(4.0);

                // ── PRESETS (single row) ─────────────────────────────────────
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 3.0;
                    let count = presets::PRESETS.len();
                    let bw = (ui.available_width() - 3.0 * (count as f32 - 1.0)) / count as f32;
                    for preset in presets::PRESETS {
                        let btn = egui::Button::new(
                            egui::RichText::new(preset.name)
                                .font(egui::FontId::monospace(8.5))
                                .color(ACCENT),
                        )
                        .fill(SECTION_BG)
                        .stroke(egui::Stroke::new(1.0, ACCENT_DIM))
                        .min_size(egui::vec2(bw, 20.0));
                        if ui.add(btn).clicked() {
                            presets::apply(preset, &params, setter);
                        }
                    }
                });

                ui.add_space(5.0);

                // ── ROW A: PITCH | AMP | SHAPE ───────────────────────────────
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 5.0;

                    panel(ui, "PITCH", |ui| {
                        knob(ui, "Start", &params.pitch_start, setter);
                        knob(ui, "End", &params.pitch_end, setter);
                        knob(ui, "Decay", &params.decay, setter);
                        knob(ui, "Curve", &params.curve, setter);
                    });

                    panel(ui, "AMP", |ui| {
                        knob(ui, "Decay", &params.amp_decay, setter);
                        knob(ui, "Curve", &params.amp_curve, setter);
                        knob(ui, "Level", &params.level, setter);
                    });

                    panel(ui, "SHAPE", |ui| {
                        lever_switch(
                            ui,
                            "MODE",
                            params.shaper.value(),
                            &[
                                (Shaper::Tube, "TUBE"),
                                (Shaper::Hard, "HARD"),
                                (Shaper::Fold, "FOLD"),
                            ],
                            |v| {
                                setter.begin_set_parameter(&params.shaper);
                                setter.set_parameter(&params.shaper, v);
                                setter.end_set_parameter(&params.shaper);
                            },
                        );
                        knob(ui, "Drive", &params.drive, setter);
                        knob(ui, "Bias", &params.bias, setter);
                        knob(ui, "Mix", &params.dist_mix, setter);
                        knob(ui, "Xover", &params.crossover_freq, setter);
                    });
                });

                ui.add_space(5.0);

                // ── ROW B: EQ | TRANSIENT | OUTPUT | SEQ ─────────────────────
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 5.0;

                    panel(ui, "EQ / SCREECH", |ui| {
                        knob(ui, "Freq", &params.pre_eq_freq, setter);
                        knob(ui, "Q", &params.pre_eq_q, setter);
                        knob(ui, "Gain", &params.pre_eq_gain, setter);
                        knob(ui, "Tone", &params.tone, setter);
                    });

                    panel(ui, "TRANSIENT", |ui| {
                        knob(ui, "Click", &params.click_level, setter);
                        knob(ui, "Decay", &params.click_decay, setter);
                        knob(ui, "Tone", &params.click_tone, setter);
                    });

                    panel(ui, "OUTPUT", |ui| {
                        lever_switch(
                            ui,
                            "OS",
                            params.oversample.value(),
                            &[
                                (OsFactor::Off, "OFF"),
                                (OsFactor::X2, "2x"),
                                (OsFactor::X4, "4x"),
                            ],
                            |v| {
                                setter.begin_set_parameter(&params.oversample);
                                setter.set_parameter(&params.oversample, v);
                                setter.end_set_parameter(&params.oversample);
                            },
                        );
                        knob(ui, "Ceil", &params.limiter_threshold, setter);
                        knob(ui, "Rel", &params.limiter_release, setter);
                    });

                    panel(ui, "SEQ", |ui| {
                        knob(ui, "BPM", &params.bpm, setter);
                    });
                });
            });
        },
    )
}
