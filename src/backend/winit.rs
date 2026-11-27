use smithay::{
    backend::{
        renderer::{
            damage::OutputDamageTracker,
            Bind, ImportDma, Renderer,
            gles::GlesRenderer,
        },
        allocator::dmabuf::Dmabuf,
        egl::EGLDevice,
        winit::{self, WinitEvent},
    },
    output::{Mode, Output, PhysicalProperties, Subpixel},
    reexports::{
        winit::{dpi::PhysicalSize, window::WindowAttributes},
        wayland_server::DisplayHandle,
    },
    wayland::dmabuf::{DmabufFeedbackBuilder, DmabufState},
    utils::{Physical, Size, Transform},
};
use std::{cell::Cell, error::Error as StdError, rc::Rc};
use tracing::{error, info};

use crate::{
    backend::{ModeChange, winit_capture::serve_capture_requests},
    core::{GlobalHazel, Hazel, HazelEventLoop, capture::target::DmabufCaptureSupport},
};

pub(super) const CLEAR_COLOR: [f32; 4] = [0.1, 0.1, 0.1, 1.0];
const FRAME_INTERVAL: std::time::Duration = std::time::Duration::from_micros(16_667);

/// The window framebuffer has its origin at the bottom left, so what the output shows is flipped on top of the output transform.
fn framebuffer_transform(output: &Output) -> Transform {
    flipped_vertically_after(output.current_transform())
}

/// `Transform + Transform` only adds degrees and xors the flip, which is not a composition for the 90 and 270 variants.
fn flipped_vertically_after(transform: Transform) -> Transform {
    match transform {
        Transform::Normal => Transform::Flipped180,
        Transform::_90 => Transform::Flipped90,
        Transform::_180 => Transform::Flipped,
        Transform::_270 => Transform::Flipped270,
        Transform::Flipped => Transform::_180,
        Transform::Flipped90 => Transform::_90,
        Transform::Flipped180 => Transform::Normal,
        Transform::Flipped270 => Transform::_270,
    }
}

fn framebuffer_tracker(output: &Output, size: Size<i32, Physical>) -> OutputDamageTracker {
    OutputDamageTracker::new(size, output.current_scale().fractional_scale(), framebuffer_transform(output))
}

#[derive(Default)]
struct WindowTracker {
    current: Option<(Size<i32, Physical>, u64, Transform, OutputDamageTracker)>,
}

impl WindowTracker {
    fn for_output(&mut self, output: &Output, size: Size<i32, Physical>) -> &mut OutputDamageTracker {
        let scale = output.current_scale().fractional_scale().to_bits();
        let transform = framebuffer_transform(output);
        let current = match self.current.take() {
            Some(current) if current.0 == size && current.1 == scale && current.2 == transform => current,
            _ => (size, scale, transform, framebuffer_tracker(output, size)),
        };
        &mut self.current.insert(current).3
    }
}

fn adopt_window_size(state: &mut Hazel, output: &Output, size: Size<i32, Physical>) {
    if output.current_mode().is_some_and(|mode| mode.size == size) {
        return;
    }
    if let Some(previous) = output.current_mode() {
        output.delete_mode(previous);
    }
    let mode = Mode { size, refresh: 60_000 };
    output.change_current_state(Some(mode), None, None, None);
    output.set_preferred(mode);
    GlobalHazel::execute(state, |hazel| hazel.output_geometry_changed(output));
}

fn advertise_dmabuf(
    display_handle: &DisplayHandle,
    dmabuf_state: &mut DmabufState,
    renderer: &mut GlesRenderer,
) -> Option<DmabufCaptureSupport> {
    let formats = renderer.dmabuf_formats();
    let render_node = EGLDevice::device_for_display(&renderer.egl_context().display())
        .ok()
        .and_then(|device| device.try_get_render_node().ok().flatten());
    let support = render_node.as_ref().zip(Bind::<Dmabuf>::supported_formats(renderer)).map(|(node, formats)| {
        DmabufCaptureSupport { device: node.dev_id(), formats }
    });
    let feedback = render_node.and_then(|node| {
        DmabufFeedbackBuilder::new(node.dev_id(), formats.iter().copied())
            .build()
            .ok()
    });
    match feedback {
        Some(feedback) => {
            dmabuf_state.create_global_with_default_feedback::<Hazel>(display_handle, &feedback);
        }
        None => {
            dmabuf_state.create_global::<Hazel>(display_handle, formats.iter().copied());
        }
    }
    support
}

#[derive(Debug)]
pub struct HazelWinitBackend {
    pub output: Output,
    pub dmabuf_capture: Option<DmabufCaptureSupport>,
    requested_size: Rc<Cell<Option<Size<i32, Physical>>>>,
}

impl HazelWinitBackend {
    pub fn new(
        display_handle: &DisplayHandle,
        dmabuf_state: &mut DmabufState,
        event_loop: &mut HazelEventLoop,
    ) -> std::result::Result<Self, Box<dyn StdError>> {
        let (mut backend, winit) =
            winit::init_from_attributes(WindowAttributes::default().with_title("Hazel"))?;

        let dmabuf_capture = advertise_dmabuf(display_handle, dmabuf_state, backend.renderer());

        let mode = Mode {
            size: backend.window_size(),
            refresh: 60_000,
        };

        let output = Output::new(
            "winit".to_string(),
            PhysicalProperties {
                size: (0, 0).into(),
                subpixel: Subpixel::Unknown,
                make: "Smithay".into(),
                model: "Winit".into(),
                serial_number: "Unknown".into(),
            },
        );

        output.change_current_state(Some(mode), Some(Transform::Normal), None, Some((0, 0).into()));

        output.set_preferred(mode);

        let mut window_tracker = WindowTracker::default();
        let mut next_buffer_age = 0;
        let requested_size: Rc<Cell<Option<Size<i32, Physical>>>> = Rc::new(Cell::new(None));
        let pending_size = requested_size.clone();
        let cloned_output = output.clone();
        event_loop
            .handle()
            .insert_source(winit, move |event, _, state| {
                match event {
                    WinitEvent::Resized { size, .. } => {
                        adopt_window_size(state, &cloned_output, size);
                    }
                    WinitEvent::Input(event) => {
                        if let Err(e) =
                            GlobalHazel::execute(state, |hazel| hazel.process_input(event))
                        {
                            error!("Error processing input event");
                            error!("{e:?}");
                        }
                    }
                    WinitEvent::Redraw => {
                        state.answer_dmabuf_imports(backend.renderer());
                        if let Some(size) = pending_size.take() {
                            let answer = backend.window().request_surface_size(PhysicalSize::new(size.w as u32, size.h as u32).into());
                            info!("Asked the window for {}x{}: {answer:?}", size.w, size.h);
                        }
                        let size = backend.window_size();
                        adopt_window_size(state, &cloned_output, size);
                        let age = next_buffer_age;

                        let feedback = state.compositor.take_presentation_feedback(&cloned_output);
                        let damage = match backend.bind() {
                            Ok((renderer, mut framebuffer)) => {
                                let elements = state
                                    .compositor
                                    .output_elements(renderer, &cloned_output, false);
                                match window_tracker.for_output(&cloned_output, size).render_output(
                                    renderer,
                                    &mut framebuffer,
                                    age,
                                    &elements,
                                    CLEAR_COLOR,
                                ) {
                                    Ok(result) => result.damage.cloned(),
                                    Err(e) => {
                                        error!("Rendering failed: {e}");
                                        None
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Could not bind the window framebuffer: {e}");
                                None
                            }
                        };
                        if let Some(damage) = damage {
                            match backend.submit(Some(&damage)) {
                                Ok(()) => state
                                    .compositor
                                    .send_presentation_feedback(feedback, FRAME_INTERVAL),
                                Err(e) => error!("Could not submit the frame: {e}"),
                            }
                            next_buffer_age = backend.buffer_age().unwrap_or(0);
                        }
                        serve_capture_requests(state, backend.renderer(), &cloned_output, size, true);
                        for output in state.compositor.virtual_outputs() {
                            let size = output.current_mode().map(|mode| mode.size).unwrap_or_default();
                            serve_capture_requests(state, backend.renderer(), &output, size, false);
                            state.compositor.send_layer_frame_callbacks(&output, state.start_time.elapsed());
                        }
                        if let Err(e) = backend.renderer().cleanup_texture_cache() {
                            error!("Could not clean up the texture cache: {e}");
                        }

                        state
                            .compositor
                            .send_frame_callbacks(&cloned_output, state.start_time.elapsed());

                        backend.window().request_redraw();
                    }
                    WinitEvent::CloseRequested => {
                        state.loop_signal.stop();
                    }
                    _ => (),
                };
            })
            .map_err(|e| Box::new(e) as Box<dyn StdError>)?;

        info!("Initialized Winit backend");

        Ok(Self { output, dmabuf_capture, requested_size })
    }

    pub fn request_mode(&mut self, mode: Mode) -> Result<ModeChange, String> {
        let size = mode.size;
        if !(1..=16384).contains(&size.w) || !(1..=16384).contains(&size.h) {
            return Err(format!("{}x{} is not a window size", size.w, size.h));
        }
        self.requested_size.set(Some(size));
        Ok(ModeChange::Requested)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Transform; 8] = [
        Transform::Normal,
        Transform::_90,
        Transform::_180,
        Transform::_270,
        Transform::Flipped,
        Transform::Flipped90,
        Transform::Flipped180,
        Transform::Flipped270,
    ];

    #[test]
    fn flipping_twice_gives_the_transform_back() {
        for transform in ALL {
            assert_eq!(flipped_vertically_after(flipped_vertically_after(transform)), transform);
        }
    }

    #[test]
    fn an_unrotated_output_is_just_flipped() {
        assert_eq!(flipped_vertically_after(Transform::Normal), Transform::Flipped180);
        assert_eq!(flipped_vertically_after(Transform::Flipped180), Transform::Normal);
    }

    #[test]
    fn a_half_turn_is_a_mirror_and_back() {
        assert_eq!(flipped_vertically_after(Transform::_180), Transform::Flipped);
        assert_eq!(flipped_vertically_after(Transform::Flipped), Transform::_180);
    }
}
