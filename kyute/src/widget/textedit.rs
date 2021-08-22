//! Text editor widget.
use crate::{
    core::Node,
    env::Environment,
    event::{Event, Modifiers, PointerEventKind},
    style::{State, StyleSet},
    theme, BoxConstraints, CompositionCtx, EnvKey, EventCtx, Key, LayoutCtx, Measurements, Offset,
    PaintCtx, Point, Rect, SideOffsets, Size, Widget,
};
use keyboard_types::KeyState;
use kyute_shell::{
    drawing::{Brush, Color, DrawTextOptions},
    text::{TextFormat, TextFormatBuilder, TextLayout},
    winit::event::VirtualKeyCode,
};
use std::{any::Any, ops::Range, sync::Arc};
use tracing::trace;
use unicode_segmentation::GraphemeCursor;

/// Text selection.
///
/// Start is the start of the selection, end is the end. The caret is at the end of the selection.
/// Note that we don't necessarily have start <= end: a selection with start > end means that the
/// user started the selection gesture from a later point in the text and then went back
/// (right-to-left in LTR languages). In this case, the cursor will appear at the "beginning"
/// (i.e. left, for LTR) of the selection.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Selection {
    pub start: usize,
    pub end: usize,
}

impl Selection {
    pub fn min(&self) -> usize {
        self.start.min(self.end)
    }
    pub fn max(&self) -> usize {
        self.start.max(self.end)
    }
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }
    pub fn empty(at: usize) -> Selection {
        Selection { start: at, end: at }
    }
}

impl Default for Selection {
    fn default() -> Self {
        Selection::empty(0)
    }
}

// layout strategy:
// - the text layout is calculated during widget layout, but also when an event causes the text to
//   change
// - update the text layout during painting if necessary

pub struct TextEdit {
    /// Formatting information.
    text_format: TextFormat,

    /// The text displayed to the user.
    text: String,

    background_style: Arc<StyleSet>,

    /// The offset to the content area
    content_offset: Offset,

    /// The size of the content area
    content_size: Size,

    /// The text layout. None if not yet calculated.
    ///
    /// FIXME: due to DirectWrite limitations, the text layout contains a copy of the string.
    /// in the future, de-duplicate.
    text_layout: Option<TextLayout>,

    /// The currently selected range. If no text is selected, this is a zero-length range
    /// at the cursor position.
    selection: Selection,

    text_color: Color,
    selection_color: Color,
    selected_text_color: Color,
}

pub enum Movement {
    Left,
    Right,
    LeftWord,
    RightWord,
}

fn prev_grapheme_cluster(text: &str, offset: usize) -> Option<usize> {
    let mut c = GraphemeCursor::new(offset, text.len(), true);
    c.prev_boundary(&text, 0).unwrap()
}

fn next_grapheme_cluster(text: &str, offset: usize) -> Option<usize> {
    let mut c = GraphemeCursor::new(offset, text.len(), true);
    c.next_boundary(&text, 0).unwrap()
}

impl TextEdit {
    pub fn new(
        text: impl Into<String>,
        text_format: TextFormat,
        background_style: Arc<StyleSet>,
        text_color: Color,
        selection_color: Color,
        selected_text_color: Color,
    ) -> TextEdit {
        TextEdit {
            text_format,
            text: text.into(),
            background_style,
            content_offset: Default::default(),
            content_size: Default::default(),
            text_layout: None,
            selection: Default::default(),
            text_color,
            selection_color,
            selected_text_color,
        }
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text_layout = None;
        self.text = text.into();
    }

    /// Moves the cursor forward or backward.
    pub fn move_cursor(&mut self, movement: Movement, modify_selection: bool) {
        let offset =
            match movement {
                Movement::Left => prev_grapheme_cluster(&self.text, self.selection.end)
                    .unwrap_or(self.selection.end),
                Movement::Right => next_grapheme_cluster(&self.text, self.selection.end)
                    .unwrap_or(self.selection.end),
                Movement::LeftWord | Movement::RightWord => {
                    // TODO word navigation (unicode word segmentation)
                    unimplemented!()
                }
            };

        if modify_selection {
            self.selection.end = offset;
        } else {
            self.selection = Selection::empty(offset);
        }
    }

    /// Inserts text.
    pub fn insert(&mut self, text: &str) {
        let min = self.selection.min();
        let max = self.selection.max();
        self.text.replace_range(min..max, text);
        self.selection = Selection::empty(min + text.len());
    }

    /// Sets cursor position.
    pub fn set_cursor(&mut self, pos: usize) {
        if self.selection.is_empty() && self.selection.end == pos {
            return;
        }
        self.selection = Selection::empty(pos);
        // reset blink
    }

    pub fn set_selection_end(&mut self, pos: usize) {
        if self.selection.end == pos {
            return;
        }
        self.selection.end = pos;
        // reset blink
    }

    pub fn select_all(&mut self) {
        self.selection.start = 0;
        self.selection.end = self.text.len();
    }

    fn position_to_text(&mut self, pos: Point) -> usize {
        let hit = self
            .text_layout
            .expect("position_to_text called before layout")
            .hit_test_point(pos)
            .unwrap();
        let pos = if hit.is_trailing_hit {
            hit.metrics.text_position + hit.metrics.length
        } else {
            hit.metrics.text_position
        };
        pos
    }
}

impl Widget for TextEdit {
    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx,
        _children: &mut [Node],
        constraints: &BoxConstraints,
    ) -> Measurements {
        let padding = self.background_style.content_padding();
        let font_size = self.text_format.font_size() as f64;

        const SELECTION_MAGIC: f64 = 3.0;
        // why default width == 200?
        let size = Size::new(
            constraints.constrain_width(200.0),
            constraints.constrain_height(font_size + SELECTION_MAGIC + padding.vertical()),
        );

        let content_size = Size::new(
            size.width - padding.horizontal(),
            size.height - padding.vertical(),
        );

        let text_layout = TextLayout::new(&self.text, &self.text_format, content_size)
            .expect("could not create TextLayout");

        let content_offset = Offset::new(padding.left, padding.top);

        // calculate baseline
        let baseline = text_layout
            .line_metrics()
            .first()
            .map(|m| content_offset.y + m.baseline as f64);

        self.content_size = content_size;
        self.content_offset = content_offset;
        self.text_layout = Some(text_layout);
        Measurements { size, baseline }
    }

    fn paint(&mut self, ctx: &mut PaintCtx, children: &mut [Node], bounds: Rect) {
        let bounds = ctx.bounds();
        let text_layout = self
            .text_layout
            .as_mut()
            .expect("paint called before layout");

        //self.background_style.draw_box(ctx, &bounds, State::ACTIVE, )

        //ctx.draw_styled_box("text_box", style::PaletteIndex(0));
        let text_brush = Brush::solid_color(ctx, self.text_color);
        let selected_bg_brush = Brush::solid_color(ctx, self.selection_color);
        let selected_text_brush = Brush::solid_color(ctx, self.selected_text_color);

        ctx.save();
        ctx.transform(&self.content_offset.to_transform());

        // text color
        text_layout.set_drawing_effect(&text_brush, ..);
        if !self.selection.is_empty() {
            // FIXME slightly changes the layout when the selection straddles a kerning pair?
            text_layout.set_drawing_effect(
                &selected_text_brush,
                self.selection.min()..self.selection.max(),
            );
        }

        // selection highlight
        if !self.selection.is_empty() {
            let selected_areas = text_layout
                .hit_test_text_range(self.selection.min()..self.selection.max(), &bounds.origin)
                .unwrap();
            for sa in selected_areas {
                ctx.fill_rectangle(dbg!(sa.bounds.round_out()), &selected_bg_brush);
            }
        }

        // text
        ctx.draw_text_layout(
            Point::origin(),
            text_layout,
            &text_brush,
            DrawTextOptions::ENABLE_COLOR_FONT,
        );

        // caret
        if ctx.is_focused() {
            let caret_hit_test = text_layout
                .hit_test_text_position(self.selection.end)
                .unwrap();

            //dbg!(caret_hit_test);
            ctx.fill_rectangle(
                Rect::new(
                    caret_hit_test.point.floor(),
                    Size::new(1.0, caret_hit_test.metrics.bounds.size.height),
                ),
                &text_brush,
            );
        }

        ctx.restore();
    }

    fn event(&mut self, ctx: &mut EventCtx, children: &mut [Node], event: &Event) {
        match event {
            Event::FocusIn => {
                trace!("focus in");
                ctx.request_redraw();
            }
            Event::FocusOut => {
                trace!("focus out");
                let pos = self.selection.end;
                self.set_cursor(pos);
                ctx.request_redraw();
            }
            Event::Pointer(p) => {
                match p.kind {
                    PointerEventKind::PointerDown => {
                        let pos = self.position_to_text(p.position);
                        if p.repeat_count == 2 {
                            // double-click selects all
                            self.select_all();
                        } else {
                            self.set_cursor(pos);
                        }
                        ctx.request_redraw();
                        ctx.request_focus();
                        ctx.capture_pointer();
                    }
                    PointerEventKind::PointerMove => {
                        // update selection
                        if ctx.is_capturing_pointer() {
                            let pos = self.position_to_text(p.position);
                            self.set_selection_end(pos);
                            trace!(?self.selection, "text selection changed");
                            ctx.request_redraw();
                        }
                    }
                    PointerEventKind::PointerUp => {
                        // nothing to do (pointer grab automatically ends)
                    }
                    _ => {}
                }
            }
            Event::Keyboard(k) => match k.state {
                KeyState::Down => match k.key {
                    keyboard_types::Key::Backspace => {
                        if self.selection.is_empty() {
                            self.move_cursor(Movement::Left, true);
                        }
                        self.insert("");
                        ctx.request_relayout();
                    }
                    keyboard_types::Key::Delete => {
                        if self.selection.is_empty() {
                            self.move_cursor(Movement::Right, true);
                        }
                        self.insert("");
                        ctx.request_relayout();
                    }
                    keyboard_types::Key::ArrowLeft => {
                        self.move_cursor(Movement::Left, k.modifiers.contains(Modifiers::SHIFT));
                        ctx.request_redraw();
                    }
                    keyboard_types::Key::ArrowRight => {
                        self.move_cursor(Movement::Right, k.modifiers.contains(Modifiers::SHIFT));
                        ctx.request_redraw();
                    }
                    keyboard_types::Key::Character(ref c) => {
                        // reject control characters (handle in KeyDown instead)
                        //trace!("insert {:?}", input.character);
                        self.insert(&c);
                        ctx.request_relayout();
                    }
                    _ => {}
                },
                KeyState::Up => {}
            },

            Event::Composition(input) => {}
            _ => {}
        }
    }
}

pub const TEXT_EDIT_BOX_STYLE_SET: EnvKey<Arc<StyleSet>> =
    EnvKey::new("kyute.text-edit.box-style-set");

struct EditState {
    text: String,
    selection: Selection,
}

impl EditState {
    pub fn new(text: String) -> EditState {
        EditState {
            text,
            selection: Default::default()
        }
    }

    pub fn set_text(&mut self, text: String) {
        self.text = text;
        self.selection = Default::default();
    }
}

fn core_line_edit(
    cx: &mut CompositionCtx,
    text: &mut String,
    background_style: Arc<StyleSet>,
    text_format: TextFormat,
    text_color: Color,
    selection_color: Color,
    selected_text_color: Color,
) {
    cx.enter(0);
    cx.with_state(
        || EditState::new(text.clone()),
        |cx, edit_state| {

            edit_state.set_text(text.clone());

            w::container(cx, TEXT_EDIT_BOX_STYLE_SET, |cx| {
                w::core_text(cx, /*text*/ &edit_state.text, /*text format*/ text_format)
                // selection box
                // issue: we want access to post-layout information here, but how?
                // -> modifiers
            });

            let changed_externally = prev_text != text;
            cx.emit_node(
                |cx| {
                    TextEdit::new(
                        text.clone(),
                        text_format.clone(),
                        background_style.clone(),
                        text_color,
                        selection_color,
                        selected_text_color,
                    )
                },
                |cx, text_edit| {
                    let mut needs_relayout = false;
                    if changed_externally {
                        text_edit.set_text(text.clone());
                        needs_relayout = true;
                    } else {
                        *text = text_edit.text.clone()
                    }

                    // it's a bit annoying to update every property by hand
                    // It would be better to just rebuild the whole text edit if it changes.
                    // However, this would erase the current selection.
                    // Instead, compose more:
                    // - render the text with a "text" composable
                    // - store the current selection as state
                    needs_relayout |= text_edit.set_text_format(text_format.clone());
                    needs_relayout |= text_edit.set_background_style(background_style.clone());
                    needs_relayout |= text_edit.set_text_color(text_color);
                    needs_relayout |= text_edit.set_selection_color(selection_color);
                    needs_relayout |= text_edit.set_selected_text_color(selected_text_color);
                },
                |_| {},
            );
        },
    );
    cx.exit();
}

/// Text editor line.
pub fn text_line_edit(cx: &mut CompositionCtx, text: &mut String) {
    text_line_edit_impl(
        cx,
        text,
        cx.get_env(&TEXT_EDIT_BOX_STYLE_SET),
        cx.get_env(&theme::DEFAULT_TEXT_FORMAT),
        cx.get_env(&theme::TEXT_COLOR),
        cx.get_env(&theme::SELECTED_TEXT_BACKGROUND_COLOR),
        cx.get_env(&theme::SELECTED_TEXT_COLOR),
    )
}
