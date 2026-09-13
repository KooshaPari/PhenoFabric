//! Settings panel — application, network, streaming, auth, secrets, about.

use crate::animation::AnimationState;
use crate::app::GuiData;
use crate::theme::LiquidTheme;
use crate::widgets;

/// Render the settings page.
pub fn show(ui: &mut egui::Ui, data: &GuiData, theme: &LiquidTheme, anim: &AnimationState) {
    let settings = &data.settings;

    ui.label(egui::RichText::new("Settings").strong().size(18.0).color(theme.text_primary));
    widgets::animated_gradient_bar(ui, anim, 2.0, theme);
    ui.add_space(8.0);

    egui::ScrollArea::vertical()
        .id_salt("settings_scroll")
        .show(ui, |ui| {
            general_section(ui, settings, theme);
            ui.add_space(8.0);
            network_section(ui, settings, theme);
            ui.add_space(8.0);
            streaming_section(ui, settings, theme);
            ui.add_space(8.0);
            auth_section(ui, &data.auth, theme);
            ui.add_space(8.0);
            secrets_section(ui, theme);
            ui.add_space(8.0);
            about_section(ui, theme);
        });
}

fn general_section(ui: &mut egui::Ui, s: &crate::app::SettingsState, theme: &LiquidTheme) {
    widgets::glass_card(ui, "General", theme, |ui| {
        setting_row(ui, "Theme:", &s.theme, "theme_settings", &["Dark", "Light"], theme);
        setting_row_number(ui, "Auto-refresh (s):", s.auto_refresh_secs, theme);
        setting_row(ui, "Startup Tab:", &s.startup_tab, "startup_tab",
            &["Dashboard", "Topology", "Routes", "Leases", "Network", "Streaming"], theme);
    });
}

fn network_section(ui: &mut egui::Ui, s: &crate::app::SettingsState, theme: &LiquidTheme) {
    widgets::glass_card(ui, "Network", theme, |ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Daemon Address:").color(theme.text_secondary));
            ui.add(egui::TextEdit::singleline(&mut String::from(&s.daemon_address))
                .desired_width(200.0));
        });
        ui.add_space(4.0);
        toggle_row(ui, "Tailscale:", s.tailscale_enabled, theme);
        toggle_row(ui, "UPnP:", s.upnp_enabled, theme);
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("STUN Server:").color(theme.text_secondary));
            ui.add(egui::TextEdit::singleline(&mut String::from(&s.stun_server))
                .desired_width(240.0));
        });
    });
}

fn streaming_section(ui: &mut egui::Ui, s: &crate::app::SettingsState, theme: &LiquidTheme) {
    widgets::glass_card(ui, "Streaming", theme, |ui| {
        setting_row(ui, "Default Codec:", &s.default_codec, "default_codec",
            &["H.264", "H.265", "VP9", "AV1"], theme);
        setting_row_number(ui, "Max Bitrate (kbps):", s.max_bitrate_kbps as u64, theme);
        setting_row_number(ui, "Keyframe Interval:", s.keyframe_interval as u64, theme);
    });
}

fn auth_section(ui: &mut egui::Ui, auth: &crate::app::AuthStatus, theme: &LiquidTheme) {
    widgets::glass_card(ui, "Authentication", theme, |ui| {
        if auth.logged_in {
            ui.horizontal(|ui| {
                ui.colored_label(theme.status_healthy, "●");
                ui.label(egui::RichText::new(format!("Logged in as {}", auth.user_name))
                    .color(theme.text_primary));
            });
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Session expires in:").color(theme.text_secondary));
                let color = if auth.session_expiry_secs < 300 { theme.status_warning } else { theme.text_primary };
                ui.label(egui::RichText::new(format!("{}s", auth.session_expiry_secs)).strong().color(color));
            });
        } else {
            ui.label(egui::RichText::new("Not authenticated").color(theme.text_muted));
        }
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if widgets::glass_button(ui, "🔐 WorkOS Login", theme) {
                tracing::info!("WorkOS login initiated");
            }
            if auth.logged_in && widgets::glass_button(ui, "Logout", theme) {
                tracing::info!("Logout requested");
            }
        });
    });
}

fn secrets_section(ui: &mut egui::Ui, theme: &LiquidTheme) {
    widgets::glass_card(ui, "Secrets (Infisical)", theme, |ui| {
        ui.horizontal(|ui| {
            ui.colored_label(theme.status_healthy, "●");
            ui.label(egui::RichText::new("Connected").color(theme.text_primary));
        });
        ui.add_space(4.0);
        if widgets::glass_button(ui, "↻ Refresh Secrets", theme) {
            tracing::info!("Refresh secrets requested");
        }
    });
}

fn about_section(ui: &mut egui::Ui, theme: &LiquidTheme) {
    widgets::glass_card(ui, "About", theme, |ui| {
        ui.label(egui::RichText::new("Phenotype Fabric").strong().color(theme.text_primary));
        ui.label(egui::RichText::new("Version 0.1.0").small().color(theme.text_secondary));
        ui.add_space(2.0);
        ui.hyperlink_to(
            egui::RichText::new("github.com/phenotype/fabric").color(theme.accent_secondary),
            "https://github.com/phenotype/fabric",
        );
    });
}

fn setting_row(
    ui: &mut egui::Ui, label: &str, current: &str, id: &str, options: &[&str], theme: &LiquidTheme,
) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).color(theme.text_secondary));
        egui::ComboBox::from_id_salt(id)
            .selected_text(current)
            .show_ui(ui, |ui| {
                for opt in options {
                    ui.selectable_label(current == *opt, *opt);
                }
            });
    });
}

fn setting_row_number(ui: &mut egui::Ui, label: &str, value: u64, theme: &LiquidTheme) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).color(theme.text_secondary));
        ui.label(egui::RichText::new(value.to_string()).strong().color(theme.text_primary));
    });
}

fn toggle_row(ui: &mut egui::Ui, label: &str, enabled: bool, theme: &LiquidTheme) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).color(theme.text_secondary));
        let color = if enabled { theme.status_healthy } else { theme.status_error };
        let text = if enabled { "ON" } else { "OFF" };
        ui.label(egui::RichText::new(text).strong().color(color));
    });
}
