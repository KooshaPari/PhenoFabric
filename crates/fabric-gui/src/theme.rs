//! Liquid glass design system for the Fabric GUI.
//!
//! Simulates frosted-glass / morphic surfaces using layered semi-transparent fills,
//! gradient backgrounds, soft shadows, and glow strokes.

use egui::{Color32, Frame, Margin, CornerRadius, Shadow, Stroke};

/// Liquid glass color palette -- dark variant.
pub struct LiquidTheme {
    // Glass surfaces
    pub glass_bg: Color32,
    pub glass_bg_light: Color32,
    pub glass_bg_vibrant: Color32,
    pub glass_border: Color32,
    pub glass_border_inner: Color32,

    // Vivid accents
    pub accent_primary: Color32,
    pub accent_secondary: Color32,
    pub accent_tertiary: Color32,
    pub accent_gradient_start: Color32,
    pub accent_gradient_end: Color32,

    // Glow effects
    pub glow_accent: Color32,
    pub glow_success: Color32,
    pub glow_error: Color32,

    // Text
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub text_muted: Color32,
    pub text_glow: Color32,

    // Morphic node colors
    pub node_gpu: Color32,
    pub node_cpu: Color32,
    pub node_io: Color32,
    pub node_network: Color32,
    pub node_default: Color32,

    // Topology edge colors
    pub edge_same_numa: Color32,
    pub edge_cross_numa: Color32,
    pub edge_cross_socket: Color32,
    pub edge_default: Color32,

    // Status
    pub status_healthy: Color32,
    pub status_warning: Color32,
    pub status_error: Color32,
    pub status_info: Color32,

    // Backgrounds
    pub bg_canvas: Color32,
    pub bg_sidebar: Color32,

    // Selection / hover
    pub hover_glass: Color32,
    pub active_glass: Color32,
}

impl LiquidTheme {
    /// Dark liquid glass theme (default).
    pub fn dark() -> Self {
        Self {
            glass_bg: rgba(30, 35, 50, 180),
            glass_bg_light: rgba(45, 50, 70, 160),
            glass_bg_vibrant: rgba(60, 70, 110, 200),
            glass_border: rgba(255, 255, 255, 30),
            glass_border_inner: rgba(255, 255, 255, 15),

            accent_primary: Color32::from_rgb(108, 99, 255),
            accent_secondary: Color32::from_rgb(0, 212, 255),
            accent_tertiary: Color32::from_rgb(255, 107, 157),
            accent_gradient_start: Color32::from_rgb(108, 99, 255),
            accent_gradient_end: Color32::from_rgb(0, 212, 255),

            glow_accent: rgba(108, 99, 255, 80),
            glow_success: rgba(34, 197, 94, 80),
            glow_error: rgba(239, 68, 68, 80),

            text_primary: Color32::from_rgb(230, 233, 240),
            text_secondary: Color32::from_rgb(156, 163, 185),
            text_muted: Color32::from_rgb(95, 104, 130),
            text_glow: Color32::from_rgb(140, 135, 255),

            node_gpu: Color32::from_rgb(0, 212, 255),
            node_cpu: Color32::from_rgb(52, 211, 153),
            node_io: Color32::from_rgb(255, 159, 67),
            node_network: Color32::from_rgb(99, 140, 255),
            node_default: Color32::from_rgb(156, 163, 175),

            edge_same_numa: Color32::from_rgb(52, 211, 153),
            edge_cross_numa: Color32::from_rgb(255, 196, 61),
            edge_cross_socket: Color32::from_rgb(255, 107, 107),
            edge_default: Color32::from_rgb(156, 163, 175),

            status_healthy: Color32::from_rgb(34, 197, 94),
            status_warning: Color32::from_rgb(234, 179, 8),
            status_error: Color32::from_rgb(239, 68, 68),
            status_info: Color32::from_rgb(99, 140, 255),

            bg_canvas: Color32::from_rgb(12, 14, 20),
            bg_sidebar: rgba(18, 22, 34, 220),

            hover_glass: rgba(255, 255, 255, 20),
            active_glass: rgba(108, 99, 255, 40),
        }
    }

    /// Light liquid glass theme.
    pub fn light() -> Self {
        Self {
            glass_bg: rgba(255, 255, 255, 180),
            glass_bg_light: rgba(245, 247, 250, 170),
            glass_bg_vibrant: rgba(230, 235, 255, 200),
            glass_border: rgba(0, 0, 0, 25),
            glass_border_inner: rgba(255, 255, 255, 100),

            accent_primary: Color32::from_rgb(88, 80, 236),
            accent_secondary: Color32::from_rgb(0, 180, 220),
            accent_tertiary: Color32::from_rgb(230, 80, 130),
            accent_gradient_start: Color32::from_rgb(88, 80, 236),
            accent_gradient_end: Color32::from_rgb(0, 180, 220),

            glow_accent: rgba(88, 80, 236, 50),
            glow_success: rgba(22, 163, 74, 50),
            glow_error: rgba(220, 38, 38, 50),

            text_primary: Color32::from_rgb(17, 24, 39),
            text_secondary: Color32::from_rgb(107, 114, 128),
            text_muted: Color32::from_rgb(156, 163, 175),
            text_glow: Color32::from_rgb(88, 80, 236),

            node_gpu: Color32::from_rgb(0, 180, 220),
            node_cpu: Color32::from_rgb(16, 163, 107),
            node_io: Color32::from_rgb(220, 110, 30),
            node_network: Color32::from_rgb(37, 99, 235),
            node_default: Color32::from_rgb(107, 114, 128),

            edge_same_numa: Color32::from_rgb(16, 163, 107),
            edge_cross_numa: Color32::from_rgb(202, 138, 4),
            edge_cross_socket: Color32::from_rgb(220, 38, 38),
            edge_default: Color32::from_rgb(107, 114, 128),

            status_healthy: Color32::from_rgb(22, 163, 74),
            status_warning: Color32::from_rgb(202, 138, 4),
            status_error: Color32::from_rgb(220, 38, 38),
            status_info: Color32::from_rgb(37, 99, 235),

            bg_canvas: Color32::from_rgb(240, 242, 246),
            bg_sidebar: rgba(250, 251, 253, 220),

            hover_glass: rgba(0, 0, 0, 15),
            active_glass: rgba(88, 80, 236, 25),
        }
    }

    /// Glass card frame with rounded corners and soft shadow.
    pub fn glass_frame(radius: u8) -> Frame {
        Frame::new()
            .fill(Color32::from_rgba_premultiplied(30, 35, 50, 180))
            .corner_radius(CornerRadius::same(radius))
            .stroke(Stroke::new(1.0_f32, rgba(255, 255, 255, 30)))
            .shadow(Shadow {
                offset: [0, 4],
                blur: 12,
                spread: 0,
                color: rgba(0, 0, 0, 60),
            })
            .inner_margin(Margin::same(14))
    }

    /// Accent-colored glass frame (vibrant variant).
    pub fn glass_frame_vibrant(radius: u8) -> Frame {
        Frame::new()
            .fill(Color32::from_rgba_premultiplied(60, 70, 110, 200))
            .corner_radius(CornerRadius::same(radius))
            .stroke(Stroke::new(1.5_f32, rgba(108, 99, 255, 60)))
            .shadow(Shadow {
                offset: [0, 4],
                blur: 16,
                spread: 0,
                color: rgba(108, 99, 255, 30),
            })
            .inner_margin(Margin::same(14))
    }

    /// Frame with inner glow stroke.
    pub fn glow_frame(color: Color32) -> Frame {
        Frame::new()
            .fill(Color32::from_rgba_premultiplied(30, 35, 50, 180))
            .corner_radius(CornerRadius::same(12))
            .stroke(Stroke::new(1.0_f32, color.linear_multiply(0.5)))
            .shadow(Shadow {
                offset: [0, 0],
                blur: 8,
                spread: 0,
                color: color.linear_multiply(0.25),
            })
            .inner_margin(Margin::same(14))
    }

    /// Heavy frosted glass: deeper shadow, stronger inner stroke, more opacity.
    pub fn heavy_glass_frame(radius: u8) -> Frame {
        Frame::new()
            .fill(Color32::from_rgba_premultiplied(22, 26, 40, 210))
            .corner_radius(CornerRadius::same(radius))
            .stroke(Stroke::new(1.5_f32, rgba(255, 255, 255, 45)))
            .shadow(Shadow {
                offset: [0, 8],
                blur: 24,
                spread: 2,
                color: rgba(0, 0, 0, 90),
            })
            .inner_margin(Margin::same(16))
    }

    /// Translucent sidebar with edge highlight.
    pub fn sidebar_glass_frame() -> Frame {
        Frame::new()
            .fill(Color32::from_rgba_premultiplied(14, 18, 28, 230))
            .corner_radius(CornerRadius::ZERO)
            .stroke(Stroke::new(1.0_f32, rgba(255, 255, 255, 22)))
            .shadow(Shadow {
                offset: [2, 0],
                blur: 16,
                spread: 0,
                color: rgba(0, 0, 0, 40),
            })
            .inner_margin(Margin::symmetric(12, 14))
    }

    /// Centered modal with outer glow.
    pub fn modal_glass_frame() -> Frame {
        Frame::new()
            .fill(Color32::from_rgba_premultiplied(26, 30, 48, 230))
            .corner_radius(CornerRadius::same(18))
            .stroke(Stroke::new(1.0_f32, rgba(108, 99, 255, 50)))
            .shadow(Shadow {
                offset: [0, 0],
                blur: 40,
                spread: 4,
                color: rgba(108, 99, 255, 35),
            })
            .inner_margin(Margin::same(20))
    }

    /// Thin glass toolbar bar.
    pub fn toolbar_frame() -> Frame {
        Frame::new()
            .fill(Color32::from_rgba_premultiplied(20, 24, 38, 200))
            .corner_radius(CornerRadius::ZERO)
            .stroke(Stroke::new(1.0_f32, rgba(255, 255, 255, 18)))
            .shadow(Shadow {
                offset: [0, 2],
                blur: 6,
                spread: 0,
                color: rgba(0, 0, 0, 30),
            })
            .inner_margin(Margin::symmetric(14, 6))
    }

    /// Soft depth shadow for morphic surfaces.
    pub fn morphic_shadow() -> Shadow {
        Shadow {
            offset: [0, 6],
            blur: 20,
            spread: 0,
            color: rgba(0, 0, 0, 50),
        }
    }

    /// Blend two colors vertically (simulates gradient top-to-bottom).
    pub fn gradient_bg(top: Color32, bottom: Color32, t: f32) -> Color32 {
        let t = t.clamp(0.0, 1.0);
        Color32::from_rgba_premultiplied(
            lerp_u8(top.r(), bottom.r(), t),
            lerp_u8(top.g(), bottom.g(), t),
            lerp_u8(top.b(), bottom.b(), t),
            lerp_u8(top.a(), bottom.a(), t),
        )
    }

    /// Color for a health status indicator.
    pub fn status_color(&self, healthy: bool) -> Color32 {
        if healthy {
            self.status_healthy
        } else {
            self.status_error
        }
    }

    /// Color for a topology node by locality code.
    pub fn node_color(&self, locality: &str) -> Color32 {
        match locality {
            "GPU" | "gpu" | "G" => self.node_gpu,
            "CPU" | "cpu" | "C" => self.node_cpu,
            "IO" | "io" | "I" => self.node_io,
            "NET" | "net" | "N" => self.node_network,
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
            self.status_healthy
        } else if ms < 2.0 {
            self.status_warning
        } else {
            self.status_error
        }
    }

    /// Color for a lease state string.
    pub fn lease_state_color(&self, state: &str) -> Color32 {
        if state.contains("Active") {
            self.status_healthy
        } else if state.contains("Pending") || state.contains("Provisioning") {
            self.status_warning
        } else if state.contains("Failed")
            || state.contains("Revoked")
            || state.contains("Expired")
        {
            self.status_error
        } else {
            self.text_muted
        }
    }

    /// Sidebar frame with glass background.
    pub fn sidebar_frame() -> Frame {
        Frame::new()
            .fill(Color32::from_rgba_premultiplied(18, 22, 34, 220))
            .inner_margin(Margin::symmetric(12, 16))
            .stroke(Stroke::new(1.0_f32, rgba(255, 255, 255, 18)))
    }

    /// Status bar frame at bottom.
    pub fn status_bar_frame() -> Frame {
        Frame::new()
            .fill(Color32::from_rgba_premultiplied(20, 24, 38, 200))
            .inner_margin(Margin::symmetric(12, 6))
            .stroke(Stroke::new(1.0_f32, rgba(255, 255, 255, 15)))
    }
}

impl Default for LiquidTheme {
    fn default() -> Self {
        Self::dark()
    }
}

// ---------------------------------------------------------------------------
// FabricTheme -- premium wrapper with extended accent / separator tokens
// ---------------------------------------------------------------------------

/// Premium wrapper that extends [`LiquidTheme`] with extra accent tokens
/// used by the morphic premium widgets (text shadows, gradient accents,
/// morphic highlights, separator colour).
pub struct FabricTheme {
    /// Base liquid glass palette.
    pub base: LiquidTheme,
    /// Colour for subtle text-shadow / glow behind headings.
    pub text_shadow_color: Color32,
    /// Two-colour gradient for gradient background fills.
    pub gradient_accent: [Color32; 2],
    /// Hover / active morphic highlight tint.
    pub morphic_highlight: Color32,
    /// Section divider line colour.
    pub separator_color: Color32,
}

impl FabricTheme {
    /// Build a dark premium theme.
    pub fn dark() -> Self {
        let base = LiquidTheme::dark();
        Self {
            text_shadow_color: rgba(108, 99, 255, 40),
            gradient_accent: [
                Color32::from_rgb(80, 60, 220),
                Color32::from_rgb(0, 180, 255),
            ],
            morphic_highlight: rgba(108, 99, 255, 35),
            separator_color: rgba(255, 255, 255, 22),
            base,
        }
    }

    /// Build a light premium theme.
    pub fn light() -> Self {
        let base = LiquidTheme::light();
        Self {
            text_shadow_color: rgba(88, 80, 236, 20),
            gradient_accent: [
                Color32::from_rgb(60, 50, 200),
                Color32::from_rgb(0, 150, 200),
            ],
            morphic_highlight: rgba(88, 80, 236, 20),
            separator_color: rgba(0, 0, 0, 18),
            base,
        }
    }

    /// Shorthand accessor to the inner [`LiquidTheme`].
    pub fn liquid(&self) -> &LiquidTheme {
        &self.base
    }
}

/// Helper: create `Color32` from RGBA components (0-255).
const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Color32 {
    Color32::from_rgba_premultiplied(r, g, b, a)
}

/// Lerp between two `u8` values.
const fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    let a = a as f32;
    let b = b as f32;
    (a + (b - a) * t) as u8
}
