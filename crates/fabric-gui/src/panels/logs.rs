//! Logs panel — daemon log viewer with level filter, search, auto-scroll.

use crate::animation::AnimationState;
use crate::app::GuiData;
use crate::theme::LiquidTheme;
use crate::widgets;

/// Render the log viewer page.
pub fn show(ui: &mut egui::Ui, data: &GuiData, theme: &LiquidTheme, anim: &AnimationState, log_state: &mut LogViewState) {
    // Header
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Logs").strong().size(18.0).color(theme.text_primary));
        ui.separator();
        ui.label(egui::RichText::new(format!("{} entries", data.logs.len()))
            .color(theme.text_secondary));
    });
    widgets::animated_gradient_bar(ui, anim, 2.0, theme);
    ui.add_space(4.0);

    // Toolbar
    toolbar(ui, log_state, theme);
    ui.add_space(4.0);

    // Log entries
    let filtered: Vec<_> = data.logs.iter()
        .filter(|e| matches_level(&e.level, &log_state.level_filter))
        .filter(|e| {
            log_state.search.is_empty()
                || e.message.to_lowercase().contains(&log_state.search.to_lowercase())
        })
        .collect();

    egui::ScrollArea::vertical()
        .id_salt("log_entries")
        .auto_shrink([false, false])
        .stick_to_bottom(log_state.auto_scroll)
        .show(ui, |ui| {
            if filtered.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(40.0);
                    ui.label(egui::RichText::new("No log entries match filter").color(theme.text_muted));
                });
                return;
            }
            for entry in &filtered {
                log_entry_row(ui, entry, theme);
            }
        });
}

/// Mutable state for the logs panel.
pub struct LogViewState {
    pub level_filter: String,
    pub search: String,
    pub auto_scroll: bool,
}

impl Default for LogViewState {
    fn default() -> Self {
        Self {
            level_filter: "ALL".into(), search: String::new(), auto_scroll: true,
        }
    }
}

fn toolbar(ui: &mut egui::Ui, state: &mut LogViewState, theme: &LiquidTheme) {
    widgets::glass_panel(ui, theme, |ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Level:").color(theme.text_secondary));
            egui::ComboBox::from_id_salt("log_level")
                .selected_text(&state.level_filter)
                .show_ui(ui, |ui| {
                    for level in &["ALL", "DEBUG", "INFO", "WARN", "ERROR"] {
                        ui.selectable_value(&mut state.level_filter, level.to_string(), *level);
                    }
                });
            ui.separator();
            ui.label(egui::RichText::new("🔍").color(theme.text_secondary));
            ui.add(egui::TextEdit::singleline(&mut state.search)
                .desired_width(180.0)
                .hint_text(egui::RichText::new("Filter logs...").color(theme.text_muted)));
            ui.separator();
            let as_text = if state.auto_scroll { "⬇ Auto-scroll ON" } else { "⬇ Auto-scroll OFF" };
            if ui.selectable_label(state.auto_scroll, as_text).clicked() {
                state.auto_scroll = !state.auto_scroll;
            }
            ui.separator();
            if widgets::glass_button(ui, "✕ Clear", theme) {
                tracing::info!("Log clear requested");
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if widgets::glass_button(ui, "💾 Export", theme) {
                    tracing::info!("Log export requested");
                }
            });
        });
    });
}

fn log_entry_row(ui: &mut egui::Ui, entry: &crate::app::LogEntry, theme: &LiquidTheme) {
    ui.horizontal(|ui| {
        let color = level_color(&entry.level, theme);
        ui.colored_label(color, format!("{:>5}", entry.level));
        ui.label(egui::RichText::new(&entry.timestamp).small().color(theme.text_muted));
        ui.label(egui::RichText::new(&entry.message).color(theme.text_primary));
    });
}

fn level_color(level: &str, theme: &LiquidTheme) -> egui::Color32 {
    match level {
        "DEBUG" => theme.text_muted,
        "INFO" => theme.status_info,
        "WARN" => theme.status_warning,
        "ERROR" => theme.status_error,
        _ => theme.text_secondary,
    }
}

fn matches_level(entry_level: &str, filter: &str) -> bool {
    filter == "ALL" || entry_level.eq_ignore_ascii_case(filter)
}
