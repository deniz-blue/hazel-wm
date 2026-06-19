#![allow(irrefutable_let_patterns)]

pub mod backend;
pub mod core;
pub mod lua;

use smithay::reexports::calloop::EventLoop;
use tracing::{error, info};

use crate::core::{GlobalHazel, Hazel, HazelEventLoop};
use crate::lua::export_typedefs::dump_typedefs;

fn main() {
    if std::env::args().any(|arg| arg == "--dump-typedefs") {
        dump_typedefs();
        return;
    }

    init_logging();

    info!("Starting Hazel...");
    if let Err(e) = run() {
        error!("{e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut event_loop: HazelEventLoop =
        EventLoop::try_new().map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    let mut state = Hazel::new(&mut event_loop)?;

    // Safety: single threaded
    unsafe { std::env::set_var("WAYLAND_DISPLAY", &state.compositor.socket_name) };

    GlobalHazel::execute(&mut state, |hazel| {
        if let Err(e) = hazel.lua.init() {
            return error!("Error initializing Lua: {e}");
        }

        hazel
            .wm()
            .events
            .emit("ready".to_owned(), ())
            .expect("Failed to emit ready event");

        let outputs = hazel.compositor.space.outputs();
        for _output in outputs {
            hazel
                .wm()
                .outputs
                .events
                .emit("added".to_owned(), Option::<bool>::None)
                .expect("Failed to emit output added event");
        }

        info!("Initialized Lua");
    });

    event_loop
        .run(None, &mut state, move |hazel| {
            GlobalHazel::execute(hazel, |hazel| {
                hazel
                    .wm()
                    .events
                    .emit("tick".to_owned(), ())
                    .expect("Failed to emit tick event");
            });

            hazel.compositor.space.refresh();
            hazel.compositor.smithay.popups.cleanup();
            let _ = hazel.display_handle.flush_clients();

            hazel.compositor.space.elements().for_each(|window| {
                window.toplevel().unwrap().send_pending_configure();
            });
        })
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    Ok(())
}

fn init_logging() {
    if let Ok(env_filter) = tracing_subscriber::EnvFilter::try_from_default_env() {
        tracing_subscriber::fmt().with_env_filter(env_filter).init();
    } else {
        tracing_subscriber::fmt().init();
    }
}
