use smithay::wayland::output::OutputHandler;

use crate::core::Hazel;

impl OutputHandler for Hazel {
	fn output_bound(&mut self, _output: smithay::output::Output, _wl_output: smithay::reexports::wayland_server::protocol::wl_output::WlOutput) {
		println!("Output bound: {}", _output.name());
	}
}

smithay::delegate_output!(Hazel);
