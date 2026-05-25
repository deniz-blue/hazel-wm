use std::{ffi::OsString, path::Path, sync::Arc};

use calloop_notify::notify::{RecursiveMode, Watcher};
use smithay::{
    desktop::{PopupManager, Space, Window, WindowSurfaceType},
    input::{Seat, SeatState},
    reexports::{
        calloop::{EventLoop, Interest, LoopSignal, Mode, PostAction, generic::Generic},
        wayland_server::{Display, DisplayHandle, protocol::wl_surface::WlSurface},
    },
    utils::{Logical, Point},
    wayland::{
        compositor::CompositorState, output::OutputManagerState,
        selection::data_device::DataDeviceState, shell::xdg::XdgShellState, shm::ShmState,
        socket::ListeningSocketSource,
    },
};

use crate::{core::client_state::ClientState, lua::HazelLua};

pub mod client_state;
pub mod handlers;

pub struct Hazel {
    pub start_time: std::time::Instant,
    pub socket_name: OsString,
    pub display_handle: DisplayHandle,

    pub space: Space<Window>,
    pub loop_signal: LoopSignal,

    pub smithay: HazelSmithay,

    pub lua: HazelLua,

    pub seat: Seat<Self>,
}

pub struct HazelSmithay {
    pub compositor_state: CompositorState,
    pub xdg_shell_state: XdgShellState,
    pub shm_state: ShmState,
    pub output_manager_state: OutputManagerState,
    pub seat_state: SeatState<Hazel>,
    pub data_device_state: DataDeviceState,
    pub popups: PopupManager,
}

impl Hazel {
    pub fn new(event_loop: &mut EventLoop<Self>, display: Display<Self>) -> Self {
        let start_time = std::time::Instant::now();

        let dh = display.handle();

        let compositor_state = CompositorState::new::<Self>(&dh);
        let xdg_shell_state = XdgShellState::new::<Self>(&dh);
        let shm_state = ShmState::new::<Self>(&dh, vec![]);
        let popups = PopupManager::default();

        let output_manager_state = OutputManagerState::new_with_xdg_output::<Self>(&dh);

        let data_device_state = DataDeviceState::new::<Self>(&dh);

        let mut seat_state = SeatState::new();
        let mut seat: Seat<Self> = seat_state.new_seat("meow");

        // ! Hack
        seat.add_keyboard(Default::default(), 200, 25).unwrap();
        seat.add_pointer();

        let space = Space::default();

        let socket_name = Self::init_wayland_listener(display, event_loop);

        Self::init_lua_files_listener(event_loop);

        let loop_signal = event_loop.get_signal();

        Self {
            start_time,
            display_handle: dh,

            space,
            loop_signal,
            socket_name,

            smithay: HazelSmithay {
                compositor_state,
                xdg_shell_state,
                shm_state,
                output_manager_state,
                seat_state,
                data_device_state,
                popups,
            },

            lua: HazelLua::new(),

            seat,
        }
    }

    fn init_wayland_listener(
        display: Display<Hazel>,
        event_loop: &mut EventLoop<Self>,
    ) -> OsString {
        let listening_socket = ListeningSocketSource::new_auto().unwrap();

        let socket_name = listening_socket.socket_name().to_os_string();

        let loop_handle = event_loop.handle();

        loop_handle
            .insert_source(listening_socket, move |client_stream, _, state| {
                state
                    .display_handle
                    .insert_client(client_stream, Arc::new(ClientState::default()))
                    .unwrap();
            })
            .expect("Failed to init the wayland event source.");

        // You also need to add the display itself to the event loop, so that client events will be processed by wayland-server.
        loop_handle
            .insert_source(
                Generic::new(display, Interest::READ, Mode::Level),
                |_, display, state| {
                    // Safety: we don't drop the display
                    unsafe {
                        display.get_mut().dispatch_clients(state).unwrap();
                    }
                    Ok(PostAction::Continue)
                },
            )
            .unwrap();

        socket_name
    }

    fn init_lua_files_listener(event_loop: &mut EventLoop<Self>) {
        let loop_handle = event_loop.handle();
        let mut notify_source = calloop_notify::NotifySource::new().unwrap();
        notify_source
            .watch(Path::new("./test"), RecursiveMode::Recursive)
            .unwrap();

        loop_handle
            .insert_source(notify_source, |event, _, hazel| {
                if !event.kind.is_modify() {
                    return;
                }

                println!("Notify Event: {event:?}");
                hazel.lua = HazelLua::new();
            })
            .unwrap();
    }

    pub fn surface_under(
        &self,
        pos: Point<f64, Logical>,
    ) -> Option<(WlSurface, Point<f64, Logical>)> {
        self.space
            .element_under(pos)
            .and_then(|(window, location)| {
                window
                    .surface_under(pos - location.to_f64(), WindowSurfaceType::ALL)
                    .map(|(s, p)| (s, (p + location).to_f64()))
            })
    }
}
