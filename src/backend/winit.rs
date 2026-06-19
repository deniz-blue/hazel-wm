use std::time::Duration;

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
use tracing::{error, info};
use std::error::Error as StdError;

use crate::core::{GlobalHazel, HazelEventLoop};

pub struct HazelWinitBackend {
    pub output: Output,
}

impl HazelWinitBackend {
    pub fn new(event_loop: &mut HazelEventLoop) -> std::result::Result<Self, Box<dyn StdError>> {
        let (mut backend, winit) =
            winit::init_from_attributes(WindowAttributes::default().with_title("Hazel"))?;

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

        output.change_current_state(
            Some(mode),
            Some(Transform::Flipped180),
            None,
            Some((0, 0).into()),
        );

        output.set_preferred(mode);

        let mut damage_tracker = OutputDamageTracker::from_output(&output);
        let cloned_output = output.clone();
        event_loop
            .handle()
            .insert_source(winit, move |event, _, state| {
                match event {
                    WinitEvent::Resized { size, .. } => {
                        cloned_output.change_current_state(
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
                            error!("Error processing input event");
                            error!("{e:?}");
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
                                &cloned_output,
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
                                &cloned_output,
                                state.start_time.elapsed(),
                                Some(Duration::ZERO),
                                |_, _| Some(cloned_output.clone()),
                            )
                        });

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

        Ok(Self { output })
    }
}
