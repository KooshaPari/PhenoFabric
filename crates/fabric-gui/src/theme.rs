//! Design system and theme for the Fabric GUI.
//!
//! Centralizes brand colors, node/edge locality colors, and status indicators
//! so every panel stays visually consistent.

use egui::Color32;

/// Full color palette used throughout the GUI.
pub struct FabricTheme {
    // Backgrounds
    pub bg_primary: Color32,
    pub bg_secondary: Color32,
    pub bg_card: Color32,

    // Accent
    pub accent: Color32,
    pub accent_hover: Color32,

    // Text
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub text_muted: Color32,

    // Borders
    pub border: Color32,
    pub border_subtle: Color32,

    // Semantic
    pub success: Color32,
    pub warning: Color32,
    pub error: Color32,
    pub info: Color32,

    // Topology node colors
    pub node_gpu: Color32,
    pub node_cpu: Color32,
    pub node_io: Color32,
    pub node_default: Color32,

    // Topology edge colors
    pub edge_same_numa: Color32,
    pub edge_cross_numa: Color32,
    pub edge_cross_socket: Color32,
    pub edge_default: Color32,
}

impl FabricTheme {
    /// Dark theme (default).
    pub fn dark() -> Self {
        Self {
            bg_primary: Color32::from_rgb(15, 17, 23),
            bg_secondary: Color32::from_rgb(24, 27, 35),
            bg_card: Color32::from_rgb(30, 34, 44),
            accent: Color32::from_rgb(99, 140, 255),
            accent_hover: Color32::from_rgb(120, 160, 255),
            text_primary: Color32::from_rgb(230, 233, 240),
            text_secondary: Color32::from_rgb(156, 163, 185),
            text_muted: Color32::from_rgb(95, 104, 130),
            border: Color32::from_rgb(50, 56, 72),
            border_subtle: Color32::from_rgb(38, 42, 56),
            success: Color32::from_rgb(34, 197, 94),
            warning: Color32::from_rgb(234, 179, 8),
            error: Color32::from_rgb(239, 68, 68),
            info: Color32::from_rgb(59, 130, 246),
            node_gpu: Color32::from_rgb(0, 210, 255),
            node_cpu: Color32::from_rgb(52, 211, 153),
            node_io: Color32::from_rgb(251, 146, 60),
            node_default: Color32::from_rgb(156, 163, 175),
            edge_same_numa: Color32::from_rgb(52, 211, 153),
            edge_cross_numa: Color32::from_rgb(251, 191, 36),
            edge_cross_socket: Color32::from_rgb(248, 113, 113),
            edge_default: Color32::from_rgb(156, 163, 175),
        }
    }

    /// Light theme.
    pub fn light() -> Self {
        Self {
            bg_primary: Color32::from_rgb(245, 247, 250),
            bg_secondary: Color32::from_rgb(255, 255, 255),
            bg_card: Color32::from_rgb(255, 255, 255),
            accent: Color32::from_rgb(59, 110, 220),
            accent_hover: Color32::from_rgb(79, 130, 240),
            text_primary: Color32::from_rgb(17, 24, 39),
            text_secondary: Color32::from_rgb(107, 114, 128),
            text_muted: Color32::from_rgb(156, 163, 175),
            border: Color32::from_rgb(209, 213, 219),
            border_subtle: Color32::from_rgb(229, 231, 235),
            success: Color32::from_rgb(22, 163, 74),
            warning: Color32::from_rgb(202, 138, 4),
            error: Color32::from_rgb(220, 38, 38),
            info: Color32::from_rgb(37, 99, 235),
            node_gpu: Color32::from_rgb(0, 180, 220),
            node_cpu: Color32::from_rgb(16, 163, 107),
            node_io: Color32::from_rgb(220, 110, 30),
            node_default: Color32::from_rgb(107, 114, 128),
            edge_same_numa: Color32::from_rgb(16, 163, 107),
            edge_cross_numa: Color32::from_rgb(202, 138, 4),
            edge_cross_socket: Color32::from_rgb(220, 38, 38),
            edge_default: Color32::from_rgb(107, 114, 128),
        }
    }

    /// Color for a health status indicator.
    pub fn status_color(&self, healthy: bool) -> Color32 {
        if healthy { self.success } else { self.error }
    }

    /// Color for a topology node by locality code.
    pub fn node_color(&self, locality: &str) -> Color32 {
        match locality {
            "GPU" | "gpu" | "G" => self.node_gpu,
            "CPU" | "cpu" | "C" => self.node_cpu,
            "IO" | "io" | "I" => self.node_io,
            _ => self.node_default,
        }
    }

    /// Color for an edge by locality code.
    pub fn edge_color(&self, locality: &str) -> Color32 {
        match locality {
            "S" | "s" | "same_numa" => self.edge_same_numa,
            "N" | "n" | "cross_numa" => self.edge_cross_numa,
            "X" | "x" | "cross_socket" => self.edge_cross_socket,
            _ => self.edge_default,
        }
    }

    /// Color that encodes latency: green < 0.5 ms, yellow < 2 ms, red otherwise.
    pub fn latency_color(&self, ms: f64) -> Color32 {
        if ms < 0.5 {
            self.success
        } else if ms < 2.0 {
            self.warning
        } else {
            self.error
        }
    }

    /// Color for a lease state string.
    pub fn lease_state_color(&self, state: &str) -> Color32 {
        if state.contains("Active") {
            self.success
        } else if state.contains("Pending") || state.contains("Provisioning") {
            self.warning
        } else if state.contains("Failed")
            || state.contains("Revoked")
            || state.contains("Expired")
        {
            self.error
        } else {
            self.text_muted
        }
    }
}

impl Default for FabricTheme {
    fn default() -> Self {
        Self::dark()
    }
}
