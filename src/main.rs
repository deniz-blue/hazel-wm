#![allow(irrefutable_let_patterns)]

pub mod backend;
pub mod core;
pub mod lua;

use smithay::reexports::calloop::EventLoop;

use crate::backend::Backend;
use crate::core::Hazel;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();

    let mut event_loop: EventLoop<Hazel> = EventLoop::try_new()?;
    let mut state = Hazel::new(&mut event_loop)?;

    let backend = Backend::new_winit();
    backend.initialize(&mut state.borrow_mut(), &mut event_loop)?;

    // Safety: single threaded
    unsafe { std::env::set_var("WAYLAND_DISPLAY", &state.borrow().compositor.socket_name) };

    spawn_client();

    event_loop.run(None, &mut state.borrow_mut(), move |_| {
        //
    })?;

    Ok(())
}

fn init_logging() {
    if let Ok(env_filter) = tracing_subscriber::EnvFilter::try_from_default_env() {
        tracing_subscriber::fmt().with_env_filter(env_filter).init();
    } else {
        tracing_subscriber::fmt().init();
    }
}

fn spawn_client() {
    let mut args = std::env::args().skip(1);
    let flag = args.next();
    let arg = args.next();

    match (flag.as_deref(), arg) {
        (Some("-c") | Some("--command"), Some(command)) => {
            std::process::Command::new(command).spawn().ok();
        }
        _ => {
            std::process::Command::new("weston-terminal").spawn().ok();
        }
    }
}
