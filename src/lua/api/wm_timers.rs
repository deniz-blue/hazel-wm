use std::{cell::RefCell, collections::HashMap, rc::Rc};

use mlua::{Function, Lua, RegistryKey, UserData};
use tracing::error;

use crate::{lua::timers::Timers, lua_typedef};

#[derive(Default)]
pub struct WmTimers {
    schedule: RefCell<Timers>,
    callbacks: RefCell<HashMap<u64, RegistryKey>>,
}

impl WmTimers {
    pub fn add(
        &self,
        lua: &Lua,
        now_ms: u64,
        delay_ms: u64,
        repeats: bool,
        callback: Function,
    ) -> mlua::Result<u64> {
        let key = lua.create_registry_value(callback)?;
        let id = self.schedule.borrow_mut().add(now_ms, delay_ms, repeats);
        self.callbacks.borrow_mut().insert(id, key);
        Ok(id)
    }

    pub fn cancel(&self, lua: &Lua, id: u64) -> bool {
        let cancelled = self.schedule.borrow_mut().cancel(id);
        if let Some(key) = self.callbacks.borrow_mut().remove(&id) {
            let _ = lua.remove_registry_value(key);
        }
        cancelled
    }

    pub fn fire_due(&self, lua: &Lua, now_ms: u64) {
        let due = self.schedule.borrow_mut().take_due(now_ms);
        for timer in due {
            let Some(callback) = self.callback_for(lua, timer.id, timer.repeats) else {
                continue;
            };
            if let Err(e) = callback.call::<()>(()) {
                error!("Error in timer callback: {e}");
            }
        }
    }

    fn callback_for(&self, lua: &Lua, id: u64, repeats: bool) -> Option<Function> {
        if repeats {
            let callbacks = self.callbacks.borrow();
            return lua.registry_value(callbacks.get(&id)?).ok();
        }
        let key = self.callbacks.borrow_mut().remove(&id)?;
        let callback = lua.registry_value(&key).ok();
        let _ = lua.remove_registry_value(key);
        callback
    }
}

pub struct WmTimer {
    pub id: u64,
    pub timers: Rc<WmTimers>,
}

impl UserData for WmTimer {
    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("cancel", |lua, this, _: ()| {
            Ok(this.timers.cancel(lua, this.id))
        });
    }
}

lua_typedef!(Timer => WmTimer {
    fn cancel() -> boolean;
});

#[cfg(test)]
mod tests {
    use super::*;

    fn counting_lua() -> (Lua, Rc<WmTimers>) {
        let lua = Lua::new();
        lua.load("count = 0").exec().unwrap();
        (lua, Rc::new(WmTimers::default()))
    }

    fn bump(lua: &Lua) -> Function {
        lua.load("return function() count = count + 1 end").eval().unwrap()
    }

    fn count(lua: &Lua) -> i64 {
        lua.globals().get("count").unwrap()
    }

    #[test]
    fn a_one_shot_callback_runs_once_and_is_dropped() {
        let (lua, timers) = counting_lua();
        timers.add(&lua, 0, 10, false, bump(&lua)).unwrap();
        timers.fire_due(&lua, 9);
        assert_eq!(count(&lua), 0);
        timers.fire_due(&lua, 10);
        timers.fire_due(&lua, 100);
        assert_eq!(count(&lua), 1);
        assert!(timers.callbacks.borrow().is_empty());
    }

    #[test]
    fn a_repeating_callback_keeps_running_until_cancelled() {
        let (lua, timers) = counting_lua();
        let id = timers.add(&lua, 0, 10, true, bump(&lua)).unwrap();
        timers.fire_due(&lua, 10);
        timers.fire_due(&lua, 20);
        assert_eq!(count(&lua), 2);
        assert!(timers.cancel(&lua, id));
        timers.fire_due(&lua, 30);
        assert_eq!(count(&lua), 2);
    }

    #[test]
    fn a_callback_may_cancel_a_timer_that_is_due_in_the_same_batch() {
        let (lua, timers) = counting_lua();
        let second = Rc::new(RefCell::new(0));
        let cancel_second = {
            let timers = timers.clone();
            let second = second.clone();
            lua.create_function(move |lua, ()| {
                timers.cancel(lua, *second.borrow());
                Ok(())
            })
            .unwrap()
        };
        timers.add(&lua, 0, 5, false, cancel_second).unwrap();
        *second.borrow_mut() = timers.add(&lua, 0, 6, false, bump(&lua)).unwrap();
        timers.fire_due(&lua, 10);
        assert_eq!(count(&lua), 0);
    }

    #[test]
    fn a_failing_callback_does_not_stop_the_others() {
        let (lua, timers) = counting_lua();
        let failing: Function = lua.load("return function() error('boom') end").eval().unwrap();
        timers.add(&lua, 0, 1, false, failing).unwrap();
        timers.add(&lua, 0, 2, false, bump(&lua)).unwrap();
        timers.fire_due(&lua, 5);
        assert_eq!(count(&lua), 1);
    }
}
