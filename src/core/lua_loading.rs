use tracing::{error, info};

use crate::{
    core::Hazel,
    lua::{api::wm_outputs::WmOutputHandle, runtime::HazelLua},
};

impl Hazel {
    pub fn load_lua(&mut self) {
        let is_reload = self.lua.loaded;
        if is_reload {
            let _ = self.wm().events.emit("unload".to_owned(), ());
        }
        let store = match self.lua.snapshot_store() {
            Ok(store) => store,
            Err(e) => {
                error!("Not reloading, the store cannot be carried over: {e}");
                return;
            }
        };
        let stale_processes = self.processes.observed_pids();
        let previous_generation = self.compositor.drawing.start_generation();
        let next = match HazelLua::load(store, is_reload) {
            Ok(next) => next,
            Err(e) => {
                error!("Error loading Lua, keeping the current one: {e}");
                self.compositor.drawing.abandon_generation(previous_generation);
                return;
            }
        };
        self.compositor.drawing.keep_only_current_generation();
        self.release_pointer_grabs();
        self.processes.drop_observers_on_reload(&stale_processes);
        self.lua = next;
        self.announce_existing_outputs();
        let _ = self.wm().events.emit("ready".to_owned(), ());
        info!("{} Lua", if is_reload { "Reloaded" } else { "Loaded" });
    }

    fn announce_existing_outputs(&self) {
        for output in self.compositor.space.outputs() {
            let _ = self
                .wm()
                .outputs
                .events
                .emit("added".to_owned(), WmOutputHandle::new(output));
        }
    }

    fn release_pointer_grabs(&mut self) {
        let pointers: Vec<_> = self.compositor.pointers.keys().cloned().collect();
        for pointer in pointers {
            self.release_pointer_grab(&pointer);
        }
    }
}
