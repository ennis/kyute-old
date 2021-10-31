//! Sliders provide a way to make a value vary linearly between two bounds by dragging a knob along
//! a line.
use crate::{
    event::{Event, LifecycleEvent, PointerEventKind},
    style::State,
    theme, BoxConstraints, Environment, EventCtx, LayoutCtx, Measurements, PaintCtx, Point, Rect,
    SideOffsets, Size, Widget,
};

// TODO just pass f64 directly as the action?
#[derive(Copy, Clone, Debug)]
enum SliderAction {
    ValueChanged(f64),
}

/// Utility class representing a slider track on which a knob can move.
struct SliderTrack {
    start: Point,
    end: Point,
}

impl SliderTrack {
    fn new(start: Point, end: Point) -> SliderTrack {
        SliderTrack { start, end }
    }

    /// Returns the value that would be set if the cursor was at the given position.
    fn value_from_position(&self, pos: Point, min: f64, max: f64) -> f64 {
        /*let hkw = 0.5 * get_knob_width(track_width, divisions, min_knob_width);
        // at the end of the sliders, there are two "dead zones" of width kw / 2 that
        // put the slider all the way left or right
        let x = pos.x.max(hkw).min(track_width-hkw-1.0);*/

        // project the point on the track line
        let v = self.end - self.start;
        let c = pos - self.start;
        let x = v.normalize().dot(c);
        let track_len = v.length();
        (min + (max - min) * x / track_len).clamp(min, max)
    }

    /// Returns the position of the knob on the track.
    fn knob_position(&self, value: f64) -> Point {
        self.start + (self.end - self.start) * value
    }
}

impl Default for SliderTrack {
    fn default() -> Self {
        SliderTrack {
            start: Default::default(),
            end: Default::default(),
        }
    }
}

/*fn draw_slider_knob(
    ctx: &mut PaintCtx,
    size: Size,
    pos: f64,
    divisions: Option<u32>,
    theme: &Theme,
) {
    // half the height
    let min_knob_w = (0.5 * theme.button_metrics.min_height).ceil();
    let knob_w = get_knob_width(size.width, divisions, min_knob_w);

    let off = ((w - knob_w) * pos).ceil();
    let knob = Rect::new(Point::new(off, 0.0), Size::new(knob_w, h));

    // draw the knob rectangle
    let knob_brush = DEFAULT_COLORS.slider_grab.into_brush();
    ctx.fill_rectangle(knob, &knob_brush);
}*/

pub struct Slider {
    track: SliderTrack,
    value: f64,
    min: f64,
    max: f64,
}

impl Slider {
    pub fn new(min: f64, max: f64, value: f64) -> Slider {
        Slider {
            // endpoints calculated during layout
            track: Default::default(),
            value: value.clamp(min, max),
            min,
            max,
        }
    }

    fn update_value(&mut self, cursor: Point) {
        self.value = self.track.value_from_position(cursor, self.min, self.max);
    }

    /// Returns the current value, normalized between 0 and 1.
    fn value_norm(&self) -> f64 {
        (self.value - self.min) / (self.max - self.min)
    }

    fn set_value(&mut self, value: f64) {
        self.value = value;
    }
}

impl Widget<f64> for Slider {
    fn debug_name(&self) -> &str {
        std::any::type_name::<Self>()
    }

    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut f64) -> Option<f64> {
        match event {
            Event::Pointer(p) => match p.kind {
                PointerEventKind::PointerOver | PointerEventKind::PointerOut => {
                    ctx.request_redraw();
                    None
                }
                PointerEventKind::PointerDown => {
                    let new_value = self
                        .track
                        .value_from_position(p.position, self.min, self.max);
                    *data = new_value;
                    ctx.capture_pointer();
                    ctx.request_focus();
                    Some(new_value)
                }
                PointerEventKind::PointerMove => {
                    if ctx.is_capturing_pointer() {
                        let new_value = self
                            .track
                            .value_from_position(p.position, self.min, self.max);
                        *data = new_value;
                        Some(new_value)
                    } else {
                        None
                    }
                }
                _ => None,
            },
            _ => None,
        }
    }

    fn lifecycle(&mut self, _ctx: &mut EventCtx, _lifecycle_event: &LifecycleEvent, _data: &f64) {
        // nothing
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        constraints: BoxConstraints,
        data: &f64,
        env: &Environment,
    ) -> Measurements {
        let height = 14.0; //env.get(theme::SliderHeight);
        let knob_width = 11.0; //env.get(theme::SliderKnobWidth);
        let knob_height = 11.0; //env.get(theme::SliderKnobHeight);
        let padding = SideOffsets::new_all_same(0.0);

        // fixed height
        let size = Size::new(
            constraints.max_width(),
            constraints.constrain_height(height),
        );

        // position the slider track inside the layout
        let inner_bounds = Rect::new(Point::origin(), size).inner_rect(padding);

        // calculate knob width
        //let knob_width = get_knob_width(inner_bounds.size.width, self.divisions, min_knob_width);
        // half knob width
        let hkw = 0.5 * knob_width;
        // y-position of the slider track
        let y = 0.5 * size.height;

        // center vertically, add some padding on the sides to account for padding and half-knob size
        self.track.start = Point::new(inner_bounds.min_x() + hkw, y);
        self.track.end = Point::new(inner_bounds.max_x() - hkw, y);

        Measurements {
            size,
            baseline: None,
        }
    }

    fn paint(&self, ctx: &mut PaintCtx, bounds: Rect, env: &Environment) {
        let track_y = env.get(theme::SLIDER_TRACK_Y).unwrap_or_default();
        let track_h = env.get(theme::SLIDER_TRACK_HEIGHT).unwrap_or_default();
        let knob_w = env.get(theme::SLIDER_KNOB_WIDTH).unwrap_or_default();
        let knob_h = env.get(theme::SLIDER_KNOB_HEIGHT).unwrap_or_default();
        let knob_y = env.get(theme::SLIDER_KNOB_Y).unwrap_or_default();
        let track_style = env.get(theme::SLIDER_TRACK_STYLE).unwrap();
        let knob_style = env.get(theme::SLIDER_KNOB_STYLE).unwrap();

        let track_x_start = self.track.start.x;
        let track_x_end = self.track.end.x;

        // track bounds
        let track_bounds = Rect::new(
            Point::new(track_x_start, track_y - 0.5 * track_h),
            Size::new(track_x_end - track_x_start, track_h),
        );

        let kpos = self.track.knob_position(self.value_norm());
        let kx = kpos.x.round() + 0.5;

        let knob_bounds = Rect::new(
            Point::new(kx - 0.5 * knob_w, track_y - knob_y),
            Size::new(knob_w, knob_h),
        );

        // track
        track_style.draw_box(ctx, &track_bounds, State::empty());
        knob_style.draw_box(ctx, &knob_bounds, State::empty());
    }
}
