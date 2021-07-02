use crate::{layout::{BoxConstraints, Measurements}, Size, Rect, Offset};
use tracing::trace;
use crate::widget::{Widget, LayoutCtx, Node};
use crate::node::{NodeRef, PaintCtx};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Axis {
    Horizontal,
    Vertical,
}

impl Axis {
    pub fn cross_axis(self) -> Axis {
        match self {
            Axis::Horizontal => Axis::Vertical,
            Axis::Vertical => Axis::Horizontal,
        }
    }

    pub fn main_len(self, size: Size) -> f64 {
        match self {
            Axis::Vertical => size.height,
            Axis::Horizontal => size.width,
        }
    }

    pub fn cross_len(self, size: Size) -> f64 {
        match self {
            Axis::Vertical => size.width,
            Axis::Horizontal => size.height,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum MainAxisAlignment {
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceEvenly,
    SpaceAround,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum CrossAxisAlignment {
    Baseline,
    Start,
    Center,
    End,
    Stretch,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum MainAxisSize {
    Min,
    Max,
}

pub struct Flex {
    axis: Axis,
}

impl Widget for Flex {
    fn layout(&mut self, ctx: &mut LayoutCtx, children: &mut [Node], constraints: &BoxConstraints) -> Measurements {
        let child_measurements: Vec<Measurements> = children.iter_mut().map(|n| n.layout(ctx, constraints)).collect();

        let max_cross_axis_len = child_measurements
            .iter()
            .map(|m| self.axis.cross_len(m.size))
            .fold(0.0, f64::max);

        // preferred size of this flex: max size in axis direction, max elem width in cross-axis direction
        let cross_axis_len = match self.axis {
            Axis::Vertical => constraints.constrain_width(max_cross_axis_len),
            Axis::Horizontal => constraints.constrain_height(max_cross_axis_len),
        };

        // distribute children
        let mut d = 0.0;
        //let spacing = env.get(theme::FlexSpacing);
        let spacing = 1.0;

        for i in 0..child_measurements.len() {
            let child = &mut children[i];
            let measurement = &child_measurements[i];
            let len = self.axis.main_len(m.size);
            // offset children
            let offset = match self.axis {
                Axis::Vertical => Offset::new(0.0, d),
                Axis::Horizontal => Offset::new(d, 0.0),
            };
            child.set_offset(offset);
            d += len + spacing;
            d = d.ceil();
            trace!("flex pos={}", d);
        }

        let size = match axis {
            Axis::Vertical => Size::new(cross_axis_len, constraints.constrain_height(d)),
            Axis::Horizontal => Size::new(constraints.constrain_width(d), cross_axis_len),
        };

        Measurements::new(size)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, children: &mut [Node], bounds: Rect) {
        for c in children.iter_mut() {
            c.paint(ctx, bounds);
        }
    }
}

impl Flex {
    pub fn new(main_axis: Axis) -> Self {
        Flex {
            axis: main_axis,
        }
    }
}
