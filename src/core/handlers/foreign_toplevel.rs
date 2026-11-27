use smithay::{
    desktop::Window,
    output::Output,
    reexports::{
        wayland_protocols_wlr::foreign_toplevel::v1::server::{
            zwlr_foreign_toplevel_handle_v1::{self, ZwlrForeignToplevelHandleV1},
            zwlr_foreign_toplevel_manager_v1::{self, ZwlrForeignToplevelManagerV1},
        },
        wayland_server::{
            Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New, Resource,
            backend::{ClientId, GlobalId},
        },
    },
};

use crate::{
    core::{Hazel, shell_window::ShellWindow, window_mapping::is_mapped},
    lua::api::wm_windows::WmWindow,
};

const VERSION: u32 = 3;
const STATE_MAXIMIZED: u32 = 0;
const STATE_MINIMIZED: u32 = 1;
const STATE_ACTIVATED: u32 = 2;
const STATE_FULLSCREEN: u32 = 3;

#[derive(Clone, Debug, PartialEq, Default)]
struct Snapshot {
    title: String,
    app_id: String,
    states: Vec<u32>,
    output: Option<Output>,
}

struct Instance {
    manager: ZwlrForeignToplevelManagerV1,
    handle: ZwlrForeignToplevelHandleV1,
    sent: Option<Snapshot>,
}

struct Entry {
    window: Window,
    instances: Vec<Instance>,
}

pub struct ForeignToplevelState {
    _global: GlobalId,
    managers: Vec<ZwlrForeignToplevelManagerV1>,
    entries: Vec<Entry>,
}

impl ForeignToplevelState {
    pub fn new(display: &DisplayHandle) -> Self {
        Self {
            _global: display.create_global::<Hazel, ZwlrForeignToplevelManagerV1, ()>(VERSION, ()),
            managers: Vec::new(),
            entries: Vec::new(),
        }
    }
}

impl GlobalDispatch<ZwlrForeignToplevelManagerV1, ()> for Hazel {
    fn bind(
        state: &mut Self,
        _: &DisplayHandle,
        _: &Client,
        resource: New<ZwlrForeignToplevelManagerV1>,
        _: &(),
        data_init: &mut DataInit<'_, Self>,
    ) {
        let manager = data_init.init(resource, ());
        state.compositor.smithay.foreign_toplevels.managers.push(manager);
    }
}

impl Dispatch<ZwlrForeignToplevelManagerV1, ()> for Hazel {
    fn request(
        state: &mut Self,
        _: &Client,
        manager: &ZwlrForeignToplevelManagerV1,
        request: zwlr_foreign_toplevel_manager_v1::Request,
        _: &(),
        _: &DisplayHandle,
        _: &mut DataInit<'_, Self>,
    ) {
        if let zwlr_foreign_toplevel_manager_v1::Request::Stop = request {
            manager.finished();
            state.forget_foreign_manager(manager);
        }
    }

    fn destroyed(state: &mut Self, _: ClientId, manager: &ZwlrForeignToplevelManagerV1, _: &()) {
        state.forget_foreign_manager(manager);
    }
}

impl Dispatch<ZwlrForeignToplevelHandleV1, u64> for Hazel {
    fn request(
        state: &mut Self,
        _: &Client,
        _: &ZwlrForeignToplevelHandleV1,
        request: zwlr_foreign_toplevel_handle_v1::Request,
        window_id: &u64,
        _: &DisplayHandle,
        _: &mut DataInit<'_, Self>,
    ) {
        use zwlr_foreign_toplevel_handle_v1::Request;
        let Some(window) = state.compositor.windows().find(|w| WmWindow((*w).clone()).id() == *window_id).cloned() else {
            return;
        };
        let (event, output) = match request {
            Request::SetMaximized => ("maximize_request", None),
            Request::UnsetMaximized => ("unmaximize_request", None),
            Request::SetMinimized => ("minimize_request", None),
            Request::UnsetMinimized => ("unminimize_request", None),
            Request::SetFullscreen { output } => (
                "fullscreen_request",
                output.as_ref().and_then(Output::from_resource),
            ),
            Request::UnsetFullscreen => ("unfullscreen_request", None),
            Request::Activate { .. } => ("activation_request", None),
            Request::Close => {
                WmWindow(window).close();
                return;
            }
            _ => return,
        };
        state.emit_toplevel_request(event, window, output);
    }

    fn destroyed(state: &mut Self, _: ClientId, handle: &ZwlrForeignToplevelHandleV1, _: &u64) {
        for entry in &mut state.compositor.smithay.foreign_toplevels.entries {
            entry.instances.retain(|instance| &instance.handle != handle);
        }
    }
}

impl Hazel {
    fn forget_foreign_manager(&mut self, manager: &ZwlrForeignToplevelManagerV1) {
        let state = &mut self.compositor.smithay.foreign_toplevels;
        state.managers.retain(|m| m != manager);
        for entry in &mut state.entries {
            entry.instances.retain(|instance| &instance.manager != manager);
        }
    }

    fn emit_toplevel_request(&self, event: &str, window: Window, output: Option<Output>) {
        let events = &self.wm().windows.events;
        let window = WmWindow(window);
        let _ = match (event, output) {
            ("fullscreen_request", output) => events.emit(
                event.to_owned(),
                (window, output.map(|o| crate::lua::api::wm_outputs::WmOutputHandle::new(&o))),
            ),
            _ => events.emit(event.to_owned(), window),
        };
    }

    pub fn sync_foreign_toplevels(&mut self) {
        let state = &self.compositor.smithay.foreign_toplevels;
        if self.compositor.session_lock.is_active() || (state.managers.is_empty() && state.entries.is_empty()) {
            return;
        }
        let windows: Vec<Window> = self
            .compositor
            .windows()
            .filter(|window| is_mapped(window) && window.is_shell_alive())
            .cloned()
            .collect();
        let snapshots: Vec<Snapshot> = windows.iter().map(|w| self.toplevel_snapshot(w)).collect();
        let dh = self.display_handle.clone();
        let state = &mut self.compositor.smithay.foreign_toplevels;

        state.managers.retain(|manager| manager.is_alive());
        state.entries.retain(|entry| {
            let open = windows.contains(&entry.window);
            if !open {
                for instance in &entry.instances {
                    instance.handle.closed();
                }
            }
            open
        });
        for window in &windows {
            if !state.entries.iter().any(|entry| &entry.window == window) {
                state.entries.push(Entry { window: window.clone(), instances: Vec::new() });
            }
        }
        let managers = state.managers.clone();
        for entry in &mut state.entries {
            let index = windows.iter().position(|w| w == &entry.window).unwrap_or(0);
            let snapshot = &snapshots[index];
            for manager in &managers {
                if entry.instances.iter().any(|i| &i.manager == manager) {
                    continue;
                }
                let Some(client) = manager.client() else { continue };
                let id = WmWindow(entry.window.clone()).id();
                let Ok(handle) = client.create_resource::<ZwlrForeignToplevelHandleV1, u64, Hazel>(&dh, manager.version(), id) else {
                    continue;
                };
                manager.toplevel(&handle);
                entry.instances.push(Instance { manager: manager.clone(), handle, sent: None });
            }
            for instance in &mut entry.instances {
                send_changes(instance, snapshot);
            }
        }
    }

    fn toplevel_snapshot(&self, window: &Window) -> Snapshot {
        use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::State;
        let wm_window = WmWindow(window.clone());
        
        let flags = [
            (STATE_MAXIMIZED, self.compositor.window_has_state(window, State::Maximized)),
            (STATE_MINIMIZED, !self.compositor.is_window_visible(window)),
            (STATE_ACTIVATED, self.compositor.window_has_state(window, State::Activated)),
            (STATE_FULLSCREEN, self.compositor.window_has_state(window, State::Fullscreen)),
        ];
        let states = flags
            .into_iter()
            .filter_map(|(code, set)| set.then_some(code))
            .collect();
        Snapshot {
            title: wm_window.title().unwrap_or_default(),
            app_id: wm_window.app_id().unwrap_or_default(),
            states,
            output: self.compositor.output_of_window(window),
        }
    }
}

fn send_changes(instance: &mut Instance, now: &Snapshot) {
    let before = instance.sent.clone().unwrap_or_default();
    if instance.sent.as_ref() == Some(now) {
        return;
    }
    let handle = &instance.handle;
    if before.title != now.title || instance.sent.is_none() {
        handle.title(now.title.clone());
    }
    if before.app_id != now.app_id || instance.sent.is_none() {
        handle.app_id(now.app_id.clone());
    }
    if before.output != now.output {
        if let (Some(old), Some(client)) = (&before.output, handle.client()) {
            for wl_output in old.client_outputs(&client) {
                handle.output_leave(&wl_output);
            }
        }
        if let (Some(new), Some(client)) = (&now.output, handle.client()) {
            for wl_output in new.client_outputs(&client) {
                handle.output_enter(&wl_output);
            }
        }
    }
    if before.states != now.states || instance.sent.is_none() {
        let bytes = now.states.iter().flat_map(|state| state.to_ne_bytes()).collect();
        handle.state(bytes);
    }
    handle.done();
    instance.sent = Some(now.clone());
}
