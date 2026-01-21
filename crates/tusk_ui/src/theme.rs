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

    // Tab bar colors
    /// Tab bar background.
    pub tab_bar_background: Hsla,
    /// Active tab background.
    pub tab_active_background: Hsla,
    /// Inactive tab background.
    pub tab_inactive_background: Hsla,
    /// Tab hover background.
    pub tab_hover_background: Hsla,

    // Panel and dock colors
    /// Panel background.
    pub panel_background: Hsla,
    /// Editor background.
    pub editor_background: Hsla,
    /// Status bar background.
    pub status_bar_background: Hsla,

    // List selection colors
    /// Active selection in lists.
    pub list_active_selection_background: Hsla,
    /// Hover state in lists.
    pub list_hover_background: Hsla,

    // Button and element colors
    /// Element background (buttons, etc).
    pub element_background: Hsla,
    /// Element hover state.
    pub element_hover: Hsla,
    /// Ghost element hover state.
    pub ghost_element_hover: Hsla,
    /// Text on accent backgrounds.
    pub on_accent: Hsla,

    // Input colors
    /// Input field background.
    pub input_background: Hsla,

    // Elevated surface (modals, dropdowns)
    /// Elevated surface background.
    pub elevated_surface_background: Hsla,

    // Semantic colors (aliases for consistency)
    /// Success color.
    pub success: Hsla,
    /// Warning color.
    pub warning: Hsla,
    /// Error color.
    pub error: Hsla,
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

            // Tab bar colors - slightly darker than surface
            // #181825 - Mocha Mantle
            tab_bar_background: hsla(240.0 / 360.0, 0.21, 0.12, 1.0),
            // #313244 - Mocha Surface0 (active tab matches surface)
            tab_active_background: hsla(237.0 / 360.0, 0.16, 0.23, 1.0),
            // Transparent (inactive tabs)
            tab_inactive_background: hsla(0.0, 0.0, 0.0, 0.0),
            // #45475a with opacity - Surface1 at 50%
            tab_hover_background: hsla(233.0 / 360.0, 0.13, 0.31, 0.5),

            // Panel and dock colors
            // #181825 - Mocha Mantle
            panel_background: hsla(240.0 / 360.0, 0.21, 0.12, 1.0),
            // #1e1e2e - Mocha Base
            editor_background: hsla(240.0 / 360.0, 0.21, 0.15, 1.0),
            // #11111b - Mocha Crust
            status_bar_background: hsla(240.0 / 360.0, 0.23, 0.09, 1.0),

            // List selection colors
            // #89b4fa at 20% - Blue selection
            list_active_selection_background: hsla(217.0 / 360.0, 0.92, 0.76, 0.2),
            // #45475a at 50% - Surface1 hover
            list_hover_background: hsla(233.0 / 360.0, 0.13, 0.31, 0.5),

            // Button and element colors
            // #45475a - Mocha Surface1
            element_background: hsla(233.0 / 360.0, 0.13, 0.31, 1.0),
            // #585b70 - Mocha Surface2
            element_hover: hsla(232.0 / 360.0, 0.12, 0.39, 1.0),
            // #45475a at 50%
            ghost_element_hover: hsla(233.0 / 360.0, 0.13, 0.31, 0.5),
            // #11111b - Dark text on accent backgrounds
            on_accent: hsla(240.0 / 360.0, 0.23, 0.09, 1.0),

            // Input colors
            // #181825 - Mocha Mantle
            input_background: hsla(240.0 / 360.0, 0.21, 0.12, 1.0),

            // Elevated surface (modals, dropdowns)
            // #45475a - Mocha Surface1
            elevated_surface_background: hsla(233.0 / 360.0, 0.13, 0.31, 1.0),

            // Semantic colors (same as status colors)
            success: hsla(115.0 / 360.0, 0.54, 0.76, 1.0),
            warning: hsla(41.0 / 360.0, 0.86, 0.83, 1.0),
            error: hsla(343.0 / 360.0, 0.81, 0.75, 1.0),
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

            // Tab bar colors
            // #ccd0da - Latte Mantle
            tab_bar_background: hsla(220.0 / 360.0, 0.21, 0.82, 1.0),
            // #eff1f5 - Latte Base (active tab matches base)
            tab_active_background: hsla(220.0 / 360.0, 0.23, 0.95, 1.0),
            // Transparent (inactive tabs)
            tab_inactive_background: hsla(0.0, 0.0, 0.0, 0.0),
            // #bcc0cc at 50% - Surface2 hover
            tab_hover_background: hsla(220.0 / 360.0, 0.12, 0.77, 0.5),

            // Panel and dock colors
            // #e6e9ef - Latte Surface0
            panel_background: hsla(220.0 / 360.0, 0.21, 0.92, 1.0),
            // #eff1f5 - Latte Base
            editor_background: hsla(220.0 / 360.0, 0.23, 0.95, 1.0),
            // #ccd0da - Latte Mantle
            status_bar_background: hsla(220.0 / 360.0, 0.21, 0.82, 1.0),

            // List selection colors
            // #1e66f5 at 15% - Blue selection
            list_active_selection_background: hsla(220.0 / 360.0, 0.91, 0.54, 0.15),
            // #bcc0cc at 50% - Surface2 hover
            list_hover_background: hsla(220.0 / 360.0, 0.12, 0.77, 0.5),

            // Button and element colors
            // #dce0e8 - Latte Surface1
            element_background: hsla(220.0 / 360.0, 0.22, 0.90, 1.0),
            // #bcc0cc - Latte Surface2
            element_hover: hsla(220.0 / 360.0, 0.12, 0.77, 1.0),
            // #dce0e8 at 50%
            ghost_element_hover: hsla(220.0 / 360.0, 0.22, 0.90, 0.5),
            // #eff1f5 - Light text on accent backgrounds
            on_accent: hsla(220.0 / 360.0, 0.23, 0.95, 1.0),

            // Input colors
            // #ccd0da - Latte Mantle
            input_background: hsla(220.0 / 360.0, 0.21, 0.82, 1.0),

            // Elevated surface (modals, dropdowns)
            // #dce0e8 - Latte Surface1
            elevated_surface_background: hsla(220.0 / 360.0, 0.22, 0.90, 1.0),

            // Semantic colors (same as status colors)
            success: hsla(109.0 / 360.0, 0.58, 0.40, 1.0),
            warning: hsla(35.0 / 360.0, 0.77, 0.49, 1.0),
            error: hsla(347.0 / 360.0, 0.87, 0.44, 1.0),
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
