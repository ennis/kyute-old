//! built-in widgets.
mod button;
mod flex;
mod grid;
mod text;
mod window;
mod slider;
mod container;
//mod textedit;

pub use button::{button, ButtonAction};
pub use flex::{Axis, CrossAxisAlignment, Flex, MainAxisAlignment, MainAxisSize, vbox, hbox, flex};
pub use window::window;
pub use slider::{SliderTrack,Slider,slider};
pub use container::{container};

use crate::CompositionCtx;
use crate::style::StyleSet;
use std::sync::Arc;


