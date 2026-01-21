//! Icon system for Tusk application.
//!
//! Provides the IconName enum with all available icons and an Icon component
//! for rendering them at various sizes.

use gpui::{
    div, prelude::*, px, App, Hsla, IntoElement, Pixels, RenderOnce, SharedString, Window,
};

use crate::TuskTheme;

/// All available icons in the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IconName {
    // Navigation
    /// Right chevron for expandable items
    ChevronRight,
    /// Down chevron for expanded items
    ChevronDown,
    /// Left chevron
    ChevronLeft,
    /// Up chevron
    ChevronUp,

    // Actions
    /// Plus sign for adding items
    Plus,
    /// Close/X button
    Close,
    /// Search magnifying glass
    Search,
    /// Refresh/reload arrows
    Refresh,
    /// Play triangle
    Play,
    /// Stop square
    Stop,
    /// Save/floppy disk
    Save,
    /// Copy
    Copy,
    /// Paste
    Paste,
    /// Edit pencil
    Edit,
    /// Trash/delete
    Trash,
    /// Undo arrow
    Undo,
    /// Redo arrow
    Redo,

    // Database objects
    /// Database cylinder
    Database,
    /// Table grid
    Table,
    /// Column/field
    Column,
    /// Primary key
    Key,
    /// Index
    Index,
    /// View
    View,
    /// Function
    Function,
    /// Schema/namespace
    Schema,
    /// Folder
    Folder,
    /// File/document
    File,
    /// Code brackets
    Code,
    /// Trigger
    Trigger,
    /// Sequence
    Sequence,
    /// Constraint
    Constraint,

    // Connection status
    /// Connected indicator
    Connected,
    /// Disconnected indicator
    Disconnected,
    /// Connection loading
    Connecting,

    // Status indicators
    /// Check mark for success
    Check,
    /// Warning triangle
    Warning,
    /// Error circle
    Error,
    /// Info circle
    Info,

    // UI elements
    /// Hamburger menu
    Menu,
    /// Settings gear
    Settings,
    /// Vertical dots (more options)
    VerticalDots,
    /// Horizontal dots
    HorizontalDots,
    /// Pin
    Pin,
    /// Unpin
    Unpin,
    /// Maximize window
    Maximize,
    /// Minimize window
    Minimize,
    /// Split horizontal
    SplitHorizontal,
    /// Split vertical
    SplitVertical,

    // Query/execution
    /// Execute query
    Execute,
    /// Cancel query
    Cancel,
    /// History
    History,
    /// Bookmark
    Bookmark,
    /// Export
    Export,
    /// Import
    Import,

    // Application
    /// Application icon
    App,
}

impl IconName {
    /// Get the icon name as a string for loading from assets.
    pub fn name(&self) -> &'static str {
        match self {
            // Navigation
            Self::ChevronRight => "chevron_right",
            Self::ChevronDown => "chevron_down",
            Self::ChevronLeft => "chevron_left",
            Self::ChevronUp => "chevron_up",

            // Actions
            Self::Plus => "plus",
            Self::Close => "close",
            Self::Search => "search",
            Self::Refresh => "refresh",
            Self::Play => "play",
            Self::Stop => "stop",
            Self::Save => "save",
            Self::Copy => "copy",
            Self::Paste => "paste",
            Self::Edit => "edit",
            Self::Trash => "trash",
            Self::Undo => "undo",
            Self::Redo => "redo",

            // Database objects
            Self::Database => "database",
            Self::Table => "table",
            Self::Column => "column",
            Self::Key => "key",
            Self::Index => "index",
            Self::View => "view",
            Self::Function => "function",
            Self::Schema => "schema",
            Self::Folder => "folder",
            Self::File => "file",
            Self::Code => "code",
            Self::Trigger => "trigger",
            Self::Sequence => "sequence",
            Self::Constraint => "constraint",

            // Connection status
            Self::Connected => "connected",
            Self::Disconnected => "disconnected",
            Self::Connecting => "connecting",

            // Status indicators
            Self::Check => "check",
            Self::Warning => "warning",
            Self::Error => "error",
            Self::Info => "info",

            // UI elements
            Self::Menu => "menu",
            Self::Settings => "settings",
            Self::VerticalDots => "vertical_dots",
            Self::HorizontalDots => "horizontal_dots",
            Self::Pin => "pin",
            Self::Unpin => "unpin",
            Self::Maximize => "maximize",
            Self::Minimize => "minimize",
            Self::SplitHorizontal => "split_horizontal",
            Self::SplitVertical => "split_vertical",

            // Query/execution
            Self::Execute => "execute",
            Self::Cancel => "cancel",
            Self::History => "history",
            Self::Bookmark => "bookmark",
            Self::Export => "export",
            Self::Import => "import",

            // Application
            Self::App => "tusk",
        }
    }

    /// Get a unicode character representation for rendering without assets.
    /// Used as fallback when SVG icons are not available.
    pub fn as_char(&self) -> &'static str {
        match self {
            // Navigation
            Self::ChevronRight => "›",
            Self::ChevronDown => "⌄",
            Self::ChevronLeft => "‹",
            Self::ChevronUp => "⌃",

            // Actions
            Self::Plus => "+",
            Self::Close => "×",
            Self::Search => "⌕",
            Self::Refresh => "↻",
            Self::Play => "▶",
            Self::Stop => "■",
            Self::Save => "💾",
            Self::Copy => "⧉",
            Self::Paste => "📋",
            Self::Edit => "✎",
            Self::Trash => "🗑",
            Self::Undo => "↶",
            Self::Redo => "↷",

            // Database objects
            Self::Database => "⛁",
            Self::Table => "⊞",
            Self::Column => "▭",
            Self::Key => "🔑",
            Self::Index => "⊟",
            Self::View => "👁",
            Self::Function => "ƒ",
            Self::Schema => "◯",
            Self::Folder => "📁",
            Self::File => "📄",
            Self::Code => "⟨⟩",
            Self::Trigger => "⚡",
            Self::Sequence => "#",
            Self::Constraint => "⧫",

            // Connection status
            Self::Connected => "●",
            Self::Disconnected => "○",
            Self::Connecting => "◐",

            // Status indicators
            Self::Check => "✓",
            Self::Warning => "⚠",
            Self::Error => "✗",
            Self::Info => "ℹ",

            // UI elements
            Self::Menu => "☰",
            Self::Settings => "⚙",
            Self::VerticalDots => "⋮",
            Self::HorizontalDots => "⋯",
            Self::Pin => "📌",
            Self::Unpin => "📌",
            Self::Maximize => "⤢",
            Self::Minimize => "⤡",
            Self::SplitHorizontal => "⫿",
            Self::SplitVertical => "⫾",

            // Query/execution
            Self::Execute => "▶",
            Self::Cancel => "⏹",
            Self::History => "⟲",
            Self::Bookmark => "★",
            Self::Export => "↗",
            Self::Import => "↙",

            // Application
            Self::App => "🐘",
        }
    }
}

/// Size variants for icons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IconSize {
    /// Extra small: 12px
    XSmall,
    /// Small: 14px
    Small,
    /// Medium: 16px (default)
    #[default]
    Medium,
    /// Large: 20px
    Large,
    /// Extra large: 24px
    XLarge,
}

impl IconSize {
    /// Get the size in pixels.
    pub fn pixels(&self) -> Pixels {
        match self {
            Self::XSmall => px(12.0),
            Self::Small => px(14.0),
            Self::Medium => px(16.0),
            Self::Large => px(20.0),
            Self::XLarge => px(24.0),
        }
    }
}

/// Icon component for rendering icons.
#[derive(IntoElement)]
pub struct Icon {
    name: IconName,
    size: IconSize,
    color: Option<Hsla>,
}

impl Icon {
    /// Create a new icon with the given name.
    pub fn new(name: IconName) -> Self {
        Self {
            name,
            size: IconSize::default(),
            color: None,
        }
    }

    /// Set the icon size.
    pub fn size(mut self, size: IconSize) -> Self {
        self.size = size;
        self
    }

    /// Set a custom color for the icon.
    pub fn color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }
}

impl RenderOnce for Icon {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.global::<TuskTheme>();
        let size = self.size.pixels();
        let color = self.color.unwrap_or(theme.colors.text);

        // For now, render the unicode character representation
        // TODO: Load SVG icons from assets when available
        div()
            .size(size)
            .flex()
            .items_center()
            .justify_center()
            .text_color(color)
            .text_size(size)
            .line_height(size)
            .child(SharedString::from(self.name.as_char()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_icon_sizes() {
        assert_eq!(IconSize::XSmall.pixels(), px(12.0));
        assert_eq!(IconSize::Small.pixels(), px(14.0));
        assert_eq!(IconSize::Medium.pixels(), px(16.0));
        assert_eq!(IconSize::Large.pixels(), px(20.0));
        assert_eq!(IconSize::XLarge.pixels(), px(24.0));
    }

    #[test]
    fn test_icon_names() {
        assert_eq!(IconName::Database.name(), "database");
        assert_eq!(IconName::Table.name(), "table");
        assert_eq!(IconName::ChevronRight.name(), "chevron_right");
    }

    #[test]
    fn test_icon_chars() {
        assert_eq!(IconName::ChevronRight.as_char(), "›");
        assert_eq!(IconName::Database.as_char(), "⛁");
        assert_eq!(IconName::Check.as_char(), "✓");
    }
}
