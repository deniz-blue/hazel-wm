use std::{collections::HashMap, ffi::OsString, path::PathBuf, sync::Arc};

use smithay::{
    output::Output,
    desktop::{PopupManager, Space, Window, WindowSurfaceType, layer_map_for_output},
    input::{
        Seat, SeatState,
        keyboard::{KeyboardHandle, ModifiersState},
        pointer::PointerHandle,
    },
    reexports::wayland_server::{DisplayHandle, protocol::wl_surface::WlSurface},
    utils::{Clock, ClockSource, Logical, Monotonic, Point},
    wayland::{
        compositor::CompositorState,
        output::OutputManagerState,
        cursor_shape::CursorShapeManagerState,
        selection::{
            data_device::DataDeviceState,
            ext_data_control::DataControlState as ExtDataControlState,
            primary_selection::PrimarySelectionState,
            wlr_data_control::DataControlState as WlrDataControlState,
        },
        fractional_scale::FractionalScaleManagerState,
        idle_inhibit::IdleInhibitManagerState,
        idle_notify::IdleNotifierState,
        keyboard_shortcuts_inhibit::KeyboardShortcutsInhibitState,
        presentation::PresentationState,
        xdg_system_bell::XdgSystemBellState,
        single_pixel_buffer::SinglePixelBufferState,
        viewporter::ViewporterState,
        xdg_activation::XdgActivationState,
        shell::{
            kde::decoration::KdeDecorationState,
            wlr_layer::{Layer, WlrLayerShellState},
            xdg::{XdgShellState, decoration::XdgDecorationState},
        },
        shm::ShmState,
        socket::ListeningSocketSource,
    },
};

use smithay::reexports::wayland_protocols_misc::server_decoration::server::org_kde_kwin_server_decoration_manager::Mode;

use tracing::{error, info};

use smithay::{
    backend::allocator::dmabuf::Dmabuf,
    wayland::{
        dmabuf::{DmabufState, ImportNotifier},
        foreign_toplevel_list::ForeignToplevelListState,
        input_method::InputMethodManagerState,
        pointer_constraints::PointerConstraintsState,
        pointer_gestures::PointerGesturesState,
        relative_pointer::RelativePointerManagerState,
        text_input::TextInputManagerState,
        session_lock::SessionLockManagerState,
    },
};
use smithay::reexports::calloop::LoopHandle;
use smithay::output::WeakOutput;
use crate::core::handlers::{foreign_toplevel::ForeignToplevelState, image_copy_capture::ImageCaptureState, screencopy::ScreencopyState};
use smithay::wayland::foreign_toplevel_list::ForeignToplevelHandle;
use crate::core::drawing::Drawing;
use crate::core::session_lock::SessionLock;
use crate::core::{
    Hazel, HazelEventLoop, client_state::ClientState, input::pointer::PointerState,
    clipboard::ClipboardState,
};

pub struct ScreenshotRequest {
    pub path: PathBuf,
    pub output: Option<WeakOutput>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KeyRepeat {
    pub rate: i32,
    pub delay: i32,
}

impl Default for KeyRepeat {
    fn default() -> Self {
        Self { rate: 25, delay: 200 }
    }
}

pub struct HazelCompositor {
    pub loop_handle: LoopHandle<'static, Hazel>,
    pub socket_name: OsString,
    pub space: Space<Window>,
    pub smithay: HazelSmithay,
    pub seats: HashMap<String, Seat<Hazel>>,
    pub device_to_seat: HashMap<String, String>,
    pub pointers: HashMap<PointerHandle<Hazel>, PointerState>,
    pub screenshot_requests: Vec<ScreenshotRequest>,
    pub clipboard: ClipboardState,
    pub key_repeat: KeyRepeat,
    pub session_lock: SessionLock,
    pub listed_toplevels: Vec<(Window, ForeignToplevelHandle)>,
    pub pending_dmabuf_imports: Vec<(Dmabuf, ImportNotifier)>,
    pub hidden_windows: Vec<(Window, Point<i32, Logical>)>,
    pub pending_focus_changes: Vec<(String, Option<Window>)>,
    pub idle_inhibitors: Vec<WlSurface>,
    pub presentation_clock: Clock<Monotonic>,
    pub presentation_sequence: u64,
    pub drawing: Drawing,
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
    pub xdg_decoration_state: XdgDecorationState,
    pub primary_selection_state: PrimarySelectionState,
    pub xdg_activation_state: XdgActivationState,
    pub cursor_shape_state: CursorShapeManagerState,
    pub viewporter_state: ViewporterState,
    pub single_pixel_buffer_state: SinglePixelBufferState,
    pub wlr_data_control_state: WlrDataControlState,
    pub ext_data_control_state: ExtDataControlState,
    pub idle_notifier_state: IdleNotifierState<Hazel>,
    pub idle_inhibit_state: IdleInhibitManagerState,
    pub keyboard_shortcuts_inhibit_state: KeyboardShortcutsInhibitState,
    pub xdg_system_bell_state: XdgSystemBellState,
    pub fractional_scale_state: FractionalScaleManagerState,
    pub presentation_state: PresentationState,
    pub foreign_toplevels: ForeignToplevelState,
    pub foreign_toplevel_list_state: ForeignToplevelListState,
    pub screencopy: ScreencopyState,
    pub image_capture: ImageCaptureState,
    pub dmabuf_state: DmabufState,
    pub relative_pointer_state: RelativePointerManagerState,
    pub pointer_constraints_state: PointerConstraintsState,
    pub session_lock_state: SessionLockManagerState,
    pub pointer_gestures_state: PointerGesturesState,
    pub text_input_state: TextInputManagerState,
    pub input_method_state: InputMethodManagerState,
}

impl HazelCompositor {
    pub fn new(event_loop: &mut HazelEventLoop, dh: &DisplayHandle) -> Result<Self, Box<dyn std::error::Error>> {
        let compositor_state = CompositorState::new::<Hazel>(dh);
        let xdg_shell_state = XdgShellState::new::<Hazel>(dh);
        let layer_shell_state = WlrLayerShellState::new::<Hazel>(dh);
        let shm_state = ShmState::new::<Hazel>(dh, vec![]);
        let popups = PopupManager::default();
        let kde_decoration_state = KdeDecorationState::new::<Hazel>(dh, Mode::Client);
        let output_manager_state = OutputManagerState::new_with_xdg_output::<Hazel>(dh);
        let data_device_state = DataDeviceState::new::<Hazel>(dh);
        let seat_state = SeatState::new();
        let xdg_decoration_state = XdgDecorationState::new::<Hazel>(dh);
        let primary_selection_state = PrimarySelectionState::new::<Hazel>(dh);
        let xdg_activation_state = XdgActivationState::new::<Hazel>(dh);
        let cursor_shape_state = CursorShapeManagerState::new::<Hazel>(dh);
        let viewporter_state = ViewporterState::new::<Hazel>(dh);
        let single_pixel_buffer_state = SinglePixelBufferState::new::<Hazel>(dh);
        let wlr_data_control_state =
            WlrDataControlState::new::<Hazel, _>(dh, Some(&primary_selection_state), |_| true);
        let ext_data_control_state =
            ExtDataControlState::new::<Hazel, _>(dh, Some(&primary_selection_state), |_| true);
        let idle_notifier_state = IdleNotifierState::new(dh, event_loop.handle());
        let idle_inhibit_state = IdleInhibitManagerState::new::<Hazel>(dh);
        let keyboard_shortcuts_inhibit_state = KeyboardShortcutsInhibitState::new::<Hazel>(dh);
        let xdg_system_bell_state = XdgSystemBellState::new::<Hazel>(dh);
        let fractional_scale_state = FractionalScaleManagerState::new::<Hazel>(dh);
        let foreign_toplevels = ForeignToplevelState::new(dh);
        let screencopy = ScreencopyState::new(dh);
        let presentation_state = PresentationState::new::<Hazel>(dh, Monotonic::ID as u32);

        let space = Space::default();

        let socket_name = Self::init_wayland_listener(event_loop)?;

        Ok(Self {
            loop_handle: event_loop.handle(),
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
                xdg_decoration_state,
                primary_selection_state,
                xdg_activation_state,
                cursor_shape_state,
                viewporter_state,
                single_pixel_buffer_state,
                wlr_data_control_state,
                ext_data_control_state,
                idle_notifier_state,
                idle_inhibit_state,
                keyboard_shortcuts_inhibit_state,
                xdg_system_bell_state,
                fractional_scale_state,
                presentation_state,
                foreign_toplevels,
                foreign_toplevel_list_state: ForeignToplevelListState::new::<Hazel>(dh),
                screencopy,
                image_capture: ImageCaptureState::new(dh),
                dmabuf_state: DmabufState::new(),
                relative_pointer_state: RelativePointerManagerState::new::<Hazel>(dh),
                pointer_constraints_state: PointerConstraintsState::new::<Hazel>(dh),
                session_lock_state: SessionLockManagerState::new::<Hazel, _>(dh, |_| true),
                pointer_gestures_state: PointerGesturesState::new::<Hazel>(dh),
                text_input_state: TextInputManagerState::new::<Hazel>(dh),
                input_method_state: InputMethodManagerState::new::<Hazel, _>(dh, |_| true),
            },

            seats: HashMap::new(),
            device_to_seat: HashMap::new(),
            pointers: HashMap::new(),
            screenshot_requests: Vec::new(),
            clipboard: ClipboardState::default(),
            key_repeat: KeyRepeat::default(),
            session_lock: SessionLock::default(),
            listed_toplevels: Vec::new(),
            pending_dmabuf_imports: Vec::new(),
            hidden_windows: Vec::new(),
            pending_focus_changes: Vec::new(),
            idle_inhibitors: Vec::new(),
            presentation_clock: Clock::new(),
            presentation_sequence: 0,
            drawing: Drawing::default(),
        })
    }

    fn init_wayland_listener(event_loop: &mut HazelEventLoop) -> Result<OsString, Box<dyn std::error::Error>> {
        let listening_socket = ListeningSocketSource::new_auto()?;
        let socket_name = listening_socket.socket_name().to_os_string();
        let loop_handle = event_loop.handle();

        loop_handle
            .insert_source(listening_socket, move |client_stream, _, state| {
                info!("New client connection");
                let client = state
                    .display_handle
                    .insert_client(client_stream, Arc::new(ClientState::default()));
                if let Err(error) = client {
                    error!("Could not accept a client connection: {error}");
                }
            })
            .map_err(|error| error.to_string())?;

        Ok(socket_name)
    }

    /// The screenshots asked for this output; one asked for no output in particular goes to the main output of the backend.
    pub fn take_screenshots(&mut self, output: &Output, is_main: bool) -> Vec<PathBuf> {
        let (wanted, others): (Vec<_>, Vec<_>) = std::mem::take(&mut self.screenshot_requests).into_iter().partition(|request| match &request.output {
            Some(weak) => weak.upgrade().as_ref() == Some(output),
            None => is_main,
        });
        self.screenshot_requests = others;
        wanted.into_iter().map(|request| request.path).collect()
    }

    pub fn get_seat_of(&self, device_id: &str) -> Option<Seat<Hazel>> {
        self.device_to_seat
            .get(device_id)
            .and_then(|seat_name| self.seats.get(seat_name).cloned())
    }

    pub fn seat_of_pointer(&self, pointer: &PointerHandle<Hazel>) -> Option<Seat<Hazel>> {
        self.seats
            .values()
            .find(|seat| seat.get_pointer().as_ref() == Some(pointer))
            .cloned()
    }

    pub fn modifiers_of_pointer(&self, pointer: &PointerHandle<Hazel>) -> ModifiersState {
        self.seat_of_pointer(pointer)
            .and_then(|seat| seat.get_keyboard())
            .map(|keyboard| keyboard.modifier_state())
            .unwrap_or_default()
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
        if self.session_lock.is_active() {
            return self
                .session_lock
                .surface_at(pos, |output| self.space.output_geometry(output));
        }
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
