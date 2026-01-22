//! Button component with variants and sizes.

use gpui::{
    div, prelude::*, px, App, ClickEvent, CursorStyle, ElementId, FocusHandle, Hsla, IntoElement,
    Pixels, RenderOnce, SharedString, Window,
};

use crate::icon::{Icon, IconName, IconSize};
use crate::spinner::{Spinner, SpinnerSize};
use crate::TuskTheme;

/// Type alias for button click handler callback.
pub type ClickHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

/// Button variant styles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonVariant {
    /// Primary action button with accent background.
    #[default]
    Primary,
    /// Secondary button with subtle background.
    Secondary,
    /// Ghost button with transparent background.
    Ghost,
    /// Danger button for destructive actions.
    Danger,
}

/// Button size variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonSize {
    /// Small: 28px height
    Small,
    /// Medium: 32px height (default)
    #[default]
    Medium,
    /// Large: 40px height
    Large,
}

impl ButtonSize {
    /// Get the button height in pixels.
    pub fn height(&self) -> Pixels {
        match self {
            Self::Small => px(28.0),
            Self::Medium => px(32.0),
            Self::Large => px(40.0),
        }
    }

    /// Get the horizontal padding.
    pub fn padding_x(&self) -> Pixels {
        match self {
            Self::Small => px(8.0),
            Self::Medium => px(12.0),
            Self::Large => px(16.0),
        }
    }

    /// Get the text size.
    pub fn text_size(&self) -> Pixels {
        match self {
            Self::Small => px(12.0),
            Self::Medium => px(14.0),
            Self::Large => px(16.0),
        }
    }

    /// Get the corresponding icon size.
    pub fn icon_size(&self) -> IconSize {
        match self {
            Self::Small => IconSize::Small,
            Self::Medium => IconSize::Medium,
            Self::Large => IconSize::Large,
        }
    }

    /// Get the corresponding spinner size.
    pub fn spinner_size(&self) -> SpinnerSize {
        match self {
            Self::Small => SpinnerSize::Small,
            Self::Medium => SpinnerSize::Small,
            Self::Large => SpinnerSize::Medium,
        }
    }
}

/// Icon position relative to label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IconPosition {
    /// Icon on the left side of the label.
    #[default]
    Left,
    /// Icon on the right side of the label.
    Right,
}

/// A button component with customizable variant, size, and icon.
///
/// Buttons support optional focus handling for keyboard navigation. When a focus handle
/// is provided via `track_focus()`, the button will display a visible focus ring when
/// focused, meeting WCAG 2.1 AA accessibility requirements.
#[derive(IntoElement)]
pub struct Button {
    id: ElementId,
    label: Option<SharedString>,
    icon: Option<IconName>,
    icon_position: IconPosition,
    variant: ButtonVariant,
    size: ButtonSize,
    disabled: bool,
    loading: bool,
    focus_handle: Option<FocusHandle>,
    on_click: Option<ClickHandler>,
}

impl Button {
    /// Create a new button with a unique ID.
    pub fn new() -> Self {
        Self {
            id: ElementId::Name("button".into()),
            label: None,
            icon: None,
            icon_position: IconPosition::default(),
            variant: ButtonVariant::default(),
            size: ButtonSize::default(),
            disabled: false,
            loading: false,
            focus_handle: None,
            on_click: None,
        }
    }

    /// Set the button ID.
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }

    /// Set the button label.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set the button icon.
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Set the icon position.
    pub fn icon_position(mut self, position: IconPosition) -> Self {
        self.icon_position = position;
        self
    }

    /// Set the button variant.
    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set the button size.
    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    /// Set the disabled state.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set the loading state.
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Track focus for keyboard navigation.
    ///
    /// When a focus handle is provided, the button will:
    /// - Be included in tab order
    /// - Show a visible focus ring when focused (WCAG 2.1 AA compliant)
    pub fn track_focus(mut self, focus_handle: &FocusHandle) -> Self {
        self.focus_handle = Some(focus_handle.clone());
        self
    }

    /// Set the click handler.
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    /// Get colors for the current variant.
    fn colors(&self, theme: &TuskTheme) -> (Hsla, Hsla, Hsla, Hsla) {
        let colors = &theme.colors;
        match self.variant {
            ButtonVariant::Primary => (
                colors.accent,       // background
                colors.accent_hover, // hover background
                colors.on_accent,    // text
                colors.on_accent,    // text (no change on hover)
            ),
            ButtonVariant::Secondary => (
                colors.element_background, // background
                colors.element_hover,      // hover background
                colors.text,               // text
                colors.text,               // text
            ),
            ButtonVariant::Ghost => (
                gpui::transparent_black(), // background
                colors.ghost_element_hover, // hover background
                colors.text,                // text
                colors.text,                // text
            ),
            ButtonVariant::Danger => (
                colors.error,
                colors.error.opacity(0.8),
                colors.on_accent,
                colors.on_accent,
            ),
        }
    }
}

impl Default for Button {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for Button {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.global::<TuskTheme>();
        let (bg, hover_bg, text_color, _hover_text) = self.colors(theme);
        let focus_ring_color = theme.colors.accent;

        let height = self.size.height();
        let padding_x = self.size.padding_x();
        let text_size = self.size.text_size();
        let icon_size = self.size.icon_size();
        let spinner_size = self.size.spinner_size();

        let is_interactive = !self.disabled && !self.loading;
        let opacity = if self.disabled { 0.5 } else { 1.0 };

        let mut button = div()
            .id(self.id)
            .h(height)
            .px(padding_x)
            .flex()
            .items_center()
            .justify_center()
            .gap(px(6.0))
            .rounded(px(4.0))
            .bg(bg)
            .text_color(text_color)
            .text_size(text_size)
            .opacity(opacity)
            // Border for focus ring - transparent by default
            .border_2()
            .border_color(gpui::transparent_black());

        // Add focus tracking and focus ring if focus handle provided
        if let Some(focus_handle) = self.focus_handle {
            button = button
                .track_focus(&focus_handle)
                // WCAG 2.1 AA compliant focus ring: 2px accent border with high contrast
                .focus(|style| style.border_color(focus_ring_color));
        }

        // Add hover effect if interactive
        if is_interactive {
            button = button
                .cursor(CursorStyle::PointingHand)
                .hover(|style| style.bg(hover_bg));
        }

        // Add click handler if interactive
        if is_interactive {
            if let Some(handler) = self.on_click {
                button = button.on_click(move |event, window, cx| handler(event, window, cx));
            }
        }

        // Render content
        if self.loading {
            // Show spinner when loading
            button = button.child(Spinner::new().size(spinner_size));
        } else {
            // Show icon on left if configured
            if let Some(icon_name) = self.icon {
                if self.icon_position == IconPosition::Left {
                    button = button.child(Icon::new(icon_name).size(icon_size).color(text_color));
                }
            }

            // Show label
            if let Some(label) = &self.label {
                button = button.child(label.clone());
            }

            // Show icon on right if configured
            if let Some(icon_name) = self.icon {
                if self.icon_position == IconPosition::Right {
                    button = button.child(Icon::new(icon_name).size(icon_size).color(text_color));
                }
            }
        }

        button
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_button_sizes() {
        assert_eq!(ButtonSize::Small.height(), px(28.0));
        assert_eq!(ButtonSize::Medium.height(), px(32.0));
        assert_eq!(ButtonSize::Large.height(), px(40.0));
    }

    #[test]
    fn test_button_construction() {
        let button = Button::new()
            .label("Click me")
            .icon(IconName::Plus)
            .variant(ButtonVariant::Primary)
            .size(ButtonSize::Medium);

        assert!(button.label.is_some());
        assert!(button.icon.is_some());
        assert_eq!(button.variant, ButtonVariant::Primary);
        assert_eq!(button.size, ButtonSize::Medium);
    }
}
