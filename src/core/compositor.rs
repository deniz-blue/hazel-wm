use std::{collections::HashMap, ffi::OsString, sync::Arc};

use smithay::{
    desktop::{PopupManager, Space, Window, WindowSurfaceType, layer_map_for_output},
    input::{Seat, SeatState, keyboard::KeyboardHandle, pointer::PointerHandle},
    reexports::wayland_server::{DisplayHandle, protocol::wl_surface::WlSurface},
    utils::{Logical, Point},
    wayland::{
        compositor::CompositorState,
        output::OutputManagerState,
        selection::data_device::DataDeviceState,
        shell::{
            kde::decoration::KdeDecorationState,
            wlr_layer::{Layer, WlrLayerShellState},
            xdg::XdgShellState,
        },
        shm::ShmState,
        socket::ListeningSocketSource,
    },
};

use smithay::reexports::wayland_protocols_misc::server_decoration::server::org_kde_kwin_server_decoration_manager::Mode;

use tracing::info;

use crate::core::{
    Hazel, HazelEventLoop, client_state::ClientState, input::pointer::PointerState,
};

pub struct HazelCompositor {
    pub socket_name: OsString,
    pub space: Space<Window>,
    pub smithay: HazelSmithay,
    pub seats: HashMap<String, Seat<Hazel>>,
    pub device_to_seat: HashMap<String, String>,
    pub pointers: HashMap<PointerHandle<Hazel>, PointerState>,
}

pub struct HazelSmithay {
    pub compositor_state: CompositorState,
    pub xdg_shell_state: XdgShellState,
    pub layer_shell_state: WlrLayerShellState,
    pub shm_state: ShmState,
    pub output_manager_state: OutputManagerState,
    pub seat_state: SeatState<Hazel>,
    pub data_device_state: DataDeviceState,
    pub popups: PopupManager,
    pub kde_decoration_state: KdeDecorationState,
}

impl HazelCompositor {
    pub fn new(event_loop: &mut HazelEventLoop, dh: &DisplayHandle) -> Self {
        let compositor_state = CompositorState::new::<Hazel>(dh);
        let xdg_shell_state = XdgShellState::new::<Hazel>(dh);
        let layer_shell_state = WlrLayerShellState::new::<Hazel>(dh);
        let shm_state = ShmState::new::<Hazel>(dh, vec![]);
        let popups = PopupManager::default();
        let kde_decoration_state = KdeDecorationState::new::<Hazel>(dh, Mode::Client);
        let output_manager_state = OutputManagerState::new_with_xdg_output::<Hazel>(dh);
        let data_device_state = DataDeviceState::new::<Hazel>(dh);
        let seat_state = SeatState::new();

        let space = Space::default();

        let socket_name = Self::init_wayland_listener(event_loop);

        Self {
            space,
            socket_name,

            smithay: HazelSmithay {
                compositor_state,
                xdg_shell_state,
                layer_shell_state,
                shm_state,
                output_manager_state,
                seat_state,
                data_device_state,
                popups,
                kde_decoration_state,
            },

            seats: HashMap::new(),
            device_to_seat: HashMap::new(),
            pointers: HashMap::new(),
        }
    }

    fn init_wayland_listener(event_loop: &mut HazelEventLoop) -> OsString {
        let listening_socket = ListeningSocketSource::new_auto().unwrap();
        let socket_name = listening_socket.socket_name().to_os_string();
        let loop_handle = event_loop.handle();

        loop_handle
            .insert_source(listening_socket, move |client_stream, _, state| {
                info!("New client connection");
                state
                    .display_handle
                    .insert_client(client_stream, Arc::new(ClientState::default()))
                    .unwrap();
            })
            .expect("Failed to init the wayland event source.");

        socket_name
    }

    pub fn get_seat_of(&self, device_id: &str) -> Option<Seat<Hazel>> {
        self.device_to_seat
            .get(device_id)
            .and_then(|seat_name| self.seats.get(seat_name).cloned())
    }

    pub fn get_pointer_handle(
        &self,
        device_id: &str,
    ) -> Option<(Seat<Hazel>, PointerHandle<Hazel>)> {
        self.get_seat_of(device_id)
            .and_then(|seat| seat.get_pointer().map(|pointer| (seat, pointer)))
    }

    pub fn get_keyboard_handle(
        &self,
        device_id: &str,
    ) -> Option<(Seat<Hazel>, KeyboardHandle<Hazel>)> {
        self.get_seat_of(device_id)
            .and_then(|seat| seat.get_keyboard().map(|keyboard| (seat, keyboard)))
    }

    pub fn window_under(&self, pos: Point<f64, Logical>) -> Option<(&Window, Point<i32, Logical>)> {
        self.space.element_under(pos)
    }

    /// Returns the topmost layer surface and its position under the given point.
    /// Returns the topmost layer surface in the given layers under `pos`.
    /// Checks layers in the order provided (first = front-most).
    /// Translates global coords to each output's local space.
    /// Respects input region — returns None if the point is outside it.
    pub fn layer_under(
        &self,
        pos: Point<f64, Logical>,
        layers: &[Layer],
    ) -> Option<(WlSurface, Point<f64, Logical>)> {
        for output in self.space.outputs() {
            let geo = self.space.output_geometry(output)?;
            let local = pos - geo.loc.to_f64();
            if local.x < 0.0
                || local.y < 0.0
                || local.x > geo.size.w as f64
                || local.y > geo.size.h as f64
            {
                continue;
            }
            let map = layer_map_for_output(output);
            for layer in layers {
                if let Some(layer_surface) = map.layer_under(*layer, local) {
                    let layer_geo = map.layer_geometry(layer_surface).unwrap_or_default();
                    if let Some((s, p)) = layer_surface
                        .surface_under(local - layer_geo.loc.to_f64(), WindowSurfaceType::ALL)
                    {
                        return Some((s, (p + layer_geo.loc + geo.loc).to_f64()));
                    }
                }
            }
        }
        None
    }

    /// Full z-order input query: top layers → windows → bottom layers.
    pub fn input_under(
        &self,
        pos: Point<f64, Logical>,
    ) -> Option<(WlSurface, Point<f64, Logical>)> {
        self.layer_under(pos, &[Layer::Overlay, Layer::Top])
            .or_else(|| {
                self.surface_under(pos)
                    .map(|(_, surface, pt)| (surface, pt))
            })
            .or_else(|| self.layer_under(pos, &[Layer::Bottom, Layer::Background]))
    }

    pub fn surface_under(
        &self,
        pos: Point<f64, Logical>,
    ) -> Option<(Window, WlSurface, Point<f64, Logical>)> {
        self.space
            .element_under(pos)
            .and_then(|(window, location)| {
                window
                    .surface_under(pos - location.to_f64(), WindowSurfaceType::ALL)
                    .map(|(s, p)| (window.clone(), s, (p + location).to_f64()))
            })
    }
}
