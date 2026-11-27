use std::{cell::Cell, rc::Rc, time::Instant};

use smithay::reexports::{
    calloop::{EventLoop, Interest, LoopSignal, Mode, PostAction, generic::Generic},
    wayland_server::{Display, DisplayHandle},
};
use std::error::Error as StdError;
use tracing::{error, debug};

use crate::{
    backend::Backend,
    core::compositor::HazelCompositor,
    lua::{api::wm::Wm, runtime::HazelLua},
};

pub mod backend_post_init;
pub mod capture;
pub mod client_state;
pub mod compositor;
pub mod drawing;
pub mod fd_transfer;
pub mod focus;
pub mod handlers;
pub mod input;
pub mod locking;
pub mod lua_loading;
pub mod outputs;
pub mod popups;
pub mod processes;
pub mod presentation;
pub mod render;
pub mod screenshot;
pub mod session_lock;
pub mod shell_window;
pub mod window_borders;
pub mod window_drag;
pub mod window_layers;
pub mod window_lifecycle;
pub mod layer_properties;
pub mod window_mapping;
pub mod window_state;
pub mod window_visibility;
pub mod clipboard;
pub mod clipboard_access;

pub struct Hazel {
    pub start_time: Instant,
    pub display_handle: DisplayHandle,
    pub loop_signal: LoopSignal,
    pub compositor: HazelCompositor,
    pub backend: Backend,
    pub lua: HazelLua,
    pub processes: processes::Processes,
    pub reload_requested: bool,
}

pub type HazelEventLoop = EventLoop<'static, Hazel>;

impl Hazel {
    pub fn new(event_loop: &mut HazelEventLoop) -> std::result::Result<Self, Box<dyn StdError>> {
        let start_time = std::time::Instant::now();
        let display = Display::new().map_err(|e| Box::new(e) as Box<dyn StdError>)?;
        let display_handle = display.handle();
        let mut compositor = HazelCompositor::new(event_loop, &display_handle)?;
        let lua = HazelLua::new_uninit();
        let loop_signal = event_loop.get_signal();

		debug!("Creating backend...");

        let backend = if std::env::var("WAYLAND_DISPLAY").is_err() {
            Backend::create_tty(&display_handle, event_loop)
        } else {
            Backend::create_winit(&display_handle, &mut compositor.smithay.dmabuf_state, event_loop)
        }?;

		debug!("Backend created: {:?}", backend);

        event_loop
            .handle()
            .insert_source(
                Generic::new(display, Interest::READ, Mode::Level),
                |_, display, state| {
                    GlobalHazel::execute(state, |hazel| {
                        // Safety: we don't drop the display
                        unsafe { display.get_mut().dispatch_clients(hazel) }
                    })?;

                    Ok(PostAction::Continue)
                },
            )
            .map_err(|e| Box::new(e) as Box<dyn StdError>)?;

		debug!("Display source inserted into event loop");
		
        lua.listen(event_loop)?;

		debug!("Lua notify listener inserted into event loop");

        let mut this = Self {
            start_time,
            display_handle,
            loop_signal,
            compositor,
            lua,
            backend,
            processes: processes::Processes::default(),
            reload_requested: false,
        };

        this.backend_post_init()?;

		debug!("Backend post-init completed");

        Ok(this)
    }

    pub fn wm(&self) -> Rc<Wm> {
        self.lua.wm.clone()
    }

    fn deliver_clipboard_events(&mut self) {
        let changes = self.compositor.clipboard.take_changes();
        let reads = self.compositor.clipboard.take_finished_reads();
        let clipboard = self.wm().clipboard.clone();
        clipboard.deliver_changes(&self.lua.lua, changes);
        clipboard.deliver_reads(&self.lua.lua, reads);
    }

    pub fn run_scheduled_work(&mut self) {
        let now = self.start_time.elapsed().as_millis() as u64;
        self.wm().timers.fire_due(&self.lua.lua, now);
        let wm = self.wm();
        self.deliver_clipboard_events();
        for event in self.processes.poll(std::time::Instant::now()) {
            wm.processes.dispatch(&self.lua.lua, event);
        }
        if std::mem::take(&mut self.reload_requested) {
            self.load_lua();
        }
    }
}

thread_local! {
    static HAZEL: Cell<Option<*mut Hazel>> = const { Cell::new(None) };
}

pub struct GlobalHazel;

impl GlobalHazel {
    pub fn execute<R>(hazel: &mut Hazel, f: impl FnOnce(&mut Hazel) -> R) -> R {
        let ptr = hazel as *mut Hazel;
        let previous = HAZEL.with(|cell| cell.replace(Some(ptr)));
        struct Guard(Option<*mut Hazel>);
        impl Drop for Guard {
            fn drop(&mut self) {
                HAZEL.with(|cell| cell.set(self.0));
            }
        }
        let _guard = Guard(previous);
        f(hazel)
    }

    pub fn with<T>(f: impl FnOnce(&mut Hazel) -> T) -> mlua::Result<T> {
        Self::try_with(|hazel| Ok(f(hazel)))
    }

    pub fn try_with<T, F>(f: F) -> mlua::Result<T>
    where
        F: FnOnce(&mut Hazel) -> mlua::Result<T>,
    {
        HAZEL.with(|cell| {
            if let Some(ptr) = cell.get() {
                let hazel = unsafe { &mut *ptr };
                f(hazel)
            } else {
                error!("GlobalHazel accessed outside of execute");
                Err(mlua::Error::external(
                    "GlobalHazel accessed outside of execute",
                ))
            }
        })
    }
}

#[macro_export]
macro_rules! with_hazel {
    ($var:ident => $expr:expr) => {
        $crate::core::GlobalHazel::with(|$var| $expr)
    };
}
