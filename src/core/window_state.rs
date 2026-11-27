use crate::core::shell_window::ShellWindow;
use smithay::{
    desktop::{Window, layer_map_for_output},
    output::Output,
    reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::State,
    utils::{Logical, Rectangle},
};

use crate::core::compositor::HazelCompositor;

impl HazelCompositor {
    pub fn output_of_window(&self, window: &Window) -> Option<Output> {
        let area = Rectangle::new(self.window_location(window), window.geometry().size);
        let mut best: Option<(i64, &Output)> = None;
        for output in self.space.outputs() {
            let Some(overlap) = self.space.output_geometry(output).and_then(|geometry| geometry.intersection(area)) else {
                continue;
            };
            let covered = i64::from(overlap.size.w) * i64::from(overlap.size.h);
            if best.is_none_or(|(most, _)| covered > most) {
                best = Some((covered, output));
            }
        }
        best.map(|(_, output)| output.clone())
            .or_else(|| self.space.output_under(area.loc.to_f64()).next().cloned())
            .or_else(|| self.space.outputs().next().cloned())
    }

    pub fn usable_area(&self, output: &Output) -> Option<Rectangle<i32, Logical>> {
        let output_geometry = self.space.output_geometry(output)?;
        let mut zone = layer_map_for_output(output).non_exclusive_zone();
        zone.loc += output_geometry.loc;
        Some(zone)
    }

    pub fn window_has_state(&self, window: &Window, state: State) -> bool {
        window.has_shell_state(state)
    }
}
