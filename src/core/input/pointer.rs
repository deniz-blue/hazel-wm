use smithay::{
    input::pointer::CursorImageStatus,
    output::Output,
    utils::{Logical, Point, Rectangle},
};

pub struct PointerState {
    /// Currently pressed buttons.
    pub pressed: Vec<u32>,
    /// Absolute-positioning mapping.
    pub mapping: PointerAbsoluteMapping,
    /// Previous absolute position for delta computation.
    pub absolute_previous: Point<f64, Logical>,
    /// Cursor image (surface or named icon).
    pub cursor_status: CursorImageStatus,
}

impl Default for PointerState {
    fn default() -> Self {
        Self::new()
    }
}

impl PointerState {
    pub fn new() -> Self {
        Self {
            pressed: Vec::new(),
            mapping: PointerAbsoluteMapping::FirstContainingOutput,
            absolute_previous: (0.0, 0.0).into(),
            cursor_status: CursorImageStatus::default_named(),
        }
    }
}

pub enum PointerAbsoluteMapping {
    /// Map into the first output containing the pointer, or the current output if none do.
    FirstContainingOutput,
    /// Map into a specific output
    Output(Output),
    /// Map into a specific area of the space
    Space(Rectangle<i32, Logical>),
}
