use smithay::{
    output::Output,
    utils::{Logical, Rectangle},
};

pub enum PointerAbsoluteMapping {
    /// Map into the first output containing the pointer, or the current output if none do.
    FirstContainingOutput,
    /// Map into a specific output
    Output(Output),
    /// Map into a specific area of the space
    Space(Rectangle<i32, Logical>),
}
