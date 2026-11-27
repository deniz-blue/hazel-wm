use std::rc::Rc;

use mlua::{Function, Lua, Table, UserData};

use crate::{
    core::{
        GlobalHazel,
        compositor::ScreenshotRequest,
        processes::{import_environment, is_valid_name},
    },
    impl_lua_event_handler, impl_lua_event_source,
    lua::{
        api::{
            wm_clipboard::WmClipboard,
            wm_input::WmInput,
            wm_layers::WmLayers,
            wm_outputs::{WmOutputHandle, WmOutputs},
            wm_processes::WmProcesses,
            wm_session_lock::WmSessionLock,
            wm_shapes::WmText,
            wm_timers::{WmTimer, WmTimers},
            wm_windows::WmWindows,
        },
        event_handler::LuaEventHandler,
    },
    lua_typedef,
};

pub const STORE_REGISTRY_KEY: &str = "hazel.store";

#[derive(Default)]
pub struct Wm {
    pub events: LuaEventHandler,
    pub input: Rc<WmInput>,
    pub clipboard: Rc<WmClipboard>,
    pub session_lock: Rc<WmSessionLock>,
    pub windows: Rc<WmWindows>,
    pub layers: Rc<WmLayers>,
    pub outputs: Rc<WmOutputs>,
    pub text: Rc<WmText>,
    pub timers: Rc<WmTimers>,
    pub processes: Rc<WmProcesses>,
    pub is_reload: bool,
}

impl_lua_event_source!(Wm);

impl Wm {
    pub fn new(is_reload: bool) -> Self {
        Self {
            is_reload,
            ..Default::default()
        }
    }

    fn start_timer(
        &self,
        lua: &Lua,
        delay_ms: f64,
        callback: Function,
        repeats: bool,
    ) -> mlua::Result<WmTimer> {
        GlobalHazel::try_with(|hazel| {
            let now = hazel.start_time.elapsed().as_millis() as u64;
            let delay = delay_ms.max(0.0) as u64;
            let id = self.timers.add(lua, now, delay, repeats, callback)?;
            Ok(WmTimer {
                id,
                timers: self.timers.clone(),
            })
        })
    }
}

fn environment_assignments(variables: &Table) -> mlua::Result<Vec<(String, String)>> {
    variables
        .pairs::<String, String>()
        .map(|pair| {
            let (name, value) = pair?;
            if is_valid_name(&name) {
                Ok((name, value))
            } else {
                Err(mlua::Error::external(format!("{name:?} is not an environment variable name")))
            }
        })
        .collect()
}

impl UserData for Wm {
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("name", |_, _| Ok("Hazel"));
        fields.add_field_method_get("input", |_, this| Ok(this.input.clone()));
        fields.add_field_method_get("clipboard", |_, this| Ok(this.clipboard.clone()));
        fields.add_field_method_get("session_lock", |_, this| Ok(this.session_lock.clone()));
        fields.add_field_method_get("windows", |_, this| Ok(this.windows.clone()));
        fields.add_field_method_get("layers", |_, this| Ok(this.layers.clone()));
        fields.add_field_method_get("outputs", |_, this| Ok(this.outputs.clone()));
        fields.add_field_method_get("text", |_, this| Ok(this.text.clone()));
        fields.add_field_method_get("is_reload", |_, this| Ok(this.is_reload));
        fields.add_field_method_get("wayland_display", |_, _| {
            GlobalHazel::try_with(|hazel| Ok(hazel.compositor.socket_name.to_string_lossy().into_owned()))
        });
        fields.add_field_method_get("is_session_locked", |_, _| {
            GlobalHazel::try_with(|hazel| Ok(hazel.compositor.session_lock.is_active()))
        });
        fields.add_field_method_get("store", |lua, _| {
            lua.named_registry_value::<Table>(STORE_REGISTRY_KEY)
        });
    }

    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        impl_lua_event_handler!(methods);

        methods.add_method("time", |_, _this, ()| {
            GlobalHazel::try_with(|hazel| Ok(hazel.start_time.elapsed().as_millis() as u64))
        });

        methods.add_method("screenshot", |_, _this, (path, output): (String, Option<WmOutputHandle>)| {
            GlobalHazel::try_with(|hazel| {
                let output = output.map(|handle| handle.0);
                hazel.compositor.screenshot_requests.push(ScreenshotRequest { path: path.into(), output });
                Ok(())
            })
        });

        methods.add_method("after", |lua, this, (delay_ms, callback): (f64, Function)| {
            this.start_timer(lua, delay_ms, callback, false)
        });

        methods.add_method("every", |lua, this, (delay_ms, callback): (f64, Function)| {
            this.start_timer(lua, delay_ms, callback, true)
        });

        methods.add_method("reload", |_, _this, ()| {
            GlobalHazel::try_with(|hazel| {
                hazel.reload_requested = true;
                Ok(())
            })
        });

        methods.add_method("import_environment", |_, _this, variables: Table| {
            let variables = environment_assignments(&variables)?;
            Ok(import_environment(&variables))
        });

        methods.add_method("quit", |_, _this, ()| {
            GlobalHazel::try_with(|hazel| {
                let _: () = hazel.loop_signal.stop();
                Ok(())
            })
        });
    }
}

lua_typedef!(Wm => Wm {
    extern wm;
    let name: string;
    let input: WmInput;
    let clipboard: WmClipboard;
    let session_lock: WmSessionLock;
    let windows: WmWindows;
    let layers: WmLayers;
    let outputs: WmOutputs;
    let text: WmText;
    let is_reload: boolean;
    let wayland_display: string;
    let is_session_locked: boolean;
    let store: table;
    use ready => nil;
    use unload => nil;
    use session_locked => nil;
    use session_unlocked => nil;
    use tick => nil;
    fn after(delay_ms: number, callback: function) -> Timer;
    fn every(interval_ms: number, callback: function) -> Timer;
    fn reload() -> nil;
    fn time() -> number;
    fn screenshot(path: string, output: Option<WmOutput>) -> nil;
    fn import_environment(variables: table) -> Array<string>;
    fn quit() -> nil;
});
