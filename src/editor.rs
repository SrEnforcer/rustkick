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

const ACCENT: egui::Color32 = egui::Color32::from_rgb(170, 130, 235);
const ACCENT_DIM: egui::Color32 = egui::Color32::from_rgb(80, 55, 120);
const PANEL_BG: egui::Color32 = egui::Color32::from_rgb(18, 14, 28);
const SECTION_BG: egui::Color32 = egui::Color32::from_rgb(30, 24, 46);
const KNOB_BODY: egui::Color32 = egui::Color32::from_rgb(46, 39, 64);
const LABEL: egui::Color32 = egui::Color32::from_rgb(185, 170, 215);
const LABEL_DIM: egui::Color32 = egui::Color32::from_rgb(110, 95, 150);

const KNOB_A0: f32 = 0.75 * PI;
const KNOB_A1: f32 = 2.25 * PI;

fn knob<P: Param>(ui: &mut egui::Ui, label: &str, param: &P, setter: &ParamSetter) {
    let cell = egui::vec2(52.0, 58.0);
    let (rect, _) = ui.allocate_exact_size(cell, egui::Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }

    let radius = 17.0;
    let center = egui::pos2(rect.center().x, rect.top() + radius + 6.0);
    let knob_rect = egui::Rect::from_center_size(center, egui::vec2(radius * 2.0, radius * 2.0));
    // Use the param's unique name (not the display label) as the egui ID salt —
    // multiple panels share short display labels like "Tone" and "Level", so
    // hashing those would alias the interactions onto a single widget.
    let resp = ui.interact(knob_rect, ui.id().with(param.name()), egui::Sense::click_and_drag());

    if resp.drag_started() {
        setter.begin_set_parameter(param);
    }
    if resp.dragged() {
        let speed = if ui.input(|i| i.modifiers.shift) { 0.0015 } else { 0.006 };
        let new = (param.unmodulated_normalized_value() - resp.drag_delta().y * speed).clamp(0.0, 1.0);
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
    painter.circle_filled(center, radius, KNOB_BODY);
    painter.circle_stroke(center, radius, egui::Stroke::new(1.0, ACCENT_DIM));

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

    let ang = KNOB_A0 + (KNOB_A1 - KNOB_A0) * t;
    let dir = egui::vec2(ang.cos(), ang.sin());
    painter.line_segment(
        [center + dir * (radius * 0.3), center + dir * (radius - 2.0)],
        egui::Stroke::new(2.0, ACCENT),
    );

    painter.text(
        egui::pos2(rect.center().x, rect.bottom() - 4.0),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::monospace(8.5),
        LABEL,
    );

    let formatted = param.normalized_value_to_string(t, true);
    resp.on_hover_text(formatted);
}

/// Compact vertical lever switch. Works with any Copy+PartialEq enum.
fn lever_switch<T: PartialEq + Copy>(
    ui: &mut egui::Ui,
    caption: &str,
    current: T,
    options: &[(T, &str)],
    mut select: impl FnMut(T),
) {
    let n = options.len();
    let track_w = 14.0;
    let track_h = (n as f32) * 14.0 + 4.0;
    let cell_h = track_h + 16.0;
    let cell_w = 52.0;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(cell_w, cell_h), egui::Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }

    let track = egui::Rect::from_min_size(
        egui::pos2(rect.left() + 4.0, rect.top() + 2.0),
        egui::vec2(track_w, track_h),
    );

    let painter = ui.painter_at(rect);
    painter.rect_filled(track, 4.0, PANEL_BG);
    painter.rect_stroke(track, 4.0, egui::Stroke::new(1.0, ACCENT_DIM), StrokeKind::Inside);

    let active_idx = options.iter().position(|(v, _)| *v == current).unwrap_or(0);

    for (i, &(variant, label)) in options.iter().enumerate() {
        let cy = track.top() + track_h * (i as f32 + 0.5) / n as f32;
        let is_active = i == active_idx;

        if is_active {
            let cap = egui::Rect::from_center_size(
                egui::pos2(track.center().x, cy),
                egui::vec2(track_w + 5.0, 11.0),
            );
            painter.rect_filled(cap, 3.0, ACCENT);
        } else {
            painter.circle_filled(egui::pos2(track.center().x, cy), 1.8, ACCENT_DIM);
        }

        painter.text(
            egui::pos2(track.right() + 5.0, cy),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::monospace(8.5),
            if is_active { ACCENT } else { LABEL_DIM },
        );

        let zone_h = track_h / n as f32;
        let zone = egui::Rect::from_min_size(
            egui::pos2(rect.left(), track.top() + zone_h * i as f32),
            egui::vec2(rect.width(), zone_h),
        );
        let resp = ui.interact(zone, ui.id().with((caption, i)), egui::Sense::click());
        if resp.clicked() && !is_active {
            select(variant);
        }
    }

    painter.text(
        egui::pos2(rect.center().x, rect.bottom() - 3.0),
        egui::Align2::CENTER_CENTER,
        caption,
        egui::FontId::monospace(8.5),
        LABEL,
    );
}

/// Framed module panel with a title and a horizontal row of controls.
fn panel(ui: &mut egui::Ui, title: &str, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(SECTION_BG)
        .stroke(egui::Stroke::new(1.0, ACCENT_DIM))
        .inner_margin(egui::Margin::symmetric(4, 4))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new(title)
                        .font(egui::FontId::monospace(8.5))
                        .color(ACCENT),
                );
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(1.0, 0.0);
                    add(ui);
                });
            });
        });
}

/// Graphical pitch envelope editor.
///
/// Three draggable handles directly control the four scalar pitch params:
/// - left handle: pitch_start (vertical = frequency, log axis)
/// - right handle: pitch_end (vertical) + decay (horizontal = time, log axis)
/// - middle handle: curve exponent (vertical drag reshapes the curve)
///
/// The frequency axis is shared between start and end (log 20–800 Hz); each
/// handle's own param range clamps it. The time axis is log 0.05–2.0 s.
fn pitch_env_editor(ui: &mut egui::Ui, params: &HardKickParams, setter: &ParamSetter) {
    let size = egui::vec2(260.0, 110.0);
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }

    let pad = egui::vec2(6.0, 6.0);
    let plot = egui::Rect::from_min_max(rect.min + pad, rect.max - pad);

    // Log mappings.
    const F_MIN: f32 = 20.0;
    const F_MAX: f32 = 800.0;
    const T_MIN: f32 = 0.05;
    const T_MAX: f32 = 2.0;
    let freq_to_y = |f: f32| -> f32 {
        let n = (f.ln() - F_MIN.ln()) / (F_MAX.ln() - F_MIN.ln());
        plot.bottom() - n.clamp(0.0, 1.0) * plot.height()
    };
    let y_to_freq = |y: f32| -> f32 {
        let n = ((plot.bottom() - y) / plot.height()).clamp(0.0, 1.0);
        (F_MIN.ln() + n * (F_MAX.ln() - F_MIN.ln())).exp()
    };
    let time_to_x = |t: f32| -> f32 {
        let n = (t.ln() - T_MIN.ln()) / (T_MAX.ln() - T_MIN.ln());
        plot.left() + n.clamp(0.0, 1.0) * plot.width()
    };
    let x_to_time = |x: f32| -> f32 {
        let n = ((x - plot.left()) / plot.width()).clamp(0.0, 1.0);
        (T_MIN.ln() + n * (T_MAX.ln() - T_MIN.ln())).exp()
    };

    let start = params.pitch_start.value();
    let end = params.pitch_end.value();
    let decay = params.decay.value();
    let curve = params.curve.value();

    let p_start = egui::pos2(plot.left(), freq_to_y(start));
    let p_end = egui::pos2(time_to_x(decay), freq_to_y(end));

    // Draw background and gridlines.
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, PANEL_BG);
    painter.rect_stroke(rect, 4.0, egui::Stroke::new(1.0, ACCENT_DIM), StrokeKind::Inside);
    let grid = egui::Color32::from_rgb(40, 32, 60);
    for &hz in &[50.0_f32, 100.0, 200.0, 400.0] {
        let y = freq_to_y(hz);
        painter.line_segment(
            [egui::pos2(plot.left(), y), egui::pos2(plot.right(), y)],
            egui::Stroke::new(0.5, grid),
        );
    }

    // Render the curve as a polyline.
    const SEGS: usize = 80;
    let mut prev: Option<egui::Pos2> = None;
    for i in 0..=SEGS {
        let t = i as f32 / SEGS as f32;
        let shaped = t.powf(curve);
        let f = start * (end / start).powf(shaped);
        let p = egui::pos2(
            plot.left() + t * (p_end.x - p_start.x),
            freq_to_y(f),
        );
        if let Some(pp) = prev {
            painter.line_segment([pp, p], egui::Stroke::new(1.6, ACCENT));
        }
        prev = Some(p);
    }

    // Mid handle position: sample the curve at t=0.5.
    let mid_t = 0.5_f32;
    let mid_shaped = mid_t.powf(curve);
    let mid_freq = start * (end / start).powf(mid_shaped);
    let p_mid = egui::pos2(
        plot.left() + mid_t * (p_end.x - p_start.x),
        freq_to_y(mid_freq),
    );

    // Draw a marker handle.
    let draw_handle = |painter: &egui::Painter, p: egui::Pos2, active: bool| {
        let r = 5.5;
        painter.circle_filled(p, r, if active { ACCENT } else { KNOB_BODY });
        painter.circle_stroke(p, r, egui::Stroke::new(1.2, ACCENT));
    };

    // Interactions — each handle gets its own small hit rect.
    let hit = |p: egui::Pos2| egui::Rect::from_center_size(p, egui::vec2(18.0, 18.0));

    let id = ui.id().with("pitch_env");
    let r_start = ui.interact(hit(p_start), id.with("start"), egui::Sense::click_and_drag());
    let r_end = ui.interact(hit(p_end), id.with("end"), egui::Sense::click_and_drag());
    let r_mid = ui.interact(hit(p_mid), id.with("mid"), egui::Sense::click_and_drag());

    if r_start.drag_started() {
        setter.begin_set_parameter(&params.pitch_start);
    }
    if r_start.dragged() {
        if let Some(pos) = r_start.interact_pointer_pos() {
            let new = y_to_freq(pos.y).clamp(20.0, 800.0);
            setter.set_parameter(&params.pitch_start, new);
        }
    }
    if r_start.drag_stopped() {
        setter.end_set_parameter(&params.pitch_start);
    }
    if r_start.double_clicked() {
        setter.begin_set_parameter(&params.pitch_start);
        setter.set_parameter_normalized(
            &params.pitch_start,
            params.pitch_start.default_normalized_value(),
        );
        setter.end_set_parameter(&params.pitch_start);
    }

    if r_end.drag_started() {
        setter.begin_set_parameter(&params.pitch_end);
        setter.begin_set_parameter(&params.decay);
    }
    if r_end.dragged() {
        if let Some(pos) = r_end.interact_pointer_pos() {
            let new_f = y_to_freq(pos.y).clamp(20.0, 200.0);
            let new_t = x_to_time(pos.x).clamp(0.05, 2.0);
            setter.set_parameter(&params.pitch_end, new_f);
            setter.set_parameter(&params.decay, new_t);
        }
    }
    if r_end.drag_stopped() {
        setter.end_set_parameter(&params.pitch_end);
        setter.end_set_parameter(&params.decay);
    }
    if r_end.double_clicked() {
        setter.begin_set_parameter(&params.pitch_end);
        setter.set_parameter_normalized(
            &params.pitch_end,
            params.pitch_end.default_normalized_value(),
        );
        setter.end_set_parameter(&params.pitch_end);
        setter.begin_set_parameter(&params.decay);
        setter.set_parameter_normalized(&params.decay, params.decay.default_normalized_value());
        setter.end_set_parameter(&params.decay);
    }

    if r_mid.drag_started() {
        setter.begin_set_parameter(&params.curve);
    }
    if r_mid.dragged() {
        if let Some(pos) = r_mid.interact_pointer_pos() {
            // Map the pointer y back to a freq, then invert the curve formula:
            // freq = start * (end/start)^(0.5^curve)  ⇒  0.5^curve = log(freq/start)/log(end/start)
            let target = y_to_freq(pos.y).clamp(end.min(start) + 0.01, start.max(end) - 0.01);
            let ratio = (target / start).ln() / (end / start).ln();
            let ratio = ratio.clamp(1e-4, 1.0 - 1e-4);
            let new_curve = ratio.ln() / 0.5_f32.ln();
            setter.set_parameter(&params.curve, new_curve.clamp(0.1, 8.0));
        }
    }
    if r_mid.drag_stopped() {
        setter.end_set_parameter(&params.curve);
    }
    if r_mid.double_clicked() {
        setter.begin_set_parameter(&params.curve);
        setter.set_parameter_normalized(&params.curve, params.curve.default_normalized_value());
        setter.end_set_parameter(&params.curve);
    }

    draw_handle(&painter, p_mid, r_mid.hovered() || r_mid.dragged());
    draw_handle(&painter, p_start, r_start.hovered() || r_start.dragged());
    draw_handle(&painter, p_end, r_end.hovered() || r_end.dragged());

    // Numeric readout under the plot.
    let txt = format!(
        "{:>4.0} → {:>3.0} Hz   {:.2} s   c={:.1}",
        start, end, decay, curve
    );
    painter.text(
        egui::pos2(rect.center().x, rect.bottom() - 2.0),
        egui::Align2::CENTER_BOTTOM,
        txt,
        egui::FontId::monospace(8.5),
        LABEL_DIM,
    );

    r_start.on_hover_text(format!("Start: {:.1} Hz", start));
    r_end.on_hover_text(format!("End: {:.1} Hz · Decay: {:.2} s", end, decay));
    r_mid.on_hover_text(format!("Curve: {:.2}", curve));
}

/// Graphical amplitude envelope editor.
///
/// Two draggable handles control amp_decay (right handle, horizontal position)
/// and amp_curve (middle handle, vertical position). The shape is drawn from
/// gain = (1 - t/decay)^curve over t ∈ [0, decay].
fn amp_env_editor(ui: &mut egui::Ui, params: &HardKickParams, setter: &ParamSetter) {
    let size = egui::vec2(160.0, 78.0);
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let pad = egui::vec2(6.0, 6.0);
    let plot = egui::Rect::from_min_max(rect.min + pad, rect.max - pad);

    const T_MIN: f32 = 0.05;
    const T_MAX: f32 = 2.0;
    let time_to_x = |t: f32| -> f32 {
        let n = (t.ln() - T_MIN.ln()) / (T_MAX.ln() - T_MIN.ln());
        plot.left() + n.clamp(0.0, 1.0) * plot.width()
    };
    let x_to_time = |x: f32| -> f32 {
        let n = ((x - plot.left()) / plot.width()).clamp(0.0, 1.0);
        (T_MIN.ln() + n * (T_MAX.ln() - T_MIN.ln())).exp()
    };

    let decay = params.amp_decay.value();
    let curve = params.amp_curve.value();

    let p_start = egui::pos2(plot.left(), plot.top());
    let p_end = egui::pos2(time_to_x(decay), plot.bottom());

    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, PANEL_BG);
    painter.rect_stroke(rect, 4.0, egui::Stroke::new(1.0, ACCENT_DIM), StrokeKind::Inside);

    const SEGS: usize = 80;
    let mut prev: Option<egui::Pos2> = None;
    for i in 0..=SEGS {
        let t = i as f32 / SEGS as f32;
        let g = (1.0 - t).powf(curve);
        let p = egui::pos2(
            plot.left() + t * (p_end.x - p_start.x),
            plot.bottom() - g * plot.height(),
        );
        if let Some(pp) = prev {
            painter.line_segment([pp, p], egui::Stroke::new(1.6, ACCENT));
        }
        prev = Some(p);
    }

    let mid_t = 0.5_f32;
    let mid_g = 0.5_f32.powf(curve);
    let p_mid = egui::pos2(
        plot.left() + mid_t * (p_end.x - p_start.x),
        plot.bottom() - mid_g * plot.height(),
    );

    let draw_handle = |painter: &egui::Painter, p: egui::Pos2, active: bool| {
        let r = 5.0;
        painter.circle_filled(p, r, if active { ACCENT } else { KNOB_BODY });
        painter.circle_stroke(p, r, egui::Stroke::new(1.2, ACCENT));
    };

    let hit = |p: egui::Pos2| egui::Rect::from_center_size(p, egui::vec2(16.0, 16.0));
    let id = ui.id().with("amp_env");
    let r_end = ui.interact(hit(p_end), id.with("end"), egui::Sense::click_and_drag());
    let r_mid = ui.interact(hit(p_mid), id.with("mid"), egui::Sense::click_and_drag());

    if r_end.drag_started() {
        setter.begin_set_parameter(&params.amp_decay);
    }
    if r_end.dragged() {
        if let Some(pos) = r_end.interact_pointer_pos() {
            setter.set_parameter(&params.amp_decay, x_to_time(pos.x).clamp(0.05, 2.0));
        }
    }
    if r_end.drag_stopped() {
        setter.end_set_parameter(&params.amp_decay);
    }
    if r_end.double_clicked() {
        setter.begin_set_parameter(&params.amp_decay);
        setter.set_parameter_normalized(
            &params.amp_decay,
            params.amp_decay.default_normalized_value(),
        );
        setter.end_set_parameter(&params.amp_decay);
    }

    if r_mid.drag_started() {
        setter.begin_set_parameter(&params.amp_curve);
    }
    if r_mid.dragged() {
        if let Some(pos) = r_mid.interact_pointer_pos() {
            let g = ((plot.bottom() - pos.y) / plot.height()).clamp(1e-4, 1.0 - 1e-4);
            let new_curve = g.ln() / 0.5_f32.ln();
            setter.set_parameter(&params.amp_curve, new_curve.clamp(0.1, 8.0));
        }
    }
    if r_mid.drag_stopped() {
        setter.end_set_parameter(&params.amp_curve);
    }
    if r_mid.double_clicked() {
        setter.begin_set_parameter(&params.amp_curve);
        setter.set_parameter_normalized(
            &params.amp_curve,
            params.amp_curve.default_normalized_value(),
        );
        setter.end_set_parameter(&params.amp_curve);
    }

    draw_handle(&painter, p_mid, r_mid.hovered() || r_mid.dragged());
    draw_handle(&painter, p_end, r_end.hovered() || r_end.dragged());

    let txt = format!("{:.2} s   c={:.1}", decay, curve);
    painter.text(
        egui::pos2(rect.center().x, rect.bottom() - 2.0),
        egui::Align2::CENTER_BOTTOM,
        txt,
        egui::FontId::monospace(8.5),
        LABEL_DIM,
    );

    r_end.on_hover_text(format!("Decay: {:.2} s", decay));
    r_mid.on_hover_text(format!("Curve: {:.2}", curve));
}

/// Two-dimensional drag pad — drives two FloatParams from a single point on a
/// square. The X axis writes `px`, the Y axis writes `py` (top = max). Both
/// use the params' own normalized 0..1 mapping.
fn xy_pad<P: Param, Q: Param>(
    ui: &mut egui::Ui,
    caption: &str,
    px: &P,
    py: &Q,
    setter: &ParamSetter,
) {
    let size = egui::vec2(78.0, 78.0);
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let pad = 4.0;
    let plot = egui::Rect::from_min_max(
        rect.min + egui::vec2(pad, pad),
        rect.max - egui::vec2(pad, pad + 10.0),
    );

    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, PANEL_BG);
    painter.rect_stroke(rect, 4.0, egui::Stroke::new(1.0, ACCENT_DIM), StrokeKind::Inside);

    let grid = egui::Color32::from_rgb(40, 32, 60);
    painter.line_segment(
        [egui::pos2(plot.center().x, plot.top()), egui::pos2(plot.center().x, plot.bottom())],
        egui::Stroke::new(0.5, grid),
    );
    painter.line_segment(
        [egui::pos2(plot.left(), plot.center().y), egui::pos2(plot.right(), plot.center().y)],
        egui::Stroke::new(0.5, grid),
    );

    let tx = px.unmodulated_normalized_value();
    let ty = py.unmodulated_normalized_value();
    let p = egui::pos2(
        plot.left() + tx * plot.width(),
        plot.bottom() - ty * plot.height(),
    );

    let resp = ui.interact(plot, ui.id().with(("xy", caption)), egui::Sense::click_and_drag());
    if resp.drag_started() {
        setter.begin_set_parameter(px);
        setter.begin_set_parameter(py);
    }
    if resp.dragged() || resp.clicked() {
        if let Some(pos) = resp.interact_pointer_pos() {
            let nx = ((pos.x - plot.left()) / plot.width()).clamp(0.0, 1.0);
            let ny = ((plot.bottom() - pos.y) / plot.height()).clamp(0.0, 1.0);
            setter.set_parameter_normalized(px, nx);
            setter.set_parameter_normalized(py, ny);
        }
    }
    if resp.drag_stopped() {
        setter.end_set_parameter(px);
        setter.end_set_parameter(py);
    }
    if resp.double_clicked() {
        setter.begin_set_parameter(px);
        setter.set_parameter_normalized(px, px.default_normalized_value());
        setter.end_set_parameter(px);
        setter.begin_set_parameter(py);
        setter.set_parameter_normalized(py, py.default_normalized_value());
        setter.end_set_parameter(py);
    }

    painter.circle_filled(p, 4.5, ACCENT);
    painter.circle_stroke(p, 4.5, egui::Stroke::new(1.0, LABEL));

    painter.text(
        egui::pos2(rect.center().x, rect.bottom() - 2.0),
        egui::Align2::CENTER_BOTTOM,
        caption,
        egui::FontId::monospace(8.5),
        LABEL,
    );

    resp.on_hover_text(format!(
        "{}: {} · {}: {}",
        px.name(),
        px.normalized_value_to_string(tx, true),
        py.name(),
        py.normalized_value_to_string(ty, true),
    ));
}

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
        // Onset ramp in preview too.
        let attack_samples = (params.amp_attack.value() * 0.001 * sr).max(1.0);
        let onset = ((i as f32 * step) / attack_samples).min(1.0);
        let amp = (1.0 - t_amp).powf(params.amp_curve.value()) * params.level.value() * onset;
        let osc = (phase * TAU).sin();
        let shaped = shape(osc, params.shaper.value(), params.drive.value(), params.bias.value());
        let mix = params.dist_mix.value();
        wave.push(((osc + (shaped - osc) * mix) * amp).clamp(-1.0, 1.0));
        phase = (phase + freq * step / sr).fract();
    }

    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 56.0), egui::Sense::hover());
    if !ui.is_rect_visible(rect) { return; }

    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, PANEL_BG);
    painter.rect_stroke(rect, 4.0, egui::Stroke::new(1.0, ACCENT_DIM), StrokeKind::Inside);

    let cy = rect.center().y;
    let h = rect.height() * 0.42;
    painter.line_segment(
        [egui::pos2(rect.left(), cy), egui::pos2(rect.right(), cy)],
        egui::Stroke::new(0.5, egui::Color32::from_rgb(55, 45, 75)),
    );
    let pts: Vec<egui::Pos2> = wave.iter().enumerate().map(|(i, &s)| {
        let x = rect.left() + i as f32 / (N - 1) as f32 * rect.width();
        egui::pos2(x, cy - s * h)
    }).collect();
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
                egui::ScrollArea::vertical().show(ui, |ui| {
                    // ── TITLE BAR ─────────────────────────────────────────────
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("HARDKICK")
                                .font(egui::FontId::proportional(16.0))
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
                            if ui.button("⚡ Trig").clicked() {
                                trigger.store(true, Ordering::Relaxed);
                            }
                            let is_playing = playing.load(Ordering::Relaxed);
                            if ui.button(if is_playing { "⏹ Stop" } else { "▶ Play" }).clicked() {
                                playing.store(!is_playing, Ordering::Relaxed);
                            }
                        });
                    });

                    ui.add_space(3.0);
                    kick_waveform(ui, &params);
                    ui.add_space(3.0);

                    // ── PRESETS ───────────────────────────────────────────────
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 3.0;
                        let n = presets::PRESETS.len() as f32;
                        let bw = (ui.available_width() - 3.0 * (n - 1.0)) / n;
                        for preset in presets::PRESETS {
                            let btn = egui::Button::new(
                                egui::RichText::new(preset.name)
                                    .font(egui::FontId::monospace(8.5))
                                    .color(ACCENT),
                            )
                            .fill(SECTION_BG)
                            .stroke(egui::Stroke::new(1.0, ACCENT_DIM))
                            .min_size(egui::vec2(bw, 19.0));
                            if ui.add(btn).clicked() {
                                presets::apply(preset, &params, setter);
                            }
                        }
                    });

                    ui.add_space(4.0);

                    // Each panel takes its natural width (knob count × cell width)
                    // instead of an equal third of the window — avoids overlap when
                    // panels have different knob counts.
                    let row_spacing = egui::vec2(4.0, 0.0);

                    // ── ROW 1: PITCH | AMP | ATTACK | SHAPE ──────────────────
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = row_spacing;

                        // PITCH — interactive curve editor (drag handles on the graph).
                        panel(ui, "PITCH ENV", |ui| {
                            pitch_env_editor(ui, &params, setter);
                        });

                        // AMP — graphical decay/curve editor + attack and master level.
                        panel(ui, "AMP ENV", |ui| {
                            amp_env_editor(ui, &params, setter);
                            knob(ui, "Atk", &params.amp_attack, setter);
                            knob(ui, "Level", &params.level, setter);
                        });

                        // ATTACK — transient click layer, mixed in parallel AFTER distortion.
                        panel(ui, "CLICK", |ui| {
                            knob(ui, "Level", &params.click_level, setter);
                            knob(ui, "Decay", &params.click_decay, setter);
                            knob(ui, "Tone", &params.click_tone, setter);
                        });
                    });

                    ui.add_space(4.0);

                    // ── ROW 2: SHAPE | EQ | OUTPUT | SEQ ─────────────────────
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = row_spacing;

                        // SHAPE — distortion applied to the HIGH band only (above Xover).
                        // The sub band always passes clean to preserve low-end punch.
                        panel(ui, "SHAPE  [high band only]", |ui| {
                            lever_switch(
                                ui, "MODE",
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
                            knob(ui, "Xover", &params.crossover_freq, setter);
                            xy_pad(ui, "DRIVE × BIAS", &params.drive, &params.bias, setter);
                            knob(ui, "Mix", &params.dist_mix, setter);
                        });

                        // EQ — pre-distortion peaking EQ generates the screech;
                        // Tone is a post-distortion high-shelf trim.
                        panel(ui, "EQ  [pre-dist]", |ui| {
                            knob(ui, "Freq", &params.pre_eq_freq, setter);
                            knob(ui, "Q", &params.pre_eq_q, setter);
                            knob(ui, "Gain", &params.pre_eq_gain, setter);
                            knob(ui, "Tone", &params.tone, setter);
                        });

                        // OUTPUT — oversampling factor and brickwall limiter.
                        panel(ui, "OUTPUT", |ui| {
                            lever_switch(
                                ui, "OS",
                                params.oversample.value(),
                                &[
                                    (OsFactor::Off, "OFF"),
                                    (OsFactor::X2, "2×"),
                                    (OsFactor::X4, "4×"),
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

                    });

                    ui.add_space(4.0);

                    // ── ROW 3: PUNCH | SEQ ──────────────────────────────────
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = row_spacing;

                        // PUNCH — short tonal burst summed into the oscillator
                        // pre-crossover, giving an extra percussive "klap" that
                        // also rides through the distortion chain.
                        panel(ui, "PUNCH  [pre-xover, distorted]", |ui| {
                            knob(ui, "Level", &params.punch_level, setter);
                            knob(ui, "Freq", &params.punch_freq, setter);
                            knob(ui, "Decay", &params.punch_decay, setter);
                            knob(ui, "Curve", &params.punch_curve, setter);
                        });

                        // COMP — body compressor on the distorted high band.
                        panel(ui, "COMP  [body]", |ui| {
                            knob(ui, "Thr", &params.comp_threshold, setter);
                            knob(ui, "Ratio", &params.comp_ratio, setter);
                            knob(ui, "Atk", &params.comp_attack, setter);
                            knob(ui, "Rel", &params.comp_release, setter);
                            knob(ui, "Makeup", &params.comp_makeup, setter);
                        });

                        // SEQ — standalone audition BPM.
                        panel(ui, "SEQ", |ui| {
                            knob(ui, "BPM", &params.bpm, setter);
                        });
                    });
                });
            });
        },
    )
}
