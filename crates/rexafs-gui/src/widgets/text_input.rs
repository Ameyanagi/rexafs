//! Single-line (or opt-in multiline) text input widget, adapted from gpui's examples/input.rs
//! (gpui 0.2.2). Emits [`InputEvent::Committed`] on Enter and, for single-line
//! inputs, on focus loss (doc: "Numeric fields commit on Enter/blur"). Emits
//! [`InputEvent::Edited`] on every user-initiated content change. Every input
//! is a tab stop. Tab / shift-Tab walk inputs in document order, committing
//! single-line fields; multiline drafts commit only on Enter.
//! [`text_input_keybindings`] must be registered with `cx.bind_keys` at
//! startup.

use std::ops::Range;

use gpui::{
    App, Bounds, ClipboardItem, Context, CursorStyle, ElementId, ElementInputHandler, Entity,
    EntityInputHandler, EventEmitter, FocusHandle, Focusable, GlobalElementId, KeyBinding,
    LayoutId, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, PaintQuad, Pixels, Point,
    ShapedLine, SharedString, Style, TextRun, UTF16Selection, UnderlineStyle, Window, actions, div,
    fill, point, prelude::*, px, relative, size,
};
use unicode_segmentation::*;

use crate::theme::Theme;

actions!(
    text_input,
    [
        Backspace,
        Delete,
        Left,
        Right,
        SelectLeft,
        SelectRight,
        SelectAll,
        Home,
        End,
        WordLeft,
        WordRight,
        DeleteToStart,
        ShowCharacterPalette,
        Paste,
        Cut,
        Copy,
        Commit,
        NextField,
        PrevField,
        StepUp,
        StepDown,
        InsertNewline,
    ]
);

/// Key bindings required by [`TextInput`]; register once at app startup.
pub fn text_input_keybindings() -> Vec<KeyBinding> {
    vec![
        KeyBinding::new("backspace", Backspace, None),
        KeyBinding::new("delete", Delete, None),
        KeyBinding::new("left", Left, None),
        KeyBinding::new("right", Right, None),
        KeyBinding::new("shift-left", SelectLeft, None),
        KeyBinding::new("shift-right", SelectRight, None),
        KeyBinding::new("cmd-a", SelectAll, None),
        KeyBinding::new("cmd-v", Paste, None),
        KeyBinding::new("cmd-c", Copy, None),
        KeyBinding::new("cmd-x", Cut, None),
        KeyBinding::new("home", Home, None),
        KeyBinding::new("end", End, None),
        // standard macOS line/word gestures
        KeyBinding::new("cmd-left", Home, None),
        KeyBinding::new("cmd-right", End, None),
        KeyBinding::new("alt-left", WordLeft, None),
        KeyBinding::new("alt-right", WordRight, None),
        KeyBinding::new("cmd-backspace", DeleteToStart, None),
        KeyBinding::new("ctrl-cmd-space", ShowCharacterPalette, None),
        KeyBinding::new("enter", Commit, None),
        KeyBinding::new("shift-enter", InsertNewline, Some("MultilineTextInput")),
        KeyBinding::new("tab", NextField, None),
        KeyBinding::new("shift-tab", PrevField, None),
        KeyBinding::new("up", StepUp, Some("TextInput")),
        KeyBinding::new("down", StepDown, Some("TextInput")),
    ]
}

/// Presentation options of a [`TextInput`].
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct InputStyle {
    /// Right-align the text (numbers line up on their last digit).
    pub align_right: bool,
    /// Monospace, tabular digits.
    pub mono: bool,
    /// Draw the placeholder in the accent colour (an "auto" value that the
    /// core computes, not an empty field).
    pub placeholder_accent: bool,
    /// Wrap text and allow explicit newlines. Enter still commits.
    pub multiline: bool,
    /// Maximum visible visual rows (clamped to at least one).
    pub max_lines: usize,
}

impl Default for InputStyle {
    fn default() -> Self {
        Self {
            align_right: false,
            mono: false,
            placeholder_accent: false,
            multiline: false,
            max_lines: 6,
        }
    }
}

pub enum InputEvent {
    /// Enter pressed (or single-line focus lost): the current text.
    Committed(SharedString),
    /// User edit (typing/backspace/paste/IME): the current text.
    /// Programmatic `set_text` deliberately does not emit, so subscribers
    /// can reset the field without feedback loops.
    Edited(SharedString),
    /// ↑ / ↓ pressed: +1 / −1 step (numeric fields interpret it).
    Step(i32),
}

impl EventEmitter<InputEvent> for TextInput {}

pub struct TextInput {
    focus_handle: FocusHandle,
    content: SharedString,
    placeholder: SharedString,
    theme: Theme,
    selected_range: Range<usize>,
    selection_reversed: bool,
    marked_range: Option<Range<usize>>,
    last_layout: Option<ShapedLine>,
    last_wrapped: Option<WrappedLayout>,
    scroll_y: Pixels,
    preferred_x: Option<Pixels>,
    /// Disambiguates the end/start of adjacent soft-wrapped rows.
    cursor_row: Option<usize>,
    last_bounds: Option<Bounds<Pixels>>,
    is_selecting: bool,
    /// Border flashes in the error color while set (rejected input).
    error: bool,
    /// Focus state seen by the previous render; a true→false transition is
    /// a blur, which commits like Enter. (Focus changes always redraw the
    /// window, so render observes every transition.)
    was_focused: bool,
    style: InputStyle,
    enabled: bool,
}

impl TextInput {
    pub fn new(
        placeholder: impl Into<SharedString>,
        initial: impl Into<SharedString>,
        theme: Theme,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            // equal tab_index everywhere -> Tab follows document order
            focus_handle: cx.focus_handle().tab_index(0).tab_stop(true),
            content: initial.into(),
            placeholder: placeholder.into(),
            theme,
            selected_range: 0..0,
            selection_reversed: false,
            marked_range: None,
            last_layout: None,
            last_wrapped: None,
            scroll_y: px(0.),
            preferred_x: None,
            cursor_row: None,
            last_bounds: None,
            is_selecting: false,
            error: false,
            was_focused: false,
            style: InputStyle::default(),
            enabled: true,
        }
    }

    pub fn set_enabled(&mut self, enabled: bool, cx: &mut Context<Self>) {
        if self.enabled != enabled {
            self.enabled = enabled;
            self.is_selecting = false;
            self.marked_range = None;
            cx.notify();
        }
    }

    /// Presentation options (alignment, mono, accent placeholder).
    pub fn with_style(mut self, style: InputStyle) -> Self {
        self.style = style;
        self
    }

    #[allow(dead_code)]
    pub fn text(&self) -> &str {
        &self.content
    }

    pub fn placeholder_text(&self) -> &str {
        &self.placeholder
    }

    pub fn set_text(&mut self, text: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.content = text.into();
        if self.style.multiline {
            self.scroll_y = px(0.);
            self.last_wrapped = None;
            self.preferred_x = None;
            self.cursor_row = None;
            self.marked_range = None;
            self.selection_reversed = false;
        }
        self.selected_range = self.content.len()..self.content.len();
        cx.notify();
    }

    pub fn set_theme(&mut self, theme: Theme, cx: &mut Context<Self>) {
        self.theme = theme;
        cx.notify();
    }

    pub fn set_placeholder(
        &mut self,
        placeholder: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) {
        self.placeholder = placeholder.into();
        cx.notify();
    }

    /// Flash (or clear) the rejected-input border.
    pub fn set_error(&mut self, error: bool, cx: &mut Context<Self>) {
        if self.error != error {
            self.error = error;
            cx.notify();
        }
    }

    fn commit(&mut self, _: &Commit, _window: &mut Window, cx: &mut Context<Self>) {
        if !self.is_composing() {
            cx.emit(InputEvent::Committed(self.content.clone()));
        }
    }

    pub(crate) fn is_composing(&self) -> bool {
        self.marked_range.is_some()
    }

    fn insert_newline(&mut self, _: &InsertNewline, window: &mut Window, cx: &mut Context<Self>) {
        if self.style.multiline && !self.is_composing() {
            self.replace_text_in_range(None, "\n", window, cx);
        }
    }

    /// Tab: commit single-line fields, then advance in the window's tab-stop ring.
    /// Every TextInput is a tab stop (equal index, so document order — the
    /// context panel renders fields in pipeline order); hidden fields are
    /// not painted and therefore skipped.
    fn next_field(&mut self, _: &NextField, window: &mut Window, cx: &mut Context<Self>) {
        if !self.style.multiline {
            cx.emit(InputEvent::Committed(self.content.clone()));
        }
        window.focus_next(cx);
    }

    fn prev_field(&mut self, _: &PrevField, window: &mut Window, cx: &mut Context<Self>) {
        if !self.style.multiline {
            cx.emit(InputEvent::Committed(self.content.clone()));
        }
        window.focus_prev(cx);
    }

    fn step_up(&mut self, _: &StepUp, _: &mut Window, cx: &mut Context<Self>) {
        if self.style.multiline {
            self.move_vertical(-1, cx);
        } else {
            cx.emit(InputEvent::Step(1));
        }
    }

    fn step_down(&mut self, _: &StepDown, _: &mut Window, cx: &mut Context<Self>) {
        if self.style.multiline {
            self.move_vertical(1, cx);
        } else {
            cx.emit(InputEvent::Step(-1));
        }
    }
    fn move_vertical(&mut self, direction: isize, cx: &mut Context<Self>) {
        let Some(layout) = self.last_wrapped.as_ref() else {
            return;
        };
        let (row, position) = layout.position(self.cursor_offset(), self.cursor_row);
        let x = self.preferred_x.unwrap_or(position.x);
        let target = row
            .saturating_add_signed(direction)
            .min(layout.rows.len().saturating_sub(1));
        let index = clamp_char_boundary(&self.content, layout.index_on_row(target, x));
        self.move_to(index, cx);
        self.preferred_x = Some(x);
        self.cursor_row = Some(target);
    }

    fn move_row_edge(&mut self, end: bool, cx: &mut Context<Self>) {
        let Some(layout) = self.last_wrapped.as_ref() else {
            return;
        };
        let row = row_for_index(&layout.rows, self.cursor_offset(), self.cursor_row);
        let Some(metrics) = layout.rows.get(row) else {
            return;
        };
        let index = if end {
            metrics.range.end
        } else {
            metrics.range.start
        };
        self.move_to(clamp_char_boundary(&self.content, index), cx);
        self.cursor_row = Some(row);
    }

    fn left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.previous_boundary(self.cursor_offset()), cx);
        } else {
            self.move_to(self.selected_range.start, cx)
        }
    }

    fn right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.next_boundary(self.selected_range.end), cx);
        } else {
            self.move_to(self.selected_range.end, cx)
        }
    }

    fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.previous_boundary(self.cursor_offset()), cx);
    }

    fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.next_boundary(self.cursor_offset()), cx);
    }

    pub(crate) fn select_all_text(&mut self, cx: &mut Context<Self>) {
        self.selected_range = 0..self.content.len();
        self.selection_reversed = false;
        cx.notify();
    }

    fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, cx);
        self.select_to(self.content.len(), cx)
    }

    fn home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        if self.style.multiline {
            self.move_row_edge(false, cx);
            return;
        }
        self.move_to(0, cx);
    }

    fn end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        if self.style.multiline {
            self.move_row_edge(true, cx);
            return;
        }
        self.move_to(self.content.len(), cx);
    }

    fn word_left(&mut self, _: &WordLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.previous_word_boundary(self.cursor_offset()), cx);
    }

    fn word_right(&mut self, _: &WordRight, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.next_word_boundary(self.cursor_offset()), cx);
    }

    fn delete_to_start(&mut self, _: &DeleteToStart, window: &mut Window, cx: &mut Context<Self>) {
        let end = self.selected_range.end;
        if end == 0 {
            return;
        }
        self.selected_range = 0..end;
        self.replace_text_in_range(None, "", window, cx)
    }

    fn backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            let prev = self.previous_boundary(self.cursor_offset());
            if self.cursor_offset() == prev {
                return;
            }
            self.select_to(prev, cx)
        }
        self.replace_text_in_range(None, "", window, cx)
    }

    fn delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            let next = self.next_boundary(self.cursor_offset());
            if self.cursor_offset() == next {
                return;
            }
            self.select_to(next, cx)
        }
        self.replace_text_in_range(None, "", window, cx)
    }

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.is_selecting = true;
        let index = self.index_for_mouse_position(event.position);

        if event.modifiers.shift {
            self.select_to(index, cx);
        } else if event.click_count >= 3 {
            self.selected_range = 0..self.content.len();
            if self.style.multiline {
                self.selection_reversed = false;
            }
            cx.notify();
        } else if event.click_count == 2 {
            self.select_word_at(index, cx);
        } else {
            self.move_to(index, cx)
        }
        self.remember_mouse_row(event.position);
    }

    /// Double-click: select the word-bound segment under `offset` (the last
    /// segment when the click lands past the end of the text).
    fn select_word_at(&mut self, offset: usize, cx: &mut Context<Self>) {
        let segment = self
            .content
            .split_word_bound_indices()
            .map(|(start, word)| start..start + word.len())
            .find(|range| range.contains(&offset) || offset < range.end)
            .or_else(|| {
                self.content
                    .split_word_bound_indices()
                    .next_back()
                    .map(|(start, word)| start..start + word.len())
            });
        if let Some(range) = segment {
            self.selected_range = range;
            self.selection_reversed = false;
            cx.notify();
        } else {
            self.move_to(offset, cx);
        }
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _window: &mut Window, _: &mut Context<Self>) {
        self.is_selecting = false;
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.is_selecting {
            self.select_to(self.index_for_mouse_position(event.position), cx);
            self.remember_mouse_row(event.position);
        }
    }

    fn show_character_palette(
        &mut self,
        _: &ShowCharacterPalette,
        window: &mut Window,
        _: &mut Context<Self>,
    ) {
        window.show_character_palette();
    }

    fn paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            if self.style.multiline {
                self.replace_text_in_range(None, &text, window, cx);
            } else {
                self.replace_text_in_range(None, &text.replace("\n", " "), window, cx);
            }
        }
    }

    fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content[self.selected_range.clone()].to_string(),
            ));
        }
    }
    fn cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content[self.selected_range.clone()].to_string(),
            ));
            self.replace_text_in_range(None, "", window, cx)
        }
    }

    fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        if self.style.multiline {
            self.preferred_x = None;
            self.cursor_row = None;
        }
        self.selected_range = offset..offset;
        cx.notify()
    }

    fn cursor_offset(&self) -> usize {
        if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        }
    }

    fn index_for_mouse_position(&self, position: Point<Pixels>) -> usize {
        if self.content.is_empty() {
            return 0;
        }

        if self.style.multiline {
            let index = self
                .last_wrapped
                .as_ref()
                .zip(self.last_bounds.as_ref())
                .map(|(layout, bounds)| {
                    layout.index_for_position(point(
                        position.x - bounds.left(),
                        position.y - bounds.top() + self.scroll_y,
                    ))
                })
                .unwrap_or_else(|| self.cursor_offset());
            return clamp_char_boundary(&self.content, index);
        }

        let (Some(bounds), Some(line)) = (self.last_bounds.as_ref(), self.last_layout.as_ref())
        else {
            return 0;
        };
        if position.y < bounds.top() {
            return 0;
        }
        if position.y > bounds.bottom() {
            return self.content.len();
        }
        clamp_char_boundary(
            &self.content,
            line.closest_index_for_x(position.x - bounds.left()),
        )
    }

    fn remember_mouse_row(&mut self, position: Point<Pixels>) {
        if self.style.multiline {
            self.preferred_x = None;
            self.cursor_row = self
                .last_wrapped
                .as_ref()
                .zip(self.last_bounds.as_ref())
                .map(|(layout, bounds)| {
                    row_at_y(
                        position.y - bounds.top() + self.scroll_y,
                        layout.line_height,
                        layout.rows.len(),
                    )
                });
        }
    }

    fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        if self.style.multiline {
            self.preferred_x = None;
            self.cursor_row = None;
        }
        if self.selection_reversed {
            self.selected_range.start = offset
        } else {
            self.selected_range.end = offset
        };
        if self.selected_range.end < self.selected_range.start {
            self.selection_reversed = !self.selection_reversed;
            self.selected_range = self.selected_range.end..self.selected_range.start;
        }
        cx.notify()
    }

    fn offset_from_utf16(&self, offset: usize) -> usize {
        let mut utf8_offset = 0;
        let mut utf16_count = 0;

        for ch in self.content.chars() {
            if utf16_count >= offset {
                break;
            }
            utf16_count += ch.len_utf16();
            utf8_offset += ch.len_utf8();
        }

        utf8_offset
    }

    fn offset_to_utf16(&self, offset: usize) -> usize {
        let mut utf16_offset = 0;
        let mut utf8_count = 0;

        for ch in self.content.chars() {
            if utf8_count >= offset {
                break;
            }
            utf8_count += ch.len_utf8();
            utf16_offset += ch.len_utf16();
        }

        utf16_offset
    }

    fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
    }

    fn range_from_utf16(&self, range_utf16: &Range<usize>) -> Range<usize> {
        self.offset_from_utf16(range_utf16.start)..self.offset_from_utf16(range_utf16.end)
    }

    fn previous_boundary(&self, offset: usize) -> usize {
        self.content
            .grapheme_indices(true)
            .rev()
            .find_map(|(idx, _)| (idx < offset).then_some(idx))
            .unwrap_or(0)
    }

    fn next_boundary(&self, offset: usize) -> usize {
        self.content
            .grapheme_indices(true)
            .find_map(|(idx, _)| (idx > offset).then_some(idx))
            .unwrap_or(self.content.len())
    }

    /// Start of the word before `offset` (alt-left semantics).
    fn previous_word_boundary(&self, offset: usize) -> usize {
        self.content
            .unicode_word_indices()
            .rev()
            .find_map(|(idx, _)| (idx < offset).then_some(idx))
            .unwrap_or(0)
    }

    /// End of the word after `offset` (alt-right semantics).
    fn next_word_boundary(&self, offset: usize) -> usize {
        self.content
            .unicode_word_indices()
            .find_map(|(idx, word)| {
                let end = idx + word.len();
                (end > offset).then_some(end)
            })
            .unwrap_or(self.content.len())
    }

    #[allow(dead_code)]
    fn reset(&mut self) {
        self.content = "".into();
        self.selected_range = 0..0;
        self.selection_reversed = false;
        self.marked_range = None;
        self.last_layout = None;
        self.last_wrapped = None;
        self.scroll_y = px(0.);
        self.preferred_x = None;
        self.cursor_row = None;
        self.last_bounds = None;
        self.is_selecting = false;
    }
}

impl EntityInputHandler for TextInput {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.range_from_utf16(&range_utf16);
        actual_range.replace(self.range_to_utf16(&range));
        Some(self.content[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.range_to_utf16(&self.selected_range),
            reversed: self.selection_reversed,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.marked_range
            .as_ref()
            .map(|range| self.range_to_utf16(range))
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.marked_range = None;
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());

        if self.style.multiline {
            self.preferred_x = None;
            self.cursor_row = None;
        }
        self.content =
            (self.content[0..range.start].to_owned() + new_text + &self.content[range.end..])
                .into();
        self.last_wrapped = None;
        self.last_layout = None;
        self.selected_range = range.start + new_text.len()..range.start + new_text.len();
        self.marked_range.take();
        cx.emit(InputEvent::Edited(self.content.clone()));
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());

        if self.style.multiline {
            self.preferred_x = None;
            self.cursor_row = None;
        }
        self.content =
            (self.content[0..range.start].to_owned() + new_text + &self.content[range.end..])
                .into();
        self.last_wrapped = None;
        self.last_layout = None;
        if !new_text.is_empty() {
            self.marked_range = Some(range.start..range.start + new_text.len());
        } else {
            self.marked_range = None;
        }
        if self.style.multiline {
            self.selected_range = new_selected_range_utf16
                .as_ref()
                .map(|selected| {
                    let start = utf16_to_byte(new_text, selected.start);
                    let end = utf16_to_byte(new_text, selected.end);
                    range.start + start..range.start + end
                })
                .unwrap_or_else(|| range.start + new_text.len()..range.start + new_text.len());
        } else {
            self.selected_range = new_selected_range_utf16
                .as_ref()
                .map(|range_utf16| self.range_from_utf16(range_utf16))
                .map(|new_range| new_range.start + range.start..new_range.end + range.end)
                .unwrap_or_else(|| range.start + new_text.len()..range.start + new_text.len());
        }
        cx.emit(InputEvent::Edited(self.content.clone()));
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let range = self.range_from_utf16(&range_utf16);
        if self.style.multiline {
            let layout = self.last_wrapped.as_ref()?;
            let hint = range.is_empty().then_some(self.cursor_row).flatten();
            let (row, start) = layout.position(range.start, hint);
            let end = layout.position_on_row(row, range.end);
            return Some(Bounds::new(
                bounds.origin + start - point(px(0.), self.scroll_y),
                size((end.x - start.x).max(px(0.)), layout.line_height),
            ));
        }
        let last_layout = self.last_layout.as_ref()?;
        Some(Bounds::from_corners(
            point(
                bounds.left() + last_layout.x_for_index(range.start),
                bounds.top(),
            ),
            point(
                bounds.left() + last_layout.x_for_index(range.end),
                bounds.bottom(),
            ),
        ))
    }

    fn character_index_for_point(
        &mut self,
        point: gpui::Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        if self.style.multiline {
            self.last_wrapped.as_ref()?;
            self.last_bounds?;
            return Some(self.offset_to_utf16(self.index_for_mouse_position(point)));
        }
        let line_point = self.last_bounds?.localize(&point)?;
        let last_layout = self.last_layout.as_ref()?;

        assert_eq!(last_layout.text, self.content);
        let utf8_index = clamp_char_boundary(
            &self.content,
            last_layout.index_for_x(point.x - line_point.x)?,
        );
        Some(self.offset_to_utf16(utf8_index))
    }
}

/// Cached layouts may refer to older text; never use their offsets inside a UTF-8 character.
fn clamp_char_boundary(text: &str, index: usize) -> usize {
    let mut index = index.min(text.len());
    while !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

/// Byte ranges exclude paragraph separators, but retain empty/trailing paragraphs.
fn paragraph_ranges(text: &str) -> Vec<Range<usize>> {
    let mut offset = 0;
    text.split('\n')
        .map(|paragraph| {
            let range = offset..offset + paragraph.len();
            offset = range.end + 1;
            range
        })
        .collect()
}

fn utf16_to_byte(text: &str, offset: usize) -> usize {
    let mut units = 0;
    for (index, ch) in text.char_indices() {
        if units >= offset {
            return index;
        }
        units += ch.len_utf16();
    }
    text.len()
}

#[derive(Debug)]
struct VisualRow {
    paragraph: usize,
    paragraph_row: usize,
    range: Range<usize>,
    /// Horizontal offset in the unwrapped paragraph.
    x: Pixels,
    width: Pixels,
}

/// Wrap boundaries are paragraph-local byte indices and unwrapped x positions.
fn paragraph_rows(
    paragraph: usize,
    range: Range<usize>,
    boundaries: &[(usize, Pixels)],
    width: Pixels,
) -> Vec<VisualRow> {
    let mut start = (0, px(0.));
    boundaries
        .iter()
        .copied()
        .chain([(range.len(), width)])
        .enumerate()
        .map(|(paragraph_row, end)| {
            let row = VisualRow {
                paragraph,
                paragraph_row,
                range: range.start + start.0..range.start + end.0,
                x: start.1,
                width: (end.1 - start.1).max(px(0.)),
            };
            start = end;
            row
        })
        .collect()
}

/// A wrap boundary normally belongs to the following row. Mouse/End/vertical
/// movement can retain the preceding row through an explicit affinity hint.
fn row_for_index(rows: &[VisualRow], index: usize, hint: Option<usize>) -> usize {
    if let Some(row) = hint.filter(|&row| {
        rows.get(row)
            .is_some_and(|r| r.range.start <= index && index <= r.range.end)
    }) {
        return row;
    }
    rows.iter()
        .rposition(|row| row.range.start <= index)
        .unwrap_or(0)
}

fn index_from_row(row: &VisualRow, paragraphs: &[Range<usize>], local: usize) -> usize {
    paragraphs[row.paragraph]
        .start
        .saturating_add(local)
        .clamp(row.range.start, row.range.end)
}

fn row_at_y(y: Pixels, line_height: Pixels, count: usize) -> usize {
    ((y.max(px(0.)) / line_height.max(px(1.))) as usize).min(count.saturating_sub(1))
}

fn visible_height(rows: usize, max_lines: usize, line_height: Pixels) -> Pixels {
    line_height * rows.max(1).min(max_lines.max(1))
}

fn scroll_to_row(
    scroll: Pixels,
    row: usize,
    count: usize,
    height: Pixels,
    line_height: Pixels,
) -> Pixels {
    let top = line_height * row;
    let scroll = scroll.min(top).max(top + line_height - height);
    scroll
        .max(px(0.))
        .min((line_height * count - height).max(px(0.)))
}

/// Rectangles in content coordinates, before applying the viewport/scroll offset.
fn selection_rects(
    start: (usize, Pixels),
    end: (usize, Pixels),
    width: Pixels,
    line_height: Pixels,
) -> Vec<Bounds<Pixels>> {
    (start.0..=end.0)
        .filter_map(|row| {
            let left = if row == start.0 { start.1 } else { px(0.) };
            let right = if row == end.0 { end.1 } else { width };
            (right > left).then(|| {
                Bounds::new(
                    point(left, line_height * row),
                    size(right - left, line_height),
                )
            })
        })
        .collect()
}

struct WrappedLayout {
    paragraphs: Vec<Range<usize>>,
    lines: Vec<gpui::WrappedLine>,
    rows: Vec<VisualRow>,
    line_height: Pixels,
}

impl WrappedLayout {
    fn shape(
        text: SharedString,
        font_size: Pixels,
        runs: &[TextRun],
        width: Pixels,
        line_height: Pixels,
        window: &Window,
    ) -> Self {
        let paragraphs = paragraph_ranges(&text);
        let mut shaped = window
            .text_system()
            .shape_text(text, font_size, runs, Some(width.max(px(1.))), None)
            .unwrap_or_default()
            .into_iter();
        let mut lines = Vec::new();
        let mut rows = Vec::new();
        for (paragraph, range) in paragraphs.iter().enumerate() {
            // Retain a row even if shaping fails, including empty paragraphs.
            let line = shaped.next().unwrap_or_default();
            let boundaries = line
                .wrap_boundaries()
                .iter()
                .filter_map(|boundary| {
                    let glyph = line
                        .runs()
                        .get(boundary.run_ix)?
                        .glyphs
                        .get(boundary.glyph_ix)?;
                    Some((glyph.index, glyph.position.x))
                })
                .collect::<Vec<_>>();
            rows.extend(paragraph_rows(
                paragraph,
                range.clone(),
                &boundaries,
                line.unwrapped_layout.width,
            ));
            lines.push(line);
        }
        Self {
            paragraphs,
            lines,
            rows,
            line_height,
        }
    }

    fn position_on_row(&self, row: usize, index: usize) -> Point<Pixels> {
        let Some(metrics) = self.rows.get(row) else {
            return point(px(0.), px(0.));
        };
        let local = index.clamp(metrics.range.start, metrics.range.end)
            - self.paragraphs[metrics.paragraph].start;
        let line = &self.lines[metrics.paragraph];
        // position_for_index uses upstream affinity at wrap boundaries. Resolve
        // a differing explicit row using the unwrapped x coordinate instead.
        let x = line
            .position_for_index(local, self.line_height)
            .map(|position| {
                if row_at_y(position.y, self.line_height, usize::MAX) == metrics.paragraph_row {
                    position.x
                } else {
                    line.unwrapped_layout.x_for_index(local) - metrics.x
                }
            })
            .unwrap_or(metrics.width);
        point(x, self.line_height * row)
    }

    fn position(&self, index: usize, hint: Option<usize>) -> (usize, Point<Pixels>) {
        let row = row_for_index(&self.rows, index, hint);
        (row, self.position_on_row(row, index))
    }

    fn index_on_row(&self, row: usize, x: Pixels) -> usize {
        let Some(metrics) = self.rows.get(row) else {
            return 0;
        };
        let line = &self.lines[metrics.paragraph];
        if line.len() < self.paragraphs[metrics.paragraph].len() {
            return metrics.range.end;
        }
        let local = line
            .closest_index_for_position(
                point(x, self.line_height * metrics.paragraph_row),
                self.line_height,
            )
            .unwrap_or_else(|end| end);
        index_from_row(metrics, &self.paragraphs, local)
    }

    fn index_for_position(&self, position: Point<Pixels>) -> usize {
        if position.y < px(0.) {
            return 0;
        }
        if position.y >= self.line_height * self.rows.len() {
            return self.paragraphs.last().map_or(0, |range| range.end);
        }
        self.index_on_row(
            row_at_y(position.y, self.line_height, self.rows.len()),
            position.x,
        )
    }
}

struct TextElement {
    input: Entity<TextInput>,
}

struct PrepaintState {
    line: Option<ShapedLine>,
    wrapped: Option<WrappedLayout>,
    selections: Vec<PaintQuad>,
    scroll_y: Pixels,
    cursor: Option<PaintQuad>,
    selection: Option<PaintQuad>,
    /// Bounds the text was laid out in (shifted for right alignment).
    text_bounds: Bounds<Pixels>,
}

impl IntoElement for TextElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TextElement {
    type RequestLayoutState = ();
    type PrepaintState = PrepaintState;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        if self.input.read(cx).style.multiline {
            let input = self.input.read(cx);
            let content = input.content.clone();
            let max_lines = input.style.max_lines;
            let text_style = window.text_style();
            let font_size = text_style.font_size.to_pixels(window.rem_size());
            let line_height = window.line_height();
            let runs = vec![text_style.to_run(content.len())];
            return (
                window.request_measured_layout(style, move |known, available, window, _| {
                    let width = known
                        .width
                        .unwrap_or(match available.width {
                            gpui::AvailableSpace::Definite(width) => width,
                            gpui::AvailableSpace::MinContent => px(1.),
                            gpui::AvailableSpace::MaxContent => px(100000.),
                        })
                        .max(px(1.));
                    let layout = WrappedLayout::shape(
                        content.clone(),
                        font_size,
                        &runs,
                        width,
                        line_height,
                        window,
                    );
                    size(
                        width,
                        visible_height(layout.rows.len(), max_lines, line_height),
                    )
                }),
                (),
            );
        }
        style.size.height = window.line_height().into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let input = self.input.read(cx);
        let content = input.content.clone();
        let selected_range = input.selected_range.clone();
        let cursor = input.cursor_offset();
        let style = window.text_style();

        let (display_text, text_color) = if content.is_empty() {
            let color = if input.style.placeholder_accent {
                input.theme.accent
            } else {
                input.theme.text_muted
            };
            (input.placeholder.clone(), color.into())
        } else {
            (content, style.color)
        };

        let run = TextRun {
            len: display_text.len(),
            font: style.font(),
            color: text_color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let runs = if let Some(marked_range) = input.marked_range.as_ref() {
            vec![
                TextRun {
                    len: marked_range.start,
                    ..run.clone()
                },
                TextRun {
                    len: marked_range.end - marked_range.start,
                    underline: Some(UnderlineStyle {
                        color: Some(run.color),
                        thickness: px(1.0),
                        wavy: false,
                    }),
                    ..run.clone()
                },
                TextRun {
                    len: display_text.len() - marked_range.end,
                    ..run
                },
            ]
            .into_iter()
            .filter(|run| run.len > 0)
            .collect()
        } else {
            vec![run]
        };

        let font_size = style.font_size.to_pixels(window.rem_size());
        if input.style.multiline {
            let line_height = window.line_height();
            // Keep the empty placeholder on one row, as in single-line inputs.
            let width = if input.content.is_empty() {
                px(100000.)
            } else {
                bounds.size.width
            };
            let layout =
                WrappedLayout::shape(display_text, font_size, &runs, width, line_height, window);
            let (row, cursor_position) = layout.position(cursor, input.cursor_row);
            let scroll_y = if input.content.is_empty() {
                px(0.)
            } else {
                scroll_to_row(
                    input.scroll_y,
                    row,
                    layout.rows.len(),
                    bounds.size.height,
                    line_height,
                )
            };
            let origin = bounds.origin - point(px(0.), scroll_y);
            let mut color = input.theme.accent;
            color.a = 0.25;
            let selections = if selected_range.is_empty() {
                Vec::new()
            } else {
                let (start_row, start) = layout.position(selected_range.start, None);
                let (end_row, end) = layout.position(selected_range.end, None);
                selection_rects(
                    (start_row, start.x),
                    (end_row, end.x),
                    bounds.size.width,
                    line_height,
                )
                .into_iter()
                .map(|rect| fill(Bounds::new(origin + rect.origin, rect.size), color))
                .collect()
            };
            return PrepaintState {
                line: None,
                wrapped: Some(layout),
                selections,
                scroll_y,
                cursor: selected_range.is_empty().then(|| {
                    fill(
                        Bounds::new(origin + cursor_position, size(px(2.), line_height)),
                        input.theme.accent,
                    )
                }),
                selection: None,
                text_bounds: bounds,
            };
        }
        let line = window
            .text_system()
            .shape_line(display_text, font_size, &runs, None);

        // Right-aligned text starts at the right edge minus its width; the
        // bounds handed to the paint / hit-test helpers are shifted the same
        // way so every `bounds.left() + x` stays correct.
        let bounds = if input.style.align_right {
            let shift = (bounds.size.width - line.width).max(px(0.));
            Bounds::new(
                point(bounds.left() + shift, bounds.top()),
                size(bounds.size.width - shift, bounds.size.height),
            )
        } else {
            bounds
        };
        let cursor_pos = line.x_for_index(cursor);
        let (selection, cursor) = if selected_range.is_empty() {
            (
                None,
                Some(fill(
                    Bounds::new(
                        point(bounds.left() + cursor_pos, bounds.top()),
                        size(px(2.), bounds.bottom() - bounds.top()),
                    ),
                    input.theme.accent,
                )),
            )
        } else {
            (
                Some(fill(
                    Bounds::from_corners(
                        point(
                            bounds.left() + line.x_for_index(selected_range.start),
                            bounds.top(),
                        ),
                        point(
                            bounds.left() + line.x_for_index(selected_range.end),
                            bounds.bottom(),
                        ),
                    ),
                    {
                        let mut c = input.theme.accent;
                        c.a = 0.25;
                        c
                    },
                )),
                None,
            )
        };
        PrepaintState {
            line: Some(line),
            wrapped: None,
            selections: Vec::new(),
            scroll_y: px(0.),
            cursor,
            selection,
            text_bounds: bounds,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus_handle = self.input.read(cx).focus_handle.clone();
        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.input.clone()),
            cx,
        );
        if let Some(layout) = prepaint.wrapped.take() {
            let bounds = prepaint.text_bounds;
            window.with_content_mask(Some(gpui::ContentMask { bounds }), |window| {
                for selection in prepaint.selections.drain(..) {
                    window.paint_quad(selection);
                }
                let mut origin = bounds.origin - point(px(0.), prepaint.scroll_y);
                for line in &layout.lines {
                    let _ = line.paint(
                        origin,
                        layout.line_height,
                        gpui::TextAlign::Left,
                        None,
                        window,
                        cx,
                    );
                    origin.y += line.size(layout.line_height).height;
                }
                if focus_handle.is_focused(window)
                    && let Some(cursor) = prepaint.cursor.take()
                {
                    window.paint_quad(cursor);
                }
            });
            self.input.update(cx, |input, _| {
                input.scroll_y = prepaint.scroll_y;
                input.last_wrapped = Some(layout);
                input.last_bounds = Some(bounds);
            });
            return;
        }
        if let Some(selection) = prepaint.selection.take() {
            window.paint_quad(selection)
        }
        let line = prepaint.line.take().unwrap();
        let bounds = prepaint.text_bounds;
        line.paint(
            bounds.origin,
            window.line_height(),
            gpui::TextAlign::Left,
            None,
            window,
            cx,
        )
        .unwrap();

        if focus_handle.is_focused(window)
            && let Some(cursor) = prepaint.cursor.take()
        {
            window.paint_quad(cursor);
        }

        self.input.update(cx, |input, _cx| {
            input.last_layout = Some(line);
            input.last_bounds = Some(bounds);
        });
    }
}

impl Render for TextInput {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = self.theme;
        if !self.enabled {
            // No focus tracking, action listeners, or IME input handler while disabled.
            self.was_focused = false;
            return div()
                .w_full()
                .min_w_0()
                .max_h(px(self.style.max_lines.max(1) as f32 * 18. + 6.))
                .overflow_hidden()
                .px(px(6.))
                .py(px(3.))
                .rounded_sm()
                .bg(t.bg)
                .border_1()
                .border_color(t.border)
                .text_color(t.text_muted)
                .opacity(0.5)
                .line_height(px(18.))
                .text_size(px(12.))
                .child(if self.content.is_empty() {
                    self.placeholder.clone()
                } else {
                    self.content.clone()
                })
                .into_any_element();
        }
        let focused = self.focus_handle.is_focused(window);
        if self.was_focused && !focused && !self.style.multiline {
            // blur commits exactly like Enter (doc: "commit on Enter/blur")
            cx.emit(InputEvent::Committed(self.content.clone()));
        }
        self.was_focused = focused;
        div()
            .flex()
            .key_context(if self.style.multiline {
                "TextInput MultilineTextInput"
            } else {
                "TextInput"
            })
            .track_focus(&self.focus_handle(cx))
            .cursor(CursorStyle::IBeam)
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::left))
            .on_action(cx.listener(Self::right))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::select_right))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .on_action(cx.listener(Self::word_left))
            .on_action(cx.listener(Self::word_right))
            .on_action(cx.listener(Self::delete_to_start))
            .on_action(cx.listener(Self::show_character_palette))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::cut))
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(Self::commit))
            .on_action(cx.listener(Self::insert_newline))
            .on_action(cx.listener(Self::next_field))
            .on_action(cx.listener(Self::prev_field))
            .on_action(cx.listener(Self::step_up))
            .on_action(cx.listener(Self::step_down))
            .when(self.style.mono, |d| d.font_family("Menlo"))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .w_full()
            .rounded_sm()
            .bg(t.bg)
            .border_1()
            .border_color(if self.error {
                t.error
            } else if focused {
                t.accent
            } else {
                t.border
            })
            .text_color(t.text)
            .line_height(px(18.))
            .text_size(px(12.))
            .child(
                div()
                    .when(!self.style.multiline, |d| d.h(px(18. + 3. * 2.)))
                    .when(self.style.multiline, |d| d.min_w_0().overflow_hidden())
                    .w_full()
                    .px(px(6.))
                    .py(px(3.))
                    .child(TextElement { input: cx.entity() }),
            )
            .into_any_element()
    }
}

impl Focusable for TextInput {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_input_clamps_cached_offsets_to_current_char_boundaries() {
        for text in ["", "ascii", "あい", "aé界😀", "e\u{301}\n"] {
            for index in (0..=text.len() + 2).chain([usize::MAX]) {
                let clamped = clamp_char_boundary(text, index);
                let expected = (0..=index.min(text.len()))
                    .rev()
                    .find(|&i| text.is_char_boundary(i))
                    .expect("zero is a character boundary");
                assert_eq!(clamped, expected, "{text:?} at {index}");
                assert!(text.get(..clamped).is_some());
            }
        }
        // Byte 4 was a boundary in "aあい", but is inside 'い' after deleting 'a'.
        assert_eq!(clamp_char_boundary("aあい", 4), 4);
        assert_eq!(clamp_char_boundary("あい", 4), 3);
    }

    #[test]
    fn text_input_wrapped_hit_testing_uses_nearest_boundary() {
        use gpui::{
            FontId, GlyphId, LineLayout, ShapedGlyph, ShapedRun, WrapBoundary, WrappedLine,
            WrappedLineLayout,
        };
        use std::sync::Arc;

        // Proportional glyph widths: "wi" / "wi", with a soft wrap at byte 2.
        let mut line = WrappedLine::default();
        *line = Arc::new(WrappedLineLayout {
            unwrapped_layout: Arc::new(LineLayout {
                len: 4,
                width: px(48.),
                runs: vec![ShapedRun {
                    font_id: FontId(0),
                    glyphs: [0., 20., 24., 44.]
                        .into_iter()
                        .enumerate()
                        .map(|(index, x)| ShapedGlyph {
                            id: GlyphId(0),
                            position: point(px(x), px(0.)),
                            index,
                            is_emoji: false,
                        })
                        .collect(),
                }],
                ..Default::default()
            }),
            wrap_boundaries: [WrapBoundary {
                run_ix: 0,
                glyph_ix: 2,
            }]
            .into_iter()
            .collect(),
            wrap_width: Some(px(24.)),
        });
        let layout = WrappedLayout {
            paragraphs: paragraph_ranges("wiwi"),
            lines: vec![line],
            rows: paragraph_rows(0, 0..4, &[(2, px(24.))], px(48.)),
            line_height: px(18.),
        };
        for row in 0..2 {
            for (x, local) in [(-10., 0), (9., 0), (11., 1), (20., 1), (23., 2), (100., 2)] {
                let expected = row * 2 + local;
                assert_eq!(layout.index_on_row(row, px(x)), expected);
                assert_eq!(
                    layout.index_for_position(point(px(x), px(18.) * row)),
                    expected
                );
            }
        }
        // The x coordinate of a caret survives a round trip between visual rows.
        let (_, caret) = layout.position(1, Some(0));
        assert_eq!(layout.index_on_row(1, caret.x), 3);
        assert_eq!(layout.index_on_row(0, caret.x), 1);
    }

    fn fake_rows() -> Vec<VisualRow> {
        // "abc def\n\né界x": two wrapped paragraphs, with an empty one between.
        let mut rows = paragraph_rows(0, 0..7, &[(4, px(40.))], px(70.));
        rows.extend(paragraph_rows(1, 8..8, &[], px(0.)));
        rows.extend(paragraph_rows(2, 9..15, &[(2, px(12.))], px(42.)));
        rows
    }

    #[test]
    fn text_input_style_defaults_preserve_single_line() {
        let style = InputStyle::default();
        assert!(!style.multiline);
        assert_eq!(style.max_lines, 6);
        assert!(!style.align_right && !style.mono && !style.placeholder_accent);
    }

    #[test]
    fn text_input_paragraph_offsets_include_empty_and_utf8_paragraphs() {
        assert_eq!(paragraph_ranges(""), vec![0..0]);
        assert_eq!(paragraph_ranges("\n"), vec![0..0, 1..1]);
        assert_eq!(paragraph_ranges("é\n\n界\n"), vec![0..2, 3..3, 4..7, 8..8]);
        assert_eq!(paragraph_ranges("abc def\n\né界x"), vec![0..7, 8..8, 9..15]);
    }

    #[test]
    fn text_input_indices_map_to_paragraph_and_visual_row() {
        let rows = fake_rows();
        for (index, paragraph, local_row) in [
            (0, 0, 0),
            (3, 0, 0),
            (4, 0, 1),
            (7, 0, 1),
            (8, 1, 0),
            (9, 2, 0),
            (11, 2, 1),
            (14, 2, 1),
            (15, 2, 1),
            (100, 2, 1),
        ] {
            let row = &rows[row_for_index(&rows, index, None)];
            assert_eq!(
                (row.paragraph, row.paragraph_row),
                (paragraph, local_row),
                "index {index}"
            );
        }
        assert_eq!(row_for_index(&rows, 4, Some(0)), 0);
        assert_eq!(row_for_index(&rows, 4, Some(1)), 1);
        assert_eq!(row_for_index(&rows, 14, Some(0)), 4);
        assert_eq!(row_for_index(&rows, 4, Some(99)), 1);
        assert_eq!(rows[1].width, px(30.));
        assert_eq!(rows[4].x, px(12.));
    }

    #[test]
    fn text_input_row_local_indices_round_trip_and_clamp() {
        let rows = fake_rows();
        let paragraphs = paragraph_ranges("abc def\n\né界x");
        for index in [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 11, 14, 15] {
            let row = row_for_index(&rows, index, None);
            let local = index - paragraphs[rows[row].paragraph].start;
            assert_eq!(index_from_row(&rows[row], &paragraphs, local), index);
        }
        assert_eq!(index_from_row(&rows[0], &paragraphs, 99), 4);
        assert_eq!(index_from_row(&rows[1], &paragraphs, 0), 4);
        assert_eq!(index_from_row(&rows[2], &paragraphs, 99), 8);
        assert_eq!(index_from_row(&rows[4], &paragraphs, 99), 15);
        assert_eq!(row_at_y(px(-20.), px(18.), rows.len()), 0);
        assert_eq!(row_at_y(px(18.), px(18.), rows.len()), 1);
        assert_eq!(row_at_y(px(900.), px(18.), rows.len()), 4);
    }

    #[test]
    fn text_input_selection_rectangles_cover_each_visual_row() {
        assert_eq!(
            selection_rects((1, px(10.)), (1, px(35.)), px(100.), px(18.)),
            vec![Bounds::new(point(px(10.), px(18.)), size(px(25.), px(18.))),]
        );
        assert_eq!(
            selection_rects((0, px(20.)), (3, px(12.)), px(100.), px(18.)),
            vec![
                Bounds::new(point(px(20.), px(0.)), size(px(80.), px(18.))),
                Bounds::new(point(px(0.), px(18.)), size(px(100.), px(18.))),
                Bounds::new(point(px(0.), px(36.)), size(px(100.), px(18.))),
                Bounds::new(point(px(0.), px(54.)), size(px(12.), px(18.))),
            ]
        );
        assert!(selection_rects((0, px(4.)), (0, px(4.)), px(100.), px(18.)).is_empty());
        // Selecting a newline fills the preceding row, with no empty final rect.
        assert_eq!(
            selection_rects((1, px(30.)), (2, px(0.)), px(100.), px(18.)).len(),
            1
        );
    }

    #[test]
    fn text_input_height_and_scroll_follow_cursor_and_reset() {
        assert_eq!(visible_height(0, 6, px(18.)), px(18.));
        assert_eq!(visible_height(2, 6, px(18.)), px(36.));
        assert_eq!(visible_height(20, 8, px(18.)), px(144.));
        assert_eq!(visible_height(3, 0, px(18.)), px(18.));
        assert_eq!(scroll_to_row(px(0.), 9, 10, px(144.), px(18.)), px(36.));
        assert_eq!(scroll_to_row(px(36.), 0, 10, px(144.), px(18.)), px(0.));
        assert_eq!(scroll_to_row(px(36.), 0, 1, px(18.), px(18.)), px(0.));
    }

    #[test]
    fn text_input_ime_selection_offsets_are_relative_to_inserted_text() {
        let text = "é\n😀界";
        assert_eq!(utf16_to_byte(text, 0), 0);
        assert_eq!(utf16_to_byte(text, 1), 2);
        assert_eq!(utf16_to_byte(text, 2), 3);
        assert_eq!(utf16_to_byte(text, 4), 7);
        assert_eq!(utf16_to_byte(text, 5), 10);
        assert_eq!(utf16_to_byte(text, 99), text.len());
    }
}
