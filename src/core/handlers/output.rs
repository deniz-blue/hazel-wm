use smithay::wayland::output::OutputHandler;

use crate::core::Hazel;

impl OutputHandler for Hazel {}
smithay::delegate_output!(Hazel);
