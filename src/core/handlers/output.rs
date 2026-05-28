use smithay::wayland::output::OutputHandler;

use crate::core::{Hazel, compositor::HazelCompositor};

impl OutputHandler for Hazel {}
smithay::delegate_output!(Hazel);
