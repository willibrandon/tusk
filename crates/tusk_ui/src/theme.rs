//! Theme definitions for Tusk application.

use gpui::{hsla, Global, Hsla, WindowAppearance};

/// Color palette for UI rendering.
///
/// All colors use GPUI's `Hsla` type (Hue, Saturation, Lightness, Alpha).
#[derive(Debug, Clone)]
pub struct ThemeColors {
    /// Window background color.
    pub background: Hsla,
    /// Panel/card background.
    pub surface: Hsla,
    /// Elevated panel background.
    pub elevated_surface: Hsla,
    /// Primary text color.
    pub text: Hsla,
    /// Secondary/dimmed text.
    pub text_muted: Hsla,
    /// Accent/link text.
    pub text_accent: Hsla,
    /// Element border color.
    pub border: Hsla,
    /// Subtle border variant.
    pub border_variant: Hsla,
    /// Primary accent color.
    pub accent: Hsla,
    /// Accent hover state.
    pub accent_hover: Hsla,
    /// Success indicator.
    pub status_success: Hsla,
    /// Warning indicator.
    pub status_warning: Hsla,
    /// Error indicator.
    pub status_error: Hsla,
    /// Info indicator.
    pub status_info: Hsla,
}

impl ThemeColors {
    /// Create the dark theme color palette.
    ///
    /// Based on Catppuccin Mocha palette.
    pub fn dark() -> Self {
        Self {
            // #1e1e2e - Mocha Base
            background: hsla(240.0 / 360.0, 0.21, 0.15, 1.0),
            // #313244 - Mocha Surface0
            surface: hsla(237.0 / 360.0, 0.16, 0.23, 1.0),
            // #45475a - Mocha Surface1
            elevated_surface: hsla(233.0 / 360.0, 0.13, 0.31, 1.0),
            // #cdd6f4 - Mocha Text
            text: hsla(226.0 / 360.0, 0.64, 0.88, 1.0),
            // #a6adc8 - Mocha Subtext0
            text_muted: hsla(228.0 / 360.0, 0.24, 0.72, 1.0),
            // #89b4fa - Mocha Blue
            text_accent: hsla(217.0 / 360.0, 0.92, 0.76, 1.0),
            // #45475a - Mocha Surface1
            border: hsla(233.0 / 360.0, 0.13, 0.31, 1.0),
            // #313244 - Mocha Surface0
            border_variant: hsla(237.0 / 360.0, 0.16, 0.23, 1.0),
            // #89b4fa - Mocha Blue
            accent: hsla(217.0 / 360.0, 0.92, 0.76, 1.0),
            // #b4befe - Mocha Lavender (slightly lighter for hover)
            accent_hover: hsla(232.0 / 360.0, 0.97, 0.85, 1.0),
            // #a6e3a1 - Mocha Green
            status_success: hsla(115.0 / 360.0, 0.54, 0.76, 1.0),
            // #f9e2af - Mocha Yellow
            status_warning: hsla(41.0 / 360.0, 0.86, 0.83, 1.0),
            // #f38ba8 - Mocha Red
            status_error: hsla(343.0 / 360.0, 0.81, 0.75, 1.0),
            // #89dceb - Mocha Sky
            status_info: hsla(189.0 / 360.0, 0.71, 0.73, 1.0),
        }
    }

    /// Create the light theme color palette.
    ///
    /// Based on Catppuccin Latte palette.
    pub fn light() -> Self {
        Self {
            // #eff1f5 - Latte Base
            background: hsla(220.0 / 360.0, 0.23, 0.95, 1.0),
            // #e6e9ef - Latte Surface0
            surface: hsla(220.0 / 360.0, 0.21, 0.92, 1.0),
            // #dce0e8 - Latte Surface1
            elevated_surface: hsla(220.0 / 360.0, 0.22, 0.90, 1.0),
            // #4c4f69 - Latte Text
            text: hsla(234.0 / 360.0, 0.16, 0.35, 1.0),
            // #6c6f85 - Latte Subtext0
            text_muted: hsla(233.0 / 360.0, 0.10, 0.47, 1.0),
            // #1e66f5 - Latte Blue
            text_accent: hsla(220.0 / 360.0, 0.91, 0.54, 1.0),
            // #dce0e8 - Latte Surface1
            border: hsla(220.0 / 360.0, 0.22, 0.90, 1.0),
            // #e6e9ef - Latte Surface0
            border_variant: hsla(220.0 / 360.0, 0.21, 0.92, 1.0),
            // #1e66f5 - Latte Blue
            accent: hsla(220.0 / 360.0, 0.91, 0.54, 1.0),
            // #7287fd - Latte Lavender
            accent_hover: hsla(231.0 / 360.0, 0.97, 0.72, 1.0),
            // #40a02b - Latte Green
            status_success: hsla(109.0 / 360.0, 0.58, 0.40, 1.0),
            // #df8e1d - Latte Yellow
            status_warning: hsla(35.0 / 360.0, 0.77, 0.49, 1.0),
            // #d20f39 - Latte Red
            status_error: hsla(347.0 / 360.0, 0.87, 0.44, 1.0),
            // #04a5e5 - Latte Sky
            status_info: hsla(197.0 / 360.0, 0.97, 0.46, 1.0),
        }
    }
}

/// Theme configuration for application styling.
#[derive(Debug, Clone)]
pub struct TuskTheme {
    /// Theme identifier.
    pub name: String,
    /// GPUI appearance (Light/Dark).
    pub appearance: WindowAppearance,
    /// Color palette.
    pub colors: ThemeColors,
}

impl TuskTheme {
    /// Create a new dark theme.
    pub fn dark() -> Self {
        Self {
            name: "dark".to_string(),
            appearance: WindowAppearance::Dark,
            colors: ThemeColors::dark(),
        }
    }

    /// Create a new light theme.
    pub fn light() -> Self {
        Self {
            name: "light".to_string(),
            appearance: WindowAppearance::Light,
            colors: ThemeColors::light(),
        }
    }
}

impl Default for TuskTheme {
    fn default() -> Self {
        Self::dark()
    }
}

impl Global for TuskTheme {}
