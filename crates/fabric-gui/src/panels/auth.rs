//! Auth panel — login status, user info, session management.

use crate::animation::AnimationState;
use crate::app::GuiData;
use crate::theme::LiquidTheme;
use crate::widgets;

/// Render the auth/status page.
pub fn show(ui: &mut egui::Ui, data: &GuiData, theme: &LiquidTheme, anim: &AnimationState) {
    let auth = &data.auth;

    // Header
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Auth").strong().size(18.0).color(theme.text_primary));
        ui.separator();
        let status_text = if auth.logged_in { "Authenticated" } else { "Not Authenticated" };
        let status_color = if auth.logged_in { theme.status_healthy } else { theme.status_error };
        widgets::status_pill(ui, status_text, status_color, theme);
    });
    widgets::animated_gradient_bar(ui, anim, 2.0, theme);
    ui.add_space(8.0);

    // Two-column layout
    ui.columns(2, |cols| {
        egui::ScrollArea::vertical()
            .id_salt("auth_left")
            .max_height(360.0)
            .show(&mut cols[0], |ui| {
                login_status_card(ui, auth, theme, anim);
                ui.add_space(8.0);
                user_info_card(ui, auth, theme);
            });
        egui::ScrollArea::vertical()
            .id_salt("auth_right")
            .max_height(360.0)
            .show(&mut cols[1], |ui| {
                session_expiry_card(ui, auth, theme, anim);
                ui.add_space(8.0);
                active_sessions_card(ui, auth, theme);
                ui.add_space(8.0);
                action_buttons(ui, auth, theme);
            });
    });
}

fn login_status_card(ui: &mut egui::Ui, auth: &crate::app::AuthStatus, theme: &LiquidTheme, anim: &AnimationState) {
    widgets::glass_card(ui, "Login Status", theme, |ui| {
        if auth.logged_in {
            let glow = anim.pulse_glow();
            ui.horizontal(|ui| {
                ui.colored_label(theme.status_healthy, "●");
                ui.label(egui::RichText::new("Connected to WorkOS").strong().color(theme.text_primary));
            });
            let _ = glow;
        } else {
            ui.horizontal(|ui| {
                ui.colored_label(theme.status_error, "●");
                ui.label(egui::RichText::new("No active session").color(theme.text_muted));
            });
            ui.add_space(6.0);
            if widgets::glass_button(ui, "🔐 WorkOS Login", theme) {
                tracing::info!("WorkOS login initiated");
            }
        }
    });
}

fn user_info_card(ui: &mut egui::Ui, auth: &crate::app::AuthStatus, theme: &LiquidTheme) {
    widgets::glass_card(ui, "User Info", theme, |ui| {
        if !auth.logged_in {
            ui.label(egui::RichText::new("Login to view user info").color(theme.text_muted));
            return;
        }
        info_row(ui, "Name:", &auth.user_name, theme);
        info_row(ui, "Email:", &auth.user_email, theme);
        info_row(ui, "Organization:", &auth.org_name, theme);
        ui.add_space(4.0);
        ui.label(egui::RichText::new("Roles:").color(theme.text_secondary));
        if auth.roles.is_empty() {
            ui.label(egui::RichText::new("  None").color(theme.text_muted));
        } else {
            for role in &auth.roles {
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new(format!("• {role}")).small().color(theme.text_primary));
                });
            }
        }
    });
}

fn session_expiry_card(ui: &mut egui::Ui, auth: &crate::app::AuthStatus, theme: &LiquidTheme, anim: &AnimationState) {
    widgets::glass_card(ui, "Session Expiry", theme, |ui| {
        if !auth.logged_in {
            ui.label(egui::RichText::new("No active session").color(theme.text_muted));
            return;
        }
        let mins = auth.session_expiry_secs / 60;
        let secs = auth.session_expiry_secs % 60;
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Expires in:").color(theme.text_secondary));
            let color = if auth.session_expiry_secs < 300 { theme.status_warning } else { theme.status_healthy };
            ui.label(egui::RichText::new(format!("{mins}m {secs}s")).strong().color(color));
        });
        ui.add_space(4.0);
        let progress = (auth.session_expiry_secs as f32 / 3600.0).clamp(0.0, 1.0);
        let bar_color = if auth.session_expiry_secs < 300 { theme.status_warning } else { theme.status_healthy };
        widgets::progress_ring(ui, progress, bar_color, 50.0, theme);
        let _ = anim;
    });
}

fn active_sessions_card(ui: &mut egui::Ui, auth: &crate::app::AuthStatus, theme: &LiquidTheme) {
    widgets::glass_card(ui, "Active Sessions", theme, |ui| {
        if auth.active_sessions.is_empty() {
            ui.label(egui::RichText::new("No other sessions").color(theme.text_muted));
            return;
        }
        for session in &auth.active_sessions {
            ui.horizontal(|ui| {
                ui.colored_label(theme.status_info, "●");
                ui.label(egui::RichText::new(&session.device).strong().color(theme.text_primary));
            });
            ui.horizontal(|ui| {
                ui.add_space(12.0);
                ui.label(egui::RichText::new(format!(
                    "{} | {}", session.session_id, session.created
                )).small().color(theme.text_muted));
            });
        }
    });
}

fn action_buttons(ui: &mut egui::Ui, auth: &crate::app::AuthStatus, theme: &LiquidTheme) {
    if auth.logged_in {
        if widgets::glass_button(ui, "🚪 Logout", theme) {
            tracing::info!("Logout requested");
        }
    }
}

fn info_row(ui: &mut egui::Ui, label: &str, value: &str, theme: &LiquidTheme) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).color(theme.text_secondary));
        ui.label(egui::RichText::new(value).strong().color(theme.text_primary));
    });
}
