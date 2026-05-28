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
    reexports::{calloop::EventLoop, winit::window::WindowAttributes},
    utils::{Rectangle, Transform},
};

use crate::Hazel;
use std::sync::{Arc, Mutex};
use crate::lua::runtime::LuaRuntime;

pub fn init_winit(
    event_loop: &mut EventLoop<Hazel>,
    state: &mut Hazel,
    lua_runtime: Arc<Mutex<LuaRuntime>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (mut backend, winit) = winit::init_from_attributes(WindowAttributes::default().with_title("Hazel"))?;

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

    state.space.map_output(&output, (0, 0));

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
                    use smithay::backend::input::InputEvent;

                    match &event {
                        InputEvent::Keyboard { event, .. } => {
                            let serial = smithay::utils::SERIAL_COUNTER.next_serial();
                            let time = smithay::backend::input::Event::time_msec(event);
                            // Emit to Lua runtime immediately
                            if let Ok(mut rt) = lua_runtime.lock() {
                                let mut apply_layout = |layout: &str| state.set_keyboard_layout_now(layout);
                                rt.emit_keypress(event.key_code().raw(), serial.into(), time, &mut apply_layout);
                            }
                        }
                        _ => {}
                    }

                    state.process_input_event(event)
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
                            [&state.space],
                            &[],
                            &mut damage_tracker,
                            [0.1, 0.1, 0.1, 1.0],
                        )
                        .unwrap();
                    }
                    backend.submit(Some(&[damage])).unwrap();

                    state.space.elements().for_each(|window| {
                        window.send_frame(
                            &output,
                            state.start_time.elapsed(),
                            Some(Duration::ZERO),
                            |_, _| Some(output.clone()),
                        )
                    });

                    state.space.refresh();
                    state.smithay.popups.cleanup();
                    let _ = state.display_handle.flush_clients();

                    // Ask for redraw to schedule new frame.
                    backend.window().request_redraw();
                }
                WinitEvent::CloseRequested => {
                    state.loop_signal.stop();
                }
                _ => (),
            };
        })?;

    Ok(())
}
