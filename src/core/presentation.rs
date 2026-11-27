use std::time::Duration;

use smithay::{
    desktop::{layer_map_for_output, utils::OutputPresentationFeedback},
    output::Output,
    reexports::wayland_protocols::wp::presentation_time::server::wp_presentation_feedback::Kind,
};

use crate::core::compositor::HazelCompositor;

impl HazelCompositor {
    pub fn take_presentation_feedback(&self, output: &Output) -> OutputPresentationFeedback {
        let mut feedback = OutputPresentationFeedback::new(output);
        for window in self.windows() {
            if self.output_of_window(window).as_ref() == Some(output) {
                window.take_presentation_feedback(
                    &mut feedback,
                    |_, _| Some(output.clone()),
                    |_, _| Kind::Vsync,
                );
            }
        }
        for layer in layer_map_for_output(output).layers() {
            layer.take_presentation_feedback(
                &mut feedback,
                |_, _| Some(output.clone()),
                |_, _| Kind::Vsync,
            );
        }
        feedback
    }

    pub fn send_presentation_feedback(&mut self, mut feedback: OutputPresentationFeedback, refresh: Duration) {
        self.presentation_sequence += 1;
        feedback.presented(
            self.presentation_clock.now(),
            smithay::wayland::presentation::Refresh::fixed(refresh),
            self.presentation_sequence,
            Kind::Vsync,
        );
    }
}
