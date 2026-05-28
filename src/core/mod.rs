use std::{cell::RefCell, path::Path, sync::Arc};

use calloop_notify::notify::{RecursiveMode, Watcher};
use smithay::reexports::{
    calloop::{EventLoop, Interest, LoopSignal, Mode, PostAction, generic::Generic},
    wayland_server::{Display, DisplayHandle},
};

use crate::{core::compositor::HazelCompositor, lua::{HazelHandle, api::wm::Wm, runtime::HazelLua}};

pub mod client_state;
pub mod compositor;
pub mod handlers;
pub mod input;

pub struct Hazel {
    pub display_handle: DisplayHandle,
    pub loop_signal: LoopSignal,
    pub compositor: HazelCompositor,
    pub lua: HazelLua,
}

impl Hazel {
    pub fn new(event_loop: &mut EventLoop<Self>) -> Result<HazelHandle, Box<dyn std::error::Error>> {
        let display = Display::new()?;
        let display_handle = display.handle();
        let compositor = HazelCompositor::new(event_loop, &display_handle);
        let lua = HazelLua::new_uninit();
        let loop_signal = event_loop.get_signal();

        event_loop.handle().insert_source(
            Generic::new(display, Interest::READ, Mode::Level),
            |_, display, state| {
                // Safety: we don't drop the display
                unsafe {
                    display.get_mut().dispatch_clients(state).unwrap();
                }
                Ok(PostAction::Continue)
            },
        )?;

        let this = Self {
            display_handle,
            loop_signal,
            compositor,
            lua,
        };

		let handle = HazelHandle::new(RefCell::new(this));
		
		handle.borrow_mut().lua.init(handle.clone())?;

		let mut notify_source = calloop_notify::NotifySource::new()?;
        notify_source.watch(Path::new("./test"), RecursiveMode::Recursive)?;
		let handle_clone = handle.clone();
        event_loop
            .handle()
            .insert_source(notify_source, move |event, _, state| {
                if !event.kind.is_modify() {
                    return;
                }

                let _ = state.lua.init(handle_clone.clone());
            })?;

		Ok(handle)
    }

	pub fn wm(&self) -> Arc<Wm> {
		self.lua.wm()
	}
}
