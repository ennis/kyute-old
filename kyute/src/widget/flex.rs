use crate::{
    core2::{LayoutCtx, PaintCtx, WidgetState},
    BoxConstraints, Environment, Event, EventCtx, LayoutItem, Measurements, Offset, Rect, Size,
    Widget, WidgetDelegate,
};
use tracing::trace;

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
    items: Vec<Widget>,
}

impl Flex {
    pub fn new(axis: Axis, items: Vec<Widget>) -> Flex {
        Flex { axis, items }
    }

    pub fn push<T: WidgetDelegate + 'static>(&mut self, item: Widget<T>) {
        self.items.push(item.into())
    }
}

impl WidgetDelegate for Flex {
    fn layout(
        &self,
        ctx: &mut LayoutCtx,
        constraints: BoxConstraints,
        env: &Environment,
    ) -> LayoutItem {
        let item_layouts: Vec<LayoutItem> = self
            .items
            .iter()
            .map(|item| item.layout(ctx, constraints, env))
            .collect();

        let max_cross_axis_len = item_layouts
            .iter()
            .map(|l| self.axis.cross_len(l.size()))
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

        let size = match self.axis {
            Axis::Vertical => Size::new(cross_axis_len, constraints.constrain_height(d)),
            Axis::Horizontal => Size::new(constraints.constrain_width(d), cross_axis_len),
        };

        let mut layout = LayoutItem::new(Measurements::new(size));

        for item_layout in item_layouts.iter() {
            let len = self.axis.main_len(layout.size());
            let offset = match self.axis {
                Axis::Vertical => Offset::new(0.0, d),
                Axis::Horizontal => Offset::new(d, 0.0),
            };
            layout.add_child(offset, item_layout.clone());
            d += len + spacing;
            d = d.ceil();
        }

        layout
    }

    fn paint(&self, ctx: &mut PaintCtx, bounds: Rect, env: &Environment) {
        todo!()
    }
}
