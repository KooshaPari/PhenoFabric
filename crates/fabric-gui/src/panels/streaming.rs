//! Streaming panel — session list, codec config, real-time stats, quality gauge.

use crate::animation::AnimationState;
use crate::app::GuiData;
use crate::theme::LiquidTheme;
use crate::widgets;

/// Render the streaming control page.
pub fn show(ui: &mut egui::Ui, data: &GuiData, theme: &LiquidTheme, anim: &AnimationState) {
    let stats = &data.streaming;

    // Header
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Streaming").strong().size(18.0).color(theme.text_primary));
        ui.separator();
        let count = stats.active_sessions.len();
        ui.label(egui::RichText::new(format!("{count} active session(s)")).color(theme.text_secondary));
    });
    widgets::animated_gradient_bar(ui, anim, 2.0, theme);
    ui.add_space(8.0);

    // Top row: quality gauge + real-time stats
    ui.columns(2, |cols| {
        quality_gauge_card(&mut cols[0], stats, theme, anim);
        real_time_stats_card(&mut cols[1], stats, theme);
    });
    ui.add_space(8.0);

    // Session list + config side by side
    ui.columns(2, |cols| {
        egui::ScrollArea::vertical()
            .id_salt("stream_sessions")
            .max_height(240.0)
            .show(&mut cols[0], |ui| {
                session_list(ui, stats, theme, anim);
            });
        egui::ScrollArea::vertical()
            .id_salt("stream_config")
            .max_height(240.0)
            .show(&mut cols[1], |ui| {
                config_panel(ui, stats, theme);
            });
    });
    ui.add_space(8.0);

    // Bottom: control buttons
    control_buttons(ui, theme);
}

fn quality_gauge_card(ui: &mut egui::Ui, stats: &crate::app::StreamingStats, theme: &LiquidTheme, anim: &AnimationState) {
    widgets::glass_card(ui, "Quality", theme, |ui| {
        let (label, color) = quality_from_stats(stats, theme);
        // Progress ring
        let progress = quality_progress(stats);
        widgets::progress_ring(ui, progress, color, 60.0, theme);
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.colored_label(color, "●");
            ui.label(egui::RichText::new(label).strong().size(16.0).color(color));
        });
        let _ = anim; // used for future pulse effects
    });
}

fn quality_from_stats(
    stats: &crate::app::StreamingStats, theme: &LiquidTheme,
) -> (&'static str, egui::Color32) {
    let drop_rate = if stats.frames_sent > 0 {
        stats.frames_dropped as f64 / stats.frames_sent as f64
    } else { 0.0 };
    if drop_rate < 0.01 && stats.latency_ms < 20.0 {
        ("Good", theme.status_healthy)
    } else if drop_rate < 0.05 && stats.latency_ms < 50.0 {
        ("Fair", theme.status_warning)
    } else {
        ("Poor", theme.status_error)
    }
}

fn quality_progress(stats: &crate::app::StreamingStats) -> f32 {
    let drop_rate = if stats.frames_sent > 0 {
        stats.frames_dropped as f32 / stats.frames_sent as f32
    } else { 0.0 };
    (1.0 - drop_rate).clamp(0.0, 1.0)
}

fn real_time_stats_card(ui: &mut egui::Ui, stats: &crate::app::StreamingStats, theme: &LiquidTheme) {
    widgets::glass_card(ui, "Real-Time Stats", theme, |ui| {
        stat_row(ui, "Frames Sent", &stats.frames_sent.to_string(), theme);
        stat_row(ui, "Frames Dropped", &stats.frames_dropped.to_string(), theme);
        stat_row(ui, "Latency", &format!("{:.1}ms", stats.latency_ms), theme);
        stat_row(ui, "Bandwidth", &format!("{:.2} Mbps", stats.bandwidth_mbps), theme);
    });
}

fn stat_row(ui: &mut egui::Ui, label: &str, value: &str, theme: &LiquidTheme) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).color(theme.text_secondary));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(egui::RichText::new(value).strong().color(theme.text_primary));
        });
    });
}

fn session_list(ui: &mut egui::Ui, stats: &crate::app::StreamingStats, theme: &LiquidTheme, _anim: &AnimationState) {
    ui.label(egui::RichText::new("Active Sessions").strong().size(14.0).color(theme.text_primary));
    ui.add_space(4.0);
    if stats.active_sessions.is_empty() {
        ui.label(egui::RichText::new("No active streams").color(theme.text_muted));
        return;
    }
    for session in &stats.active_sessions {
        widgets::glass_card(ui, "", theme, |ui| {
            ui.horizontal(|ui| {
                let state_color = if session.state == "Active" { theme.status_healthy } else { theme.status_warning };
                ui.colored_label(state_color, "●");
                ui.label(egui::RichText::new(&session.target).strong().color(theme.text_primary));
            });
            ui.label(egui::RichText::new(format!(
                "{} | {} | {}", session.codec, session.resolution, session.state
            )).small().color(theme.text_secondary));
        });
        ui.add_space(4.0);
    }
}

fn config_panel(ui: &mut egui::Ui, stats: &crate::app::StreamingStats, theme: &LiquidTheme) {
    ui.label(egui::RichText::new("Stream Config").strong().size(14.0).color(theme.text_primary));
    ui.add_space(4.0);

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Codec:").color(theme.text_secondary));
        egui::ComboBox::from_id_salt("codec_select")
            .selected_text(&stats.codec)
            .show_ui(ui, |ui| {
                for c in &["H.264", "H.265", "VP9", "AV1"] {
                    ui.selectable_value(&mut String::from(&stats.codec), c.to_string(), *c);
                }
            });
    });
    ui.add_space(2.0);

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Resolution:").color(theme.text_secondary));
        egui::ComboBox::from_id_salt("res_select")
            .selected_text(&stats.resolution)
            .show_ui(ui, |ui| {
                for r in &["1280x720", "1920x1080", "2560x1440", "3840x2160"] {
                    ui.selectable_value(&mut String::from(&stats.resolution), r.to_string(), *r);
                }
            });
    });
    ui.add_space(2.0);

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("FPS:").color(theme.text_secondary));
        ui.label(egui::RichText::new(stats.fps.to_string()).strong().color(theme.text_primary));
    });
    ui.add_space(2.0);

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Bitrate:").color(theme.text_secondary));
        ui.label(egui::RichText::new(format!("{} kbps", stats.bitrate_kbps)).strong().color(theme.text_primary));
    });
}

fn control_buttons(ui: &mut egui::Ui, theme: &LiquidTheme) {
    ui.horizontal(|ui| {
        if widgets::glass_button(ui, "🖥 Input Grab", theme) {
            tracing::info!("Input grab toggled");
        }
        if widgets::glass_button(ui, "⛶ Fullscreen", theme) {
            tracing::info!("Fullscreen toggled");
        }
        if widgets::glass_button(ui, "📸 Screenshot", theme) {
            tracing::info!("Screenshot requested");
        }
    });
}
