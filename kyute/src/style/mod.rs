//! Drawing code for GUI elements.
use crate::Rect;

/// Unit of length: device-independent pixel.
pub struct Dip;

/// A length in DIPs.
pub type DipLength = euclid::Length<f64, Dip>;
pub type Angle = euclid::Angle<f64>;

/// Length specification.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Length {
    /// Actual screen pixels (the actual physical size depends on the density of the screen).
    Px(f64),
    /// Device-independent pixels (DIPs), close to 1/96th of an inch.
    Dip(f64),
    /// Inches (logical inches? approximate inches?).
    In(f64),
}


/*
fn rect_to_sk(rect: Rect) -> sk::Rect {
    sk::Rect::new(rect.min_x() as f32, rect.min_y() as f32, rect.max_x() as f32, rect.max_y() as f32)
}

const BUTTON_COLOR: sk::Color4f = sk::Color4f::new(0.1, 0.1, 0.2, 1.0);

pub fn draw_button(canvas: &mut sk::Canvas, bounds: Rect, label: &str) {
    let paint = sk::Paint::new(BUTTON_COLOR, None);

    canvas.draw_rect(
        rect_to_sk(bounds),
        &paint
    );
}*/