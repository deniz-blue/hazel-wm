use std::{ffi::OsString, sync::Arc};

use smithay::{
    desktop::{PopupManager, Space, Window, WindowSurfaceType},
    input::{Seat, SeatState},
    reexports::wayland_server::{DisplayHandle, protocol::wl_surface::WlSurface},
    utils::{Logical, Point},
    wayland::{
        compositor::CompositorState, output::OutputManagerState,
        selection::data_device::DataDeviceState, shell::xdg::XdgShellState, shm::ShmState,
        socket::ListeningSocketSource,
    },
};

use crate::core::{Hazel, HazelEventLoop, client_state::ClientState};

pub struct HazelCompositor {
    pub socket_name: OsString,
    pub space: Space<Window>,
    pub smithay: HazelSmithay,
    pub seat: Seat<Hazel>,
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

impl HazelCompositor {
    pub fn new(event_loop: &mut HazelEventLoop, dh: &DisplayHandle) -> Self {
        let compositor_state = CompositorState::new::<Hazel>(&dh);
        let xdg_shell_state = XdgShellState::new::<Hazel>(&dh);
        let shm_state = ShmState::new::<Hazel>(&dh, vec![]);
        let popups = PopupManager::default();

        let output_manager_state = OutputManagerState::new_with_xdg_output::<Hazel>(&dh);

        let data_device_state = DataDeviceState::new::<Hazel>(&dh);

        let mut seat_state = SeatState::new();
        let mut seat: Seat<Hazel> = seat_state.new_seat("meow");

        // ! Hack
        seat.add_keyboard(Default::default(), 200, 25).unwrap();
        seat.add_pointer();

        let space = Space::default();

        let socket_name = Self::init_wayland_listener(event_loop);

        Self {
            space,
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

            seat,
        }
    }

    fn init_wayland_listener(event_loop: &mut HazelEventLoop) -> OsString {
        let listening_socket = ListeningSocketSource::new_auto().unwrap();

        let socket_name = listening_socket.socket_name().to_os_string();

        let loop_handle = event_loop.handle();

        loop_handle
            .insert_source(listening_socket, move |client_stream, _, state| {
				println!("New client connection");
                state
                    .display_handle
                    .insert_client(client_stream, Arc::new(ClientState::default()))
                    .unwrap();
            })
            .expect("Failed to init the wayland event source.");

        socket_name
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
