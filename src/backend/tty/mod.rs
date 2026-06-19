use std::{collections::HashMap, path::Path};

use smithay::{
    backend::{
        allocator::gbm::{GbmAllocator, GbmBufferFlags, GbmDevice},
        drm::{DrmDevice, DrmDeviceFd, DrmNode, NodeType},
        egl::{EGLDevice, EGLDisplay, context::ContextPriority},
        libinput::{LibinputInputBackend, LibinputSessionInterface},
        renderer::{
            ImportEgl,
            gles::GlesRenderer,
            multigpu::{GpuManager, gbm::GbmGlesBackend},
        },
        session::{self, Session, libseat::LibSeatSession},
        udev::{UdevBackend, UdevEvent, all_gpus, primary_gpu},
    },
    reexports::{
        calloop::{
            LoopHandle,
            timer::{TimeoutAction, Timer},
        },
        drm::control::crtc,
        input::Libinput,
        rustix::fs::OFlags,
        wayland_server::DisplayHandle,
    },
    utils::DeviceFd,
};
use smithay::{
    backend::{
        allocator::{Format, Fourcc, Modifier},
        drm::{DrmEvent, compositor::DrmCompositor, exporter::gbm::GbmFramebufferExporter},
    },
    output::{Mode as OutputMode, Output, OutputModeSource, PhysicalProperties},
    reexports::drm::control::{Mode as DrmMode, ModeTypeFlags},
};
use smithay_drm_extras::drm_scanner::{DrmScanEvent, DrmScanner};
use tracing::{debug, error, info, warn};

use crate::backend::tty::output_device::Surface;

const SUPPORTED_COLOR_FORMATS: [Fourcc; 4] = [
    Fourcc::Xrgb8888,
    Fourcc::Xbgr8888,
    Fourcc::Argb8888,
    Fourcc::Abgr8888,
];

use crate::{
    backend::{Backend, tty::output_device::OutputDevice},
    core::{GlobalHazel, Hazel, HazelEventLoop},
};

pub mod drm_handlers;
pub mod output_device;

pub struct HazelTtyBackend {
    pub session: LibSeatSession,
    pub gpu_manager: GpuManager<GbmGlesBackend<GlesRenderer, DrmDeviceFd>>,
    pub primary_gpu: DrmNode,
    pub output_devices: HashMap<DrmNode, OutputDevice>,
}

impl HazelTtyBackend {
    pub fn new(
        display_handle: &DisplayHandle,
        event_loop: &mut HazelEventLoop,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let (session, session_notifier) = LibSeatSession::new()?;
        let seat = &session.seat();
        let udev = UdevBackend::new(seat)?;
        let mut libinput = Libinput::new_with_udev::<LibinputSessionInterface<LibSeatSession>>(
            session.clone().into(),
        );
        libinput
            .udev_assign_seat(seat)
            .map_err(|_| "Failed to assign seat to libinput")?;
        let input_backend = LibinputInputBackend::new(libinput.clone());

        // Session

        event_loop
            .handle()
            .insert_source(session_notifier, move |event, _, _| match event {
                session::Event::ActivateSession => {
                    if libinput.resume().is_err() {
                        error!("Failed to resume libinput!");
                    };

                    info!("Session activated");
                }
                session::Event::PauseSession => {
                    libinput.suspend();
                    info!("Session paused");
                }
            })?;

        // Input

        event_loop
            .handle()
            .insert_source(input_backend, move |event, _, state| {
                if let Err(e) = GlobalHazel::execute(state, |hazel| hazel.process_input(event)) {
                    error!("Error processing input event: {}", e);
                }
            })?;

        // Compositing

        let primary_gpu = primary_gpu(seat)?
            .and_then(|x| {
                DrmNode::from_path(x)
                    .ok()?
                    .node_with_type(NodeType::Render)?
                    .ok()
            })
            .unwrap_or_else(|| {
                all_gpus(seat)
                    .unwrap()
                    .into_iter()
                    .find_map(|x| DrmNode::from_path(x).ok())
                    .expect("No GPU!")
            });

        info!("Using GPU: {}", primary_gpu);

        let primary_node = primary_gpu
            .node_with_type(NodeType::Primary)
            .and_then(|node| node.ok());

        // Collect device list before udev is moved into the event source below.
        let device_list: Vec<(u64, std::path::PathBuf)> = udev
            .device_list()
            .map(|(id, path)| (id, path.to_owned()))
            .collect();

        let primary_device = device_list
            .iter()
            .find(|(device_id, _)| {
                primary_node
                    .map(|primary_node| *device_id == primary_node.dev_id())
                    .unwrap_or(false)
                    || *device_id == primary_gpu.dev_id()
            })
            .map(|(id, path)| (*id, path.as_path()));

        let api = GbmGlesBackend::with_context_priority(ContextPriority::High);
        let gpu_manager = GpuManager::new(api)?;

        // Udev

        event_loop
            .handle()
            .insert_source(udev, move |event, _, _| match event {
                UdevEvent::Added { device_id, path } => {
                    info!("[UDEV] Device added: {} at path {:?}", device_id, path);
                }
                UdevEvent::Changed { device_id } => {
                    info!("[UDEV] Device changed: {}", device_id);
                }
                UdevEvent::Removed { device_id } => {
                    info!("[UDEV] Device removed: {}", device_id);
                }
            })?;

        // Done

        info!("Initialized Udev backend");

        let mut this = Self {
            session,
            gpu_manager,
            primary_gpu,
            output_devices: HashMap::new(),
        };

        let loop_handle = event_loop.handle();
        if let Some((device_id, path)) = primary_device
            && let Err(e) = this.on_device_added(&loop_handle, device_id, path)
        {
            error!("Failed to initialize primary DRM device: {e}");
        }

        let primary_device_id = primary_device.map(|(device_id, _)| device_id);
        for (device_id, path) in &device_list {
            if Some(*device_id) == primary_device_id {
                continue;
            }

            let Ok(_node) = DrmNode::from_dev_id(*device_id) else {
                error!("Skipping device {}: failed to get DRM node", device_id);
                continue;
            };

            if let Err(e) = this.on_device_added(&loop_handle, *device_id, path) {
                error!("Error adding device {}: {e}", device_id);
            }
        }

        if let Ok(mut renderer) = this.gpu_manager.single_renderer(&this.primary_gpu) {
            if let Err(e) = renderer.bind_wl_display(display_handle) {
                warn!("Failed to bind renderer to display: {e}");
            }
            info!("Initialized renderer");
        } else {
            warn!("No renderer available for primary GPU after device init");
        }

        Ok(this)
    }

    pub fn on_device_added(
        &mut self,
        loop_handle: &LoopHandle<Hazel>,
        device_id: u64,
        path: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let node = DrmNode::from_dev_id(device_id)?;

        if node.ty() != NodeType::Primary {
            debug!("Device {device_id} is not a primary node, skipping");
            return Ok(());
        }

        let openflags = OFlags::RDWR | OFlags::CLOEXEC | OFlags::NOCTTY | OFlags::NONBLOCK;
        let fd = self.session.open(path, openflags)?;
        let device_fd = DrmDeviceFd::new(DeviceFd::from(fd));

        let (drm, drm_notifier) = DrmDevice::new(device_fd.clone(), true)?;
        let gbm = GbmDevice::new(device_fd)?;

        let mut try_initialize_gpu = || {
            let display = unsafe { EGLDisplay::new(gbm.clone())? };
            let egl_device = EGLDevice::device_for_display(&display)?;

            let render_node = egl_device
                .try_get_render_node()
                .ok()
                .flatten()
                .unwrap_or(node);

            self.gpu_manager
                .as_mut()
                .add_node(render_node, gbm.clone())?;

            Ok::<_, Box<dyn std::error::Error>>(render_node)
        };

        let render_node = match try_initialize_gpu() {
            Ok(render_node) => {
                debug!("got render node: {render_node}");
                Some(render_node)
            }
            Err(err) => {
                debug!("failed to initialize renderer, falling back to primary gpu: {err:?}");
                None
            }
        };

        let allocator_gbm = if render_node.is_some() {
            gbm.clone()
        } else {
            panic!("no render node available for device {node}");
        };
        let gbm_flags = GbmBufferFlags::RENDERING | GbmBufferFlags::SCANOUT;
        let allocator = GbmAllocator::new(allocator_gbm, gbm_flags);

        let handle = loop_handle.clone();
        let token = loop_handle.insert_source(drm_notifier, move |event, meta, state| {
            match event {
                DrmEvent::VBlank(crtc) => {
                    let meta = meta.expect("VBlank events must have metadata");
                    let tty = match &mut state.backend {
                        Backend::Udev(tty) => tty,
                        _ => unreachable!(),
                    };
                    tty.on_vblank(node, crtc, meta);

                    // Schedule the next render after this VBlank.
                    let h = handle.clone();
                    let h_clone = h.clone();
                    let timer = Timer::immediate();
                    let _ = h.insert_source(timer, move |_, _, state: &mut Hazel| {
                        state.render_tty_output(node, crtc, h_clone.clone());
                        TimeoutAction::Drop
                    });
                }
                DrmEvent::Error(error) => warn!("DRM error: {error}"),
            }
        })?;

        let device = OutputDevice {
            token,
            render_node,
            drm,
            gbm,
            allocator,
            drm_scanner: DrmScanner::new(),
            surfaces: HashMap::new(),
        };

        self.output_devices.insert(node, device);

        self.scan_device_connectors(node);

        // Kickstart each surface with an immediate timer.
        let crtcs: Vec<crtc::Handle> = self
            .output_devices
            .get(&node)
            .map(|d| d.surfaces.keys().copied().collect())
            .unwrap_or_default();
        for &crtc in &crtcs {
            let h = loop_handle.clone();
            let timer = Timer::immediate();
            let _ = loop_handle.insert_source(timer, move |_, _, state: &mut Hazel| {
                state.render_tty_output(node, crtc, h.clone());
                TimeoutAction::Drop
            });
        }

        Ok(())
    }

    pub fn scan_device_connectors(&mut self, node: DrmNode) {
        let Some(device) = self.output_devices.get_mut(&node) else {
            error!("scan_device_connectors: device not found");
            return;
        };

        let scan_result = match device.drm_scanner.scan_connectors(&device.drm) {
            Ok(r) => r,
            Err(e) => {
                error!("Error scanning connectors on device {node:?}: {e}");
                return;
            }
        };

        for event in scan_result {
            match event {
                DrmScanEvent::Connected {
                    connector,
                    crtc: Some(crtc),
                } => {
                    let connector_name = format_connector_name(&connector);
                    info!("Connector connected: {} (CRTC: {crtc:?})", connector_name);

                    let Some(drm_mode) = pick_mode(&connector) else {
                        error!("No valid mode for connector {connector_name}");
                        continue;
                    };

                    info!(
                        "Picked mode: {}x{}@{}",
                        drm_mode.size().0,
                        drm_mode.size().1,
                        drm_mode.vrefresh()
                    );

                    let (physical_width, physical_height) = connector.size().unwrap_or((0, 0));
                    let output = Output::new(
                        connector_name.clone(),
                        PhysicalProperties {
                            size: (physical_width as i32, physical_height as i32).into(),
                            subpixel: connector.subpixel().into(),
                            make: "Unknown".to_owned(),
                            model: "Unknown".to_owned(),
                        },
                    );

                    let wl_mode = OutputMode::from(drm_mode);
                    output.change_current_state(Some(wl_mode), None, None, None);
                    output.set_preferred(wl_mode);

                    let drm_surface =
                        match device
                            .drm
                            .create_surface(crtc, drm_mode, &[connector.handle()])
                        {
                            Ok(s) => s,
                            Err(e) => {
                                error!("Failed to create DRM surface: {e}");
                                continue;
                            }
                        };

                    let render_formats = SUPPORTED_COLOR_FORMATS.iter().map(|&code| Format {
                        code,
                        modifier: Modifier::Invalid,
                    });

                    let compositor = match DrmCompositor::new(
                        OutputModeSource::Auto(output.clone()),
                        drm_surface,
                        None, // auto-detect planes
                        device.allocator.clone(),
                        GbmFramebufferExporter::new(device.gbm.clone(), device.render_node),
                        SUPPORTED_COLOR_FORMATS,
                        render_formats,
                        device.drm.cursor_size(),
                        Some(device.gbm.clone()),
                    ) {
                        Ok(c) => c,
                        Err(e) => {
                            error!("Failed to create DRM compositor: {e:?}");
                            continue;
                        }
                    };

                    let surface = Surface {
                        compositor,
                        connector: connector.handle(),
                        output: output.clone(),
                    };

                    device.surfaces.insert(crtc, surface);
                    info!("Output created for connector {connector_name}");
                }
                DrmScanEvent::Connected { .. } => {
                    info!("Connector connected but no CRTC assigned yet");
                }
                DrmScanEvent::Disconnected { connector, crtc } => {
                    let connector_name = format_connector_name(&connector);
                    info!(
                        "Connector disconnected: {} (CRTC: {crtc:?})",
                        connector_name
                    );

                    if let Some(crtc) = crtc
                        && let Some(mut surface) = device.surfaces.remove(&crtc)
                        && let Err(e) = surface.compositor.clear()
                    {
                        error!("Error clearing compositor for {connector_name}: {e}");
                    }
                }
            }
        }
    }
}

fn pick_mode(connector: &smithay::reexports::drm::control::connector::Info) -> Option<DrmMode> {
    let modes = connector.modes();
    modes
        .iter()
        .find(|m| m.mode_type().contains(ModeTypeFlags::PREFERRED))
        .cloned()
        .or_else(|| modes.iter().next().cloned())
}

fn format_connector_name(connector: &smithay::reexports::drm::control::connector::Info) -> String {
    format!(
        "{}-{}",
        connector.interface().as_str(),
        connector.interface_id()
    )
}
