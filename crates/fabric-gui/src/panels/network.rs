//! Network panel — connection status, Tailscale peers, UPnP mappings, NAT type.

use crate::animation::AnimationState;
use crate::app::GuiData;
use crate::theme::LiquidTheme;
use crate::widgets;

/// Render the network status page.
pub fn show(ui: &mut egui::Ui, data: &GuiData, theme: &LiquidTheme, anim: &AnimationState) {
    let net = &data.network;

    // Header
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Network").strong().size(18.0).color(theme.text_primary));
        ui.separator();
        let (color, text) = if net.daemon_connected {
            (theme.status_healthy, "Connected")
        } else {
            (theme.status_error, "Disconnected")
        };
        widgets::status_pill(ui, text, color, theme);
    });
    widgets::animated_gradient_bar(ui, anim, 2.0, theme);
    ui.add_space(8.0);

    // Connection status cards
    ui.horizontal(|ui| {
        widgets::glass_card(ui, "Daemon", theme, |ui| {
            connection_indicator(ui, net.daemon_connected, theme, anim);
        });
        ui.add_space(6.0);
        widgets::glass_card(ui, "Tailscale", theme, |ui| {
            connection_indicator(ui, net.tailscale_connected, theme, anim);
        });
        ui.add_space(6.0);
        widgets::glass_card(ui, "UPnP", theme, |ui| {
            connection_indicator(ui, net.upnp_active, theme, anim);
        });
    });
    ui.add_space(8.0);

    // Two-column layout
    ui.columns(2, |cols| {
        egui::ScrollArea::vertical()
            .id_salt("net_left")
            .max_height(320.0)
            .show(&mut cols[0], |ui| {
                tailscale_section(ui, net, theme, anim);
                ui.add_space(8.0);
                upnp_section(ui, net, theme);
            });
        egui::ScrollArea::vertical()
            .id_salt("net_right")
            .max_height(320.0)
            .show(&mut cols[1], |ui| {
                nat_section(ui, net, theme);
                ui.add_space(8.0);
                topology_section(ui, data, theme, anim);
                ui.add_space(8.0);
                manual_connect_section(ui, theme);
            });
    });
}

fn connection_indicator(ui: &mut egui::Ui, ok: bool, theme: &LiquidTheme, anim: &AnimationState) {
    let color = if ok { theme.status_healthy } else { theme.status_error };
    let text = if ok { "Active" } else { "Inactive" };
    let glow = anim.pulse_glow();
    if ok {
        let painter = ui.painter();
        let dot_center = ui.cursor().left_center() + egui::vec2(8.0, 0.0);
        painter.circle_filled(dot_center, 8.0 + glow * 2.0, color.linear_multiply(glow * 0.2));
    }
    ui.horizontal(|ui| {
        ui.colored_label(color, "●");
        ui.label(egui::RichText::new(text).strong().color(theme.text_primary));
    });
}

fn tailscale_section(ui: &mut egui::Ui, net: &crate::app::NetworkStatus, theme: &LiquidTheme, _anim: &AnimationState) {
    ui.label(egui::RichText::new("Tailscale Peers").strong().size(14.0).color(theme.text_primary));
    ui.add_space(4.0);
    if net.tailscale_peers.is_empty() {
        ui.label(egui::RichText::new("No peers connected").color(theme.text_muted));
        return;
    }
    for peer in &net.tailscale_peers {
        widgets::glass_card(ui, "", theme, |ui| {
            ui.horizontal(|ui| {
                let color = if peer.online { theme.status_healthy } else { theme.status_error };
                ui.colored_label(color, "●");
                ui.label(egui::RichText::new(&peer.hostname).strong().color(theme.text_primary));
                ui.label(egui::RichText::new(&peer.ip).small().color(theme.text_muted));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let lat_color = theme.latency_color(peer.latency_ms);
                    ui.label(egui::RichText::new(format!("{:.1}ms", peer.latency_ms)).small().color(lat_color));
                });
            });
        });
        ui.add_space(4.0);
    }
}

fn upnp_section(ui: &mut egui::Ui, net: &crate::app::NetworkStatus, theme: &LiquidTheme) {
    ui.label(egui::RichText::new("UPnP Port Mappings").strong().size(14.0).color(theme.text_primary));
    ui.add_space(4.0);
    if net.upnp_mappings.is_empty() {
        ui.label(egui::RichText::new("No active mappings").color(theme.text_muted));
        return;
    }
    for mapping in &net.upnp_mappings {
        ui.horizontal(|ui| {
            ui.colored_label(theme.accent_secondary, "⇄");
            ui.label(&mapping.protocol);
            ui.label(egui::RichText::new(format!(
                "{}: {} → {}", mapping.description, mapping.internal_port, mapping.external_port
            )).small().color(theme.text_secondary));
        });
    }
}

fn nat_section(ui: &mut egui::Ui, net: &crate::app::NetworkStatus, theme: &LiquidTheme) {
    widgets::glass_card(ui, "NAT / Public IP", theme, |ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("NAT Type:").color(theme.text_secondary));
            ui.label(egui::RichText::new(&net.nat_type).strong().color(theme.text_primary));
        });
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Public IP:").color(theme.text_secondary));
            ui.label(egui::RichText::new(&net.public_ip).strong().color(theme.accent_secondary));
        });
    });
}

fn topology_section(ui: &mut egui::Ui, data: &GuiData, theme: &LiquidTheme, anim: &AnimationState) {
    ui.label(egui::RichText::new("Network Topology").strong().size(14.0).color(theme.text_primary));
    ui.add_space(4.0);
    if data.topology.nodes.is_empty() {
        ui.label(egui::RichText::new("No topology data").color(theme.text_muted));
        return;
    }
    widgets::mini_topology(ui, &data.topology.nodes, &data.topology.edges, theme, anim, 180.0);
}

fn manual_connect_section(ui: &mut egui::Ui, theme: &LiquidTheme) {
    ui.label(egui::RichText::new("Manual Connect").strong().size(14.0).color(theme.text_primary));
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        let mut addr = String::new();
        ui.label(egui::RichText::new("Peer address:").color(theme.text_secondary));
        widgets::glass_text_input(ui, "ip:port", &mut addr, theme);
        if widgets::glass_button(ui, "Connect", theme) {
            tracing::info!("Manual connect to: {addr}");
        }
    });
}
