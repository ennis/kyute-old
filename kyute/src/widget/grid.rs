use crate::style::Length;
use crate::widget::Widget;
use crate::node::{NodeRef, PaintCtx};
use crate::{Rect, Size};
use crate::layout::Measurements;

pub enum GridLength {
    /// Size relative to other rows or columns
    Relative(f64),
    /// Absolute row/col size
    Absolute(Length),
    /// Size relative to contents
    SizeToContents
}

pub struct Grid {
    rows: Vec<GridLength>,
    columns: Vec<GridLength>,
}

impl Widget for Grid {
    fn layout(&self, this_node: NodeRef, available_size: Size) -> Measurements {
        todo!()
    }

    fn render(&self, this_node: NodeRef, paint_ctx: &PaintCtx, bounds: Rect) {
        todo!()
    }
}