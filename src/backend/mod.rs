use std::time::Duration;

use crate::err::IntoDiagnostic;
use miette::Result;
use smithay::{
    backend::{
        renderer::{
            damage::OutputDamageTracker, element::surface::WaylandSurfaceRenderElement,
            gles::GlesRenderer,
        },
        winit::{self, WinitEvent},
    },
    output::{Mode, Output, PhysicalProperties, Subpixel},
    reexports::winit::window::WindowAttributes,
    utils::{Rectangle, Transform},
};

use crate::core::{GlobalHazel, Hazel, HazelEventLoop};

pub enum Backend {
    Winit {
        backend: winit::WinitGraphicsBackend<GlesRenderer>,
        winit: winit::WinitEventLoop,
    },
}

impl Backend {
    pub fn new_winit() -> Self {
        let (backend, winit) =
            winit::init_from_attributes(WindowAttributes::default().with_title("Hazel")).unwrap();
        Self::Winit { backend, winit }
    }

    pub fn initialize(
        self,
        state: &mut Hazel,
        event_loop: &mut HazelEventLoop,
    ) -> Result<()> {
        match self {
            Backend::Winit { mut backend, winit } => {
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
                    },
                );

                let _global = output.create_global::<Hazel>(&state.display_handle);
                output.change_current_state(
                    Some(mode),
                    Some(Transform::Flipped180),
                    None,
                    Some((0, 0).into()),
                );
                output.set_preferred(mode);

                state.compositor.space.map_output(&output, (0, 0));

                let mut damage_tracker = OutputDamageTracker::from_output(&output);
                event_loop
                    .handle()
                    .insert_source(winit, move |event, _, state| {
                        match event {
                            WinitEvent::Resized { size, .. } => {
                                output.change_current_state(
                                    Some(Mode {
                                        size,
                                        refresh: 60_000,
                                    }),
                                    None,
                                    None,
                                    None,
                                );
                            }
                            WinitEvent::Input(event) => {
                                if let Err(e) =
                                    GlobalHazel::execute(state, |hazel| hazel.process_input(event))
                                {
                                    eprintln!("Error processing input event");
                                    eprintln!("{e:?}");
                                }
                            }
                            WinitEvent::Redraw => {
                                let size = backend.window_size();
                                let damage = Rectangle::from_size(size);

                                {
                                    let (renderer, mut framebuffer) = backend.bind().unwrap();
                                    smithay::desktop::space::render_output::<
                                        _,
                                        WaylandSurfaceRenderElement<GlesRenderer>,
                                        _,
                                        _,
                                    >(
                                        &output,
                                        renderer,
                                        &mut framebuffer,
                                        1.0,
                                        0,
                                        [&state.compositor.space],
                                        &[],
                                        &mut damage_tracker,
                                        [0.1, 0.1, 0.1, 1.0],
                                    )
                                    .unwrap();
                                }
                                backend.submit(Some(&[damage])).unwrap();

                                state.compositor.space.elements().for_each(|window| {
                                    window.send_frame(
                                        &output,
                                        state.start_time.elapsed(),
                                        Some(Duration::ZERO),
                                        |_, _| Some(output.clone()),
                                    )
                                });

                                state.compositor.space.refresh();
                                state.compositor.smithay.popups.cleanup();
                                let _ = state.display_handle.flush_clients();

                                // Ask for redraw to schedule new frame.
                                backend.window().request_redraw();
                            }
                            WinitEvent::CloseRequested => {
                                state.loop_signal.stop();
                            }
                            _ => (),
                        };
                    }).into_diagnostic()?;

                Ok(())
            }
        }
    }
}
