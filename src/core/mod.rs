use std::{cell::Cell, rc::Rc, time::Instant};

use smithay::reexports::{
    calloop::{EventLoop, Interest, LoopSignal, Mode, PostAction, generic::Generic},
    wayland_server::{Display, DisplayHandle},
};

use crate::{
    core::compositor::HazelCompositor,
    lua::{api::wm::Wm, runtime::HazelLua},
};

pub mod client_state;
pub mod compositor;
pub mod handlers;
pub mod input;

pub struct Hazel {
    pub start_time: Instant,
    pub display_handle: DisplayHandle,
    pub loop_signal: LoopSignal,
    pub compositor: HazelCompositor,
    pub lua: HazelLua,
}

pub type HazelEventLoop<'a> = EventLoop<'a, Hazel>;

impl Hazel {
    pub fn new(event_loop: &mut HazelEventLoop) -> Result<Self, Box<dyn std::error::Error>> {
        let start_time = std::time::Instant::now();
        let display = Display::new()?;
        let display_handle = display.handle();
        let compositor = HazelCompositor::new(event_loop, &display_handle);
        let lua = HazelLua::new_uninit();
        let loop_signal = event_loop.get_signal();

        event_loop.handle().insert_source(
            Generic::new(display, Interest::READ, Mode::Level),
            |_, display, state| {
                GlobalHazel::execute(state, |hazel| {
                    // Safety: we don't drop the display
                    unsafe { display.get_mut().dispatch_clients(hazel) }
                })?;

                Ok(PostAction::Continue)
            },
        )?;

        Ok(Self {
            start_time,
            display_handle,
            loop_signal,
            compositor,
            lua,
        })
    }

    pub fn wm(&mut self) -> Rc<Wm> {
        self.lua.wm.clone()
    }
}

thread_local! {
    static HAZEL: Cell<Option<*mut Hazel>> = const { Cell::new(None) };
}

pub struct GlobalHazel;

impl GlobalHazel {
    pub fn execute<R>(hazel: &mut Hazel, f: impl FnOnce(&mut Hazel) -> R) -> R {
        let ptr = hazel as *mut Hazel;
        HAZEL.with(|cell| cell.set(Some(ptr)));
        struct Guard;
        impl Drop for Guard {
            fn drop(&mut self) {
                HAZEL.with(|cell| cell.set(None));
            }
        }
        let _guard = Guard;
        f(hazel)
    }

    pub fn with<T, F>(f: F) -> Result<T, mlua::Error>
    where
        F: FnOnce(&mut Hazel) -> Result<T, mlua::Error>,
    {
        HAZEL.with(|cell| {
            if let Some(ptr) = cell.get() {
                let hazel = unsafe { &mut *ptr };
                f(hazel)
            } else {
                eprintln!("GlobalHazel accessed outside of execute");
                Err(mlua::Error::external(
                    "GlobalHazel accessed outside of execute",
                ))
            }
        })
    }
}
