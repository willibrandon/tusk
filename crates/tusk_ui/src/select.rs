//! Select/dropdown component with keyboard navigation and search filtering.

use gpui::{
    anchored, deferred, div, prelude::*, px, App, Context, Corner, CursorStyle, ElementId,
    EventEmitter, FocusHandle, Focusable, InteractiveElement, IntoElement, ParentElement, Render,
    SharedString, StatefulInteractiveElement, Styled, Subscription, Window,
};

use crate::icon::{Icon, IconName, IconSize};
use crate::key_bindings::select::{Close, Confirm, Open, SelectNextOption, SelectPreviousOption};
use crate::TuskTheme;

/// Events emitted by Select.
#[derive(Clone, Debug)]
pub enum SelectEvent<T: Clone> {
    /// The selected value changed.
    Changed(T),
    /// The dropdown was opened.
    Opened,
    /// The dropdown was closed.
    Closed,
}

/// An option in the select dropdown.
#[derive(Clone, Debug)]
pub struct SelectOption<T: Clone> {
    /// The value of this option.
    pub value: T,
    /// The display label for this option.
    pub label: SharedString,
    /// Whether this option is disabled.
    pub disabled: bool,
}

impl<T: Clone> SelectOption<T> {
    /// Create a new select option.
    pub fn new(value: T, label: impl Into<SharedString>) -> Self {
        Self {
            value,
            label: label.into(),
            disabled: false,
        }
    }

    /// Mark this option as disabled.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// A dropdown select component with keyboard navigation.
pub struct Select<T: Clone + PartialEq + 'static> {
    id: ElementId,
    options: Vec<SelectOption<T>>,
    selected: Option<T>,
    placeholder: SharedString,
    open: bool,
    highlighted_index: usize,
    focus_handle: FocusHandle,
    popover_focus_handle: FocusHandle,
    disabled: bool,
    #[allow(dead_code)]
    focus_subscription: Option<Subscription>,
    #[allow(dead_code)]
    blur_subscription: Option<Subscription>,
}

impl<T: Clone + PartialEq + 'static> Select<T> {
    /// Create a new select component with the given options.
    pub fn new(
        id: impl Into<ElementId>,
        options: Vec<SelectOption<T>>,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            id: id.into(),
            options,
            selected: None,
            placeholder: "Select...".into(),
            open: false,
            highlighted_index: 0,
            focus_handle: cx.focus_handle(),
            popover_focus_handle: cx.focus_handle(),
            disabled: false,
            focus_subscription: None,
            blur_subscription: None,
        }
    }

    /// Set the placeholder text when no option is selected.
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Set the initially selected value.
    pub fn selected(mut self, value: Option<T>) -> Self {
        self.selected = value;
        self
    }

    /// Set whether the select is disabled.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Get the currently selected value.
    pub fn selected_value(&self) -> Option<&T> {
        self.selected.as_ref()
    }

    /// Set the selected value programmatically.
    pub fn set_selected(&mut self, value: Option<T>, cx: &mut Context<Self>) {
        if let Some(ref v) = value {
            self.selected = Some(v.clone());
            cx.emit(SelectEvent::Changed(v.clone()));
        } else {
            self.selected = None;
        }
        cx.notify();
    }

    /// Get the label for the currently selected value.
    fn selected_label(&self) -> Option<&SharedString> {
        self.selected.as_ref().and_then(|selected| {
            self.options
                .iter()
                .find(|opt| &opt.value == selected)
                .map(|opt| &opt.label)
        })
    }

    /// Subscribe to focus events.
    fn subscribe_to_focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.focus_subscription.is_none() {
            let focus_sub = cx.on_focus(&self.focus_handle, window, |_this, _window, cx| {
                cx.notify();
            });
            self.focus_subscription = Some(focus_sub);
        }

        if self.blur_subscription.is_none() {
            let blur_sub = cx.on_blur(&self.popover_focus_handle, window, |this, _window, cx| {
                // Close dropdown when it loses focus
                if this.open {
                    this.open = false;
                    cx.emit(SelectEvent::Closed);
                    cx.notify();
                }
            });
            self.blur_subscription = Some(blur_sub);
        }
    }

    /// Open the dropdown.
    fn open_dropdown(&mut self, _: &Open, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled || self.open {
            return;
        }
        self.open = true;
        // Set highlighted to selected index or 0
        self.highlighted_index = self
            .selected
            .as_ref()
            .and_then(|selected| {
                self.options
                    .iter()
                    .position(|opt| &opt.value == selected && !opt.disabled)
            })
            .unwrap_or(0);
        // Focus the popover
        window.focus(&self.popover_focus_handle, cx);
        cx.emit(SelectEvent::Opened);
        cx.notify();
    }

    /// Close the dropdown.
    fn close_dropdown(&mut self, _: &Close, window: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            return;
        }
        self.open = false;
        window.focus(&self.focus_handle, cx);
        cx.emit(SelectEvent::Closed);
        cx.notify();
    }

    /// Select the next option.
    fn select_next(&mut self, _: &SelectNextOption, _: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            return;
        }
        // Find next non-disabled option
        let len = self.options.len();
        if len == 0 {
            return;
        }
        for i in 1..=len {
            let idx = (self.highlighted_index + i) % len;
            if !self.options[idx].disabled {
                self.highlighted_index = idx;
                cx.notify();
                return;
            }
        }
    }

    /// Select the previous option.
    fn select_previous(&mut self, _: &SelectPreviousOption, _: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            return;
        }
        // Find previous non-disabled option
        let len = self.options.len();
        if len == 0 {
            return;
        }
        for i in 1..=len {
            let idx = (self.highlighted_index + len - i) % len;
            if !self.options[idx].disabled {
                self.highlighted_index = idx;
                cx.notify();
                return;
            }
        }
    }

    /// Confirm the highlighted selection.
    fn confirm(&mut self, _: &Confirm, window: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            return;
        }
        if let Some(option) = self.options.get(self.highlighted_index) {
            if !option.disabled {
                let value = option.value.clone();
                self.selected = Some(value.clone());
                self.open = false;
                window.focus(&self.focus_handle, cx);
                cx.emit(SelectEvent::Changed(value));
                cx.emit(SelectEvent::Closed);
                cx.notify();
            }
        }
    }

    /// Handle clicking on an option.
    fn select_option(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(option) = self.options.get(index) {
            if !option.disabled {
                let value = option.value.clone();
                self.selected = Some(value.clone());
                self.open = false;
                window.focus(&self.focus_handle, cx);
                cx.emit(SelectEvent::Changed(value));
                cx.emit(SelectEvent::Closed);
                cx.notify();
            }
        }
    }

    /// Toggle the dropdown open/closed.
    fn toggle_dropdown(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        if self.open {
            self.open = false;
            window.focus(&self.focus_handle, cx);
            cx.emit(SelectEvent::Closed);
        } else {
            self.open = true;
            self.highlighted_index = self
                .selected
                .as_ref()
                .and_then(|selected| {
                    self.options
                        .iter()
                        .position(|opt| &opt.value == selected && !opt.disabled)
                })
                .unwrap_or(0);
            window.focus(&self.popover_focus_handle, cx);
            cx.emit(SelectEvent::Opened);
        }
        cx.notify();
    }

    /// Render the closed state trigger button.
    fn render_trigger(&self, theme: &TuskTheme, is_focused: bool) -> impl IntoElement {
        let display_text = self
            .selected_label()
            .cloned()
            .unwrap_or_else(|| self.placeholder.clone());

        let text_color = if self.selected.is_some() {
            theme.colors.text
        } else {
            theme.colors.text_muted
        };

        let opacity = if self.disabled { 0.5 } else { 1.0 };

        div()
            .h(px(32.0))
            .w_full()
            .px(px(12.0))
            .flex()
            .items_center()
            .justify_between()
            .gap(px(8.0))
            .bg(theme.colors.element_background)
            .border_1()
            .border_color(if is_focused && !self.disabled {
                theme.colors.accent
            } else {
                theme.colors.border
            })
            .rounded(px(4.0))
            .opacity(opacity)
            .when(!self.disabled, |el| el.cursor(CursorStyle::PointingHand))
            .when(!self.disabled, |el| {
                el.hover(|style| style.bg(theme.colors.element_hover))
            })
            .child(
                div()
                    .text_sm()
                    .text_color(text_color)
                    .overflow_hidden()
                    .child(display_text),
            )
            .child(
                Icon::new(if self.open {
                    IconName::ChevronUp
                } else {
                    IconName::ChevronDown
                })
                .size(IconSize::Small)
                .color(theme.colors.text_muted),
            )
    }

    /// Render the dropdown popover with options.
    fn render_popover(&self, theme: &TuskTheme, cx: &Context<Self>) -> impl IntoElement {
        let options_count = self.options.len();

        div()
            .id("select-popover-content")
            .key_context("SelectPopover")
            .track_focus(&self.popover_focus_handle)
            .on_action(cx.listener(Self::close_dropdown))
            .on_action(cx.listener(Self::select_next))
            .on_action(cx.listener(Self::select_previous))
            .on_action(cx.listener(Self::confirm))
            .min_w(px(120.0))
            .max_h(px(240.0))
            .overflow_y_scroll()
            .bg(theme.colors.elevated_surface_background)
            .border_1()
            .border_color(theme.colors.border)
            .rounded(px(4.0))
            .shadow_md()
            .py(px(4.0))
            .children((0..options_count).map(|index| {
                let option = &self.options[index];
                let is_selected = self
                    .selected
                    .as_ref()
                    .map(|s| s == &option.value)
                    .unwrap_or(false);
                let is_highlighted = index == self.highlighted_index;

                let bg_color = if is_highlighted {
                    theme.colors.list_active_selection_background
                } else if is_selected {
                    theme.colors.element_background
                } else {
                    gpui::transparent_black()
                };

                let text_color = if option.disabled {
                    theme.colors.text_muted.opacity(0.5)
                } else {
                    theme.colors.text
                };

                div()
                    .id(("option", index))
                    .h(px(28.0))
                    .px(px(12.0))
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .bg(bg_color)
                    .text_sm()
                    .text_color(text_color)
                    .when(!option.disabled, |el| {
                        el.cursor(CursorStyle::PointingHand)
                            .hover(|style| style.bg(theme.colors.ghost_element_hover))
                            .on_mouse_down(gpui::MouseButton::Left, |_, _, _| {})
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.select_option(index, window, cx);
                            }))
                    })
                    .when(is_selected, |el| {
                        el.child(
                            Icon::new(IconName::Check)
                                .size(IconSize::Small)
                                .color(theme.colors.accent),
                        )
                    })
                    .when(!is_selected, |el| el.child(div().w(px(14.0)))) // Spacer for alignment
                    .child(option.label.clone())
            }))
    }
}

impl<T: Clone + PartialEq + 'static> EventEmitter<SelectEvent<T>> for Select<T> {}

impl<T: Clone + PartialEq + 'static> Focusable for Select<T> {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        if self.open {
            self.popover_focus_handle.clone()
        } else {
            self.focus_handle.clone()
        }
    }
}

impl<T: Clone + PartialEq + 'static> Render for Select<T> {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Subscribe to focus events on first render
        self.subscribe_to_focus(window, cx);

        let theme = cx.global::<TuskTheme>();
        let is_focused = self.focus_handle.is_focused(window);

        let trigger = self.render_trigger(theme, is_focused);

        let mut container = div()
            .id(self.id.clone())
            .key_context("Select")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::open_dropdown))
            .relative()
            .w_full()
            .child(
                div()
                    .id("select-trigger")
                    .when(!self.disabled, |el| {
                        el.on_click(cx.listener(|this, _, window, cx| {
                            this.toggle_dropdown(window, cx);
                        }))
                    })
                    .child(trigger),
            );

        // Render popover when open
        if self.open {
            let popover = self.render_popover(theme, cx);

            container = container.child(
                deferred(
                    anchored()
                        .anchor(Corner::TopLeft)
                        .child(div().occlude().mt(px(4.0)).child(popover)),
                )
                .with_priority(1),
            );
        }

        container
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_option_construction() {
        let option = SelectOption::new("test_value", "Test Label");
        assert_eq!(option.value, "test_value");
        assert_eq!(option.label.as_ref(), "Test Label");
        assert!(!option.disabled);

        let disabled_option = SelectOption::new(42, "Number").disabled(true);
        assert!(disabled_option.disabled);
    }
}
