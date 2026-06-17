use crate::params::HardKickParams;
use nih_plug::prelude::Editor;
use nih_plug_egui::{create_egui_editor, egui, widgets, EguiState};
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
            // Space = one-shot trigger
            if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
                trigger.store(true, Ordering::Relaxed);
            }

            egui::CentralPanel::default().show(ctx, |ui| {
                ui.heading("HardKick");

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
                        ui.add(widgets::ParamSlider::for_param(&params.bpm, setter));
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
