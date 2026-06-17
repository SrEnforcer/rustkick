use crate::params::HardKickParams;
use nih_plug::prelude::Editor;
use nih_plug_egui::{create_egui_editor, egui, widgets, EguiState};
use std::f32::consts::TAU;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

fn section_header(ui: &mut egui::Ui, label: &str) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 20.0), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        let fill = ui.visuals().widgets.noninteractive.bg_fill;
        let text_color = ui.visuals().widgets.noninteractive.fg_stroke.color;
        ui.painter().rect_filled(rect, 2.0, fill);
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(11.0),
            text_color,
        );
    }
}

/// Renders a preview of the kick waveform derived from the current parameter values.
/// Simulates N evenly-spaced samples over the full amplitude decay duration and draws
/// the result as a polyline so the user can see the pitch sweep and envelope shape at a glance.
fn kick_waveform(ui: &mut egui::Ui, params: &HardKickParams) {
    const N: usize = 256;
    let sr = 44_100.0_f32;

    let amp_samples = params.amp_decay.value() * sr;
    let pitch_samples = params.decay.value() * sr;
    // Each simulated point covers this many real samples.
    let step = amp_samples / N as f32;

    let mut phase = 0.0_f32;
    let mut wave = Vec::with_capacity(N);

    for i in 0..N {
        let t_amp = i as f32 / (N - 1) as f32;
        let t_pitch = (i as f32 * step / pitch_samples).min(1.0);
        let freq = params.pitch_start.value()
            + (params.pitch_end.value() - params.pitch_start.value())
                * t_pitch.powf(params.curve.value());
        let amp = (1.0 - t_amp).powf(params.amp_curve.value()) * params.level.value();
        wave.push((phase * TAU).sin() * amp);
        phase = (phase + freq * step / sr).fract();
    }

    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), 80.0),
        egui::Sense::hover(),
    );

    if !ui.is_rect_visible(rect) {
        return;
    }

    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, egui::Color32::from_rgb(20, 16, 32));

    let cy = rect.center().y;
    let h = rect.height() * 0.45;
    let w = rect.width();

    // Subtle centre line.
    painter.line_segment(
        [egui::pos2(rect.left(), cy), egui::pos2(rect.right(), cy)],
        egui::Stroke::new(0.5, egui::Color32::from_rgb(55, 45, 75)),
    );

    // Waveform polyline.
    let pts: Vec<egui::Pos2> = wave
        .iter()
        .enumerate()
        .map(|(i, &s)| {
            let x = rect.left() + i as f32 / (N - 1) as f32 * w;
            egui::pos2(x, cy - s * h)
        })
        .collect();

    for win in pts.windows(2) {
        painter.line_segment(
            [win[0], win[1]],
            egui::Stroke::new(1.5, egui::Color32::from_rgb(160, 120, 220)),
        );
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
        |_, _| {},
        move |ctx, setter, _state| {
            // Space = one-shot trigger.
            if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
                trigger.store(true, Ordering::Relaxed);
            }

            egui::CentralPanel::default().show(ctx, |ui| {
                ui.heading("HardKick");
                ui.add_space(4.0);

                kick_waveform(ui, &params);
                ui.add_space(4.0);

                section_header(ui, "PITCH");
                ui.add_space(4.0);

                egui::Grid::new("pitch_params")
                    .num_columns(2)
                    .min_col_width(90.0)
                    .spacing([12.0, 6.0])
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

                ui.add_space(4.0);
                section_header(ui, "AMPLITUDE");
                ui.add_space(4.0);

                egui::Grid::new("amp_params")
                    .num_columns(2)
                    .min_col_width(90.0)
                    .spacing([12.0, 6.0])
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

                ui.add_space(4.0);
                section_header(ui, "SEQUENCER");
                ui.add_space(4.0);

                egui::Grid::new("seq_params")
                    .num_columns(2)
                    .min_col_width(90.0)
                    .spacing([12.0, 6.0])
                    .show(ui, |ui| {
                        ui.label("BPM");
                        // DragValue: click-to-type or drag left/right for coarse/fine control.
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

                ui.add_space(4.0);
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
