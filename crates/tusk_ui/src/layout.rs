//! Layout utilities for consistent spacing and alignment.

use gpui::{px, Pixels};

/// Standard spacing constants for UI layout.
pub mod spacing {
    use gpui::{px, Pixels};

    /// Extra small spacing: 4px
    pub const XS: Pixels = px(4.0);
    /// Small spacing: 8px
    pub const SM: Pixels = px(8.0);
    /// Medium spacing: 12px
    pub const MD: Pixels = px(12.0);
    /// Large spacing: 16px
    pub const LG: Pixels = px(16.0);
    /// Extra large spacing: 24px
    pub const XL: Pixels = px(24.0);
    /// Double extra large spacing: 32px
    pub const XXL: Pixels = px(32.0);
}

/// Standard border radius constants.
pub mod radius {
    use gpui::{px, Pixels};

    /// Small radius: 2px
    pub const SM: Pixels = px(2.0);
    /// Medium radius: 4px
    pub const MD: Pixels = px(4.0);
    /// Large radius: 6px
    pub const LG: Pixels = px(6.0);
    /// Extra large radius: 8px
    pub const XL: Pixels = px(8.0);
    /// Full radius (for circular elements)
    pub const FULL: Pixels = px(9999.0);
}

/// Standard sizing constants.
pub mod sizes {
    use gpui::{px, Pixels};

    /// Minimum dock width/height
    pub const DOCK_MIN: Pixels = px(120.0);
    /// Maximum side dock width
    pub const DOCK_MAX_SIDE: Pixels = px(600.0);
    /// Minimum bottom dock height
    pub const DOCK_MIN_BOTTOM: Pixels = px(100.0);
    /// Maximum bottom dock height (as pixels, actual max is 50vh)
    pub const DOCK_MAX_BOTTOM: Pixels = px(400.0);

    /// Tab bar height
    pub const TAB_BAR_HEIGHT: Pixels = px(36.0);
    /// Status bar height
    pub const STATUS_BAR_HEIGHT: Pixels = px(28.0);
    /// Default side dock width
    pub const DEFAULT_DOCK_WIDTH: Pixels = px(240.0);
    /// Default bottom dock height
    pub const DEFAULT_DOCK_HEIGHT: Pixels = px(200.0);

    /// Resizer handle size
    pub const RESIZER_SIZE: Pixels = px(6.0);

    /// Tree item height
    pub const TREE_ITEM_HEIGHT: Pixels = px(28.0);
    /// Tree indent per level
    pub const TREE_INDENT: Pixels = px(16.0);
}

/// Helper to convert pixels to f32 for calculations.
pub fn to_f32(pixels: Pixels) -> f32 {
    pixels.into()
}

/// Helper to create pixels from f32.
pub fn from_f32(value: f32) -> Pixels {
    px(value)
}

/// Clamp a pixel value between min and max.
pub fn clamp_pixels(value: Pixels, min: Pixels, max: Pixels) -> Pixels {
    let val: f32 = value.into();
    let min_val: f32 = min.into();
    let max_val: f32 = max.into();
    if val < min_val {
        min
    } else if val > max_val {
        max
    } else {
        value
    }
}
