use std::{cell::RefCell, collections::HashMap};

use mlua::{Function, Lua, RegistryKey};
use tracing::error;

use crate::core::processes::{ExitReport, LineBuffer, ProcessEvent};

#[derive(Default)]
pub struct Callbacks {
    pub on_stdout: Option<RegistryKey>,
    pub on_stderr: Option<RegistryKey>,
    pub on_exit: Option<RegistryKey>,
}

#[derive(Default)]
struct Observed {
    callbacks: Callbacks,
    stdout: LineBuffer,
    stderr: LineBuffer,
}

#[derive(Default)]
pub struct WmProcesses {
    observed: RefCell<HashMap<u32, Observed>>,
}

impl WmProcesses {
    pub fn register(&self, pid: u32, callbacks: Callbacks) {
        self.observed
            .borrow_mut()
            .insert(pid, Observed { callbacks, ..Default::default() });
    }

    pub fn dispatch(&self, lua: &Lua, event: ProcessEvent) {
        match event {
            ProcessEvent::Stdout(pid, chunk) => self.deliver_lines(lua, pid, chunk, true),
            ProcessEvent::Stderr(pid, chunk) => self.deliver_lines(lua, pid, chunk, false),
            ProcessEvent::Exited(pid, report) => self.deliver_exit(lua, pid, report),
        }
    }

    fn callback(&self, lua: &Lua, pid: u32, pick: impl Fn(&Callbacks) -> Option<&RegistryKey>) -> Option<Function> {
        let observed = self.observed.borrow();
        let key = pick(&observed.get(&pid)?.callbacks)?;
        lua.registry_value(key).ok()
    }

    fn deliver_lines(&self, lua: &Lua, pid: u32, chunk: Vec<u8>, is_stdout: bool) {
        let lines = {
            let mut observed = self.observed.borrow_mut();
            let Some(observed) = observed.get_mut(&pid) else { return };
            let buffer = if is_stdout { &mut observed.stdout } else { &mut observed.stderr };
            buffer.push(&chunk)
        };
        self.call_with_lines(lua, pid, is_stdout, lines);
    }

    fn call_with_lines(&self, lua: &Lua, pid: u32, is_stdout: bool, lines: Vec<Vec<u8>>) {
        let callback = self.callback(lua, pid, |callbacks| {
            if is_stdout { callbacks.on_stdout.as_ref() } else { callbacks.on_stderr.as_ref() }
        });
        let Some(callback) = callback else { return };
        for line in lines {
            let Ok(line) = lua.create_string(&line) else { continue };
            if let Err(e) = callback.call::<()>(line) {
                error!("Error in process output callback: {e}");
            }
        }
    }

    fn deliver_exit(&self, lua: &Lua, pid: u32, report: ExitReport) {
        let rest = {
            let mut observed = self.observed.borrow_mut();
            let Some(observed) = observed.get_mut(&pid) else { return };
            (observed.stdout.finish(), observed.stderr.finish())
        };
        self.call_with_lines(lua, pid, true, rest.0.into_iter().collect());
        self.call_with_lines(lua, pid, false, rest.1.into_iter().collect());
        let callback = self.callback(lua, pid, |callbacks| callbacks.on_exit.as_ref());
        if let Some(callback) = callback {
            let result = exit_table(lua, report).and_then(|table| callback.call::<()>(table));
            if let Err(e) = result {
                error!("Error in process exit callback: {e}");
            }
        }
        if let Some(observed) = self.observed.borrow_mut().remove(&pid) {
            for key in [observed.callbacks.on_stdout, observed.callbacks.on_stderr, observed.callbacks.on_exit].into_iter().flatten() {
                let _ = lua.remove_registry_value(key);
            }
        }
    }
}

pub fn exit_table(lua: &Lua, report: ExitReport) -> mlua::Result<mlua::Table> {
    let table = lua.create_table()?;
    table.set("status", report.code)?;
    table.set("signal", report.signal)?;
    table.set("timed_out", report.timed_out)?;
    Ok(table)
}
