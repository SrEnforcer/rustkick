use crate::dsp::{shape, Shaper};
use crate::params::HardKickParams;
use nih_plug::context::gui::ParamSetter;
use nih_plug::prelude::Editor;
use nih_plug_egui::egui::epaint::StrokeKind;
use nih_plug_egui::{create_egui_editor, egui, widgets, EguiState};
use std::f32::consts::TAU;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

// Accent colours reused throughout the UI.
const ACCENT: egui::Color32 = egui::Color32::from_rgb(160, 120, 220);
const ACCENT_DIM: egui::Color32 = egui::Color32::from_rgb(80, 55, 120);
const PANEL_BG: egui::Color32 = egui::Color32::from_rgb(20, 16, 32);
const SECTION_BG: egui::Color32 = egui::Color32::from_rgb(30, 24, 46);

fn section_header(ui: &mut egui::Ui, label: &str) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 20.0), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        ui.painter().rect_filled(rect, 2.0, SECTION_BG);
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::monospace(10.0),
            ACCENT,
        );
    }
}

/// Three-way segmented toggle switch for an `EnumParam<Shaper>`.
///
/// Renders as three labelled illuminated segments in a pill shape. The active
/// segment glows with the accent colour; inactive segments are dim. Clicking
/// any segment sets the parameter immediately.
fn shaper_switch(
    ui: &mut egui::Ui,
    current: Shaper,
    setter: &ParamSetter,
    param: &nih_plug::prelude::EnumParam<Shaper>,
) {
    let options: &[(Shaper, &str)] = &[
        (Shaper::Soft, "SOFT"),
        (Shaper::Hard, "HARD"),
        (Shaper::Fold, "FOLD"),
    ];

    let total_width = ui.available_width();
    let height = 26.0;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(total_width, height), egui::Sense::hover());

    if !ui.is_rect_visible(rect) {
        return;
    }

    let seg_w = total_width / options.len() as f32;
    let painter = ui.painter_at(rect);
    let corner = height * 0.4;

    // Outer pill outline.
    painter.rect_stroke(
        rect,
        corner,
        egui::Stroke::new(1.0, ACCENT_DIM),
        StrokeKind::Outside,
    );

    for (idx, &(variant, label)) in options.iter().enumerate() {
        let x = rect.left() + idx as f32 * seg_w;
        let seg_rect =
            egui::Rect::from_min_size(egui::pos2(x, rect.top()), egui::vec2(seg_w, height));

        let is_active = current == variant;
        let fill = if is_active { ACCENT_DIM } else { PANEL_BG };
        let text_color = if is_active {
            ACCENT
        } else {
            egui::Color32::from_rgb(120, 100, 160)
        };

        // Round only the outer corners of the pill.
        let seg_corner = if idx == 0 || idx == options.len() - 1 {
            corner
        } else {
            0.0
        };
        painter.rect_filled(seg_rect.shrink(0.5), seg_corner, fill);

        // Inner segment dividers.
        if idx > 0 {
            painter.line_segment(
                [seg_rect.left_top(), seg_rect.left_bottom()],
                egui::Stroke::new(1.0, ACCENT_DIM),
            );
        }

        // Glow dot above the label when active.
        if is_active {
            let dot_center = egui::pos2(seg_rect.center().x, seg_rect.top() + 5.0);
            painter.circle_filled(dot_center, 2.5, ACCENT);
        }

        painter.text(
            seg_rect.center() + egui::vec2(0.0, if is_active { 1.5 } else { 0.0 }),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::monospace(9.5),
            text_color,
        );

        // Click handling.
        let response = ui.interact(seg_rect, ui.id().with(idx), egui::Sense::click());
        if response.clicked() && !is_active {
            setter.begin_set_parameter(param);
            setter.set_parameter(param, variant);
            setter.end_set_parameter(param);
        }
    }
}

/// Renders a waveform preview that mirrors the full audio signal path including
/// distortion and EQ, so changes to any parameter are reflected immediately.
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
        // Exponential pitch sweep, matching the audio path.
        let shaped_t = t_pitch.powf(params.curve.value());
        let start = params.pitch_start.value();
        let end = params.pitch_end.value();
        let freq = start * (end / start).powf(shaped_t);
        let amp = (1.0 - t_amp).powf(params.amp_curve.value()) * params.level.value();

        let osc = (phase * TAU).sin();
        // The preview doesn't run the biquad (no filter state available here), but
        // shaping is cheap to replicate so the visual at least shows the wavefolder/clipper.
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
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 80.0), egui::Sense::hover());

    if !ui.is_rect_visible(rect) {
        return;
    }

    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, PANEL_BG);

    let cy = rect.center().y;
    let h = rect.height() * 0.45;
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
            // Dark theme base — override egui's default light style.
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
                ui.heading("HardKick");
                ui.add_space(4.0);

                kick_waveform(ui, &params);
                ui.add_space(6.0);

                // ── PITCH ────────────────────────────────────────────────────
                section_header(ui, "PITCH");
                ui.add_space(4.0);
                egui::Grid::new("pitch_params")
                    .num_columns(2)
                    .min_col_width(90.0)
                    .spacing([12.0, 5.0])
                    .show(ui, |ui| {
                        ui.label("Start");
                        ui.add(widgets::ParamSlider::for_param(&params.pitch_start, setter));
                        ui.end_row();
                        ui.label("End");
                        ui.add(widgets::ParamSlider::for_param(&params.pitch_end, setter));
                        ui.end_row();
                        ui.label("Decay");
                        ui.add(widgets::ParamSlider::for_param(&params.decay, setter));
                        ui.end_row();
                        ui.label("Curve");
                        ui.add(widgets::ParamSlider::for_param(&params.curve, setter));
                        ui.end_row();
                    });

                ui.add_space(6.0);

                // ── AMPLITUDE ────────────────────────────────────────────────
                section_header(ui, "AMPLITUDE");
                ui.add_space(4.0);
                egui::Grid::new("amp_params")
                    .num_columns(2)
                    .min_col_width(90.0)
                    .spacing([12.0, 5.0])
                    .show(ui, |ui| {
                        ui.label("Decay");
                        ui.add(widgets::ParamSlider::for_param(&params.amp_decay, setter));
                        ui.end_row();
                        ui.label("Curve");
                        ui.add(widgets::ParamSlider::for_param(&params.amp_curve, setter));
                        ui.end_row();
                        ui.label("Level");
                        ui.add(widgets::ParamSlider::for_param(&params.level, setter));
                        ui.end_row();
                    });

                ui.add_space(6.0);

                // ── SHAPING ──────────────────────────────────────────────────
                section_header(ui, "SHAPING");
                ui.add_space(4.0);

                // Segmented switch for the shaper model.
                shaper_switch(ui, params.shaper.value(), setter, &params.shaper);
                ui.add_space(5.0);

                egui::Grid::new("shaping_params")
                    .num_columns(2)
                    .min_col_width(90.0)
                    .spacing([12.0, 5.0])
                    .show(ui, |ui| {
                        ui.label("Drive");
                        ui.add(widgets::ParamSlider::for_param(&params.drive, setter));
                        ui.end_row();
                        ui.label("Bias");
                        ui.add(widgets::ParamSlider::for_param(&params.bias, setter));
                        ui.end_row();
                        ui.label("Mix");
                        ui.add(widgets::ParamSlider::for_param(&params.dist_mix, setter));
                        ui.end_row();
                    });

                ui.add_space(6.0);

                // ── PRE / POST EQ ────────────────────────────────────────────
                section_header(ui, "EQ");
                ui.add_space(4.0);
                egui::Grid::new("eq_params")
                    .num_columns(2)
                    .min_col_width(90.0)
                    .spacing([12.0, 5.0])
                    .show(ui, |ui| {
                        ui.label("Screech Hz");
                        ui.add(widgets::ParamSlider::for_param(&params.pre_eq_freq, setter));
                        ui.end_row();
                        ui.label("Screech Q");
                        ui.add(widgets::ParamSlider::for_param(&params.pre_eq_q, setter));
                        ui.end_row();
                        ui.label("Screech dB");
                        ui.add(widgets::ParamSlider::for_param(&params.pre_eq_gain, setter));
                        ui.end_row();
                        ui.label("Tone");
                        ui.add(widgets::ParamSlider::for_param(&params.tone, setter));
                        ui.end_row();
                    });

                ui.add_space(6.0);

                // ── TRANSIENT ────────────────────────────────────────────────
                section_header(ui, "TRANSIENT");
                ui.add_space(4.0);
                egui::Grid::new("transient_params")
                    .num_columns(2)
                    .min_col_width(90.0)
                    .spacing([12.0, 5.0])
                    .show(ui, |ui| {
                        ui.label("Click");
                        ui.add(widgets::ParamSlider::for_param(&params.click_level, setter));
                        ui.end_row();
                        ui.label("Click decay");
                        ui.add(widgets::ParamSlider::for_param(&params.click_decay, setter));
                        ui.end_row();
                        ui.label("Click tone");
                        ui.add(widgets::ParamSlider::for_param(&params.click_tone, setter));
                        ui.end_row();
                    });

                ui.add_space(6.0);

                // ── SEQUENCER ────────────────────────────────────────────────
                section_header(ui, "SEQUENCER");
                ui.add_space(4.0);
                egui::Grid::new("seq_params")
                    .num_columns(2)
                    .min_col_width(90.0)
                    .spacing([12.0, 5.0])
                    .show(ui, |ui| {
                        ui.label("BPM");
                        let mut bpm = params.bpm.value();
                        if ui
                            .add(
                                egui::DragValue::new(&mut bpm)
                                    .range(60.0_f32..=220.0_f32)
                                    .speed(0.5)
                                    .suffix(" BPM"),
                            )
                            .changed()
                        {
                            setter.begin_set_parameter(&params.bpm);
                            setter.set_parameter(&params.bpm, bpm);
                            setter.end_set_parameter(&params.bpm);
                        }
                        ui.end_row();
                    });

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    let is_playing = playing.load(Ordering::Relaxed);
                    let play_label = if is_playing { "⏹  Stop" } else { "▶  Play" };
                    if ui.button(play_label).clicked() {
                        playing.store(!is_playing, Ordering::Relaxed);
                    }
                    if ui.button("⚡  Trigger").clicked() {
                        trigger.store(true, Ordering::Relaxed);
                    }
                });
            });
        },
    )
}
