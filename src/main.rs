#![allow(irrefutable_let_patterns)]

pub mod backend;
pub mod core;
pub mod lua;

use crate::core::shell_window::ShellWindow;
use std::time::Duration;

use smithay::reexports::calloop::EventLoop;
use tracing::{error, info, debug};

use crate::core::{GlobalHazel, Hazel, HazelEventLoop};
use crate::lua::export_typedefs::dump_typedefs;

const LOOP_INTERVAL: Duration = Duration::from_millis(20);
const CHILD_EXIT_GRACE: Duration = Duration::from_millis(500);

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
	info!("Initializing event loop...");

    let mut event_loop: HazelEventLoop =
        EventLoop::try_new().map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

	info!("Before Hazel::new()...");
    let mut state = Hazel::new(&mut event_loop)?;
	info!("After Hazel::new()...");

    // Safety: single threaded
    unsafe { std::env::set_var("WAYLAND_DISPLAY", &state.compositor.socket_name) };
	debug!("WAYLAND_DISPLAY set to {}", state.compositor.socket_name.display());

    GlobalHazel::execute(&mut state, |hazel| hazel.load_lua());

	info!("Starting event loop...");

    event_loop
        .run(Some(LOOP_INTERVAL), &mut state, move |hazel| {
            GlobalHazel::execute(hazel, |hazel| {
                hazel.emit_pending_focus_changes();
                let _ = hazel.wm().events.emit("tick".to_owned(), ());
                hazel.run_scheduled_work();
            });

            hazel.compositor.restack();
            hazel.sync_foreign_toplevels();
            hazel.sync_foreign_toplevel_list();
            hazel.compositor.space.refresh();
            hazel.compositor.smithay.popups.cleanup();
            let _ = hazel.display_handle.flush_clients();

            for window in hazel.compositor.all_windows() {
                window.send_pending_configure();
            }
        })
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

	info!("Event loop exited, shutting down...");
    state.processes.terminate_all(CHILD_EXIT_GRACE);

    Ok(())
}

fn init_logging() {
    if let Ok(env_filter) = tracing_subscriber::EnvFilter::try_from_default_env() {
        tracing_subscriber::fmt().with_env_filter(env_filter).init();
    } else {
        tracing_subscriber::fmt().pretty().init();
    }
}
