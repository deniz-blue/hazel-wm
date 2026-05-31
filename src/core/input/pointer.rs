use smithay::{output::Output, utils::{Logical, Size}};

pub enum PointerAbsoluteMapping {
	/// Map into the first output containing the pointer, or the current output if none do.
    FirstContainingOutput,
	/// Map into a specific output
	Output(Output),
	/// Map into a specific area of the space
    Space(Size<i32, Logical>),
}
