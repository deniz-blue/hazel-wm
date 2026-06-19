use std::{collections::HashMap, path::Path};

use smithay::{
    backend::{
        allocator::gbm::{GbmAllocator, GbmBufferFlags, GbmDevice},
        drm::{DrmDevice, DrmDeviceFd, DrmEvent, DrmNode, NodeType},
        egl::{EGLContext, EGLDevice, EGLDisplay, context::ContextPriority},
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
        calloop::LoopHandle, input::Libinput, rustix::fs::OFlags, wayland_server::DisplayHandle,
    },
    utils::DeviceFd,
};
use tracing::{debug, error, info, warn};

use crate::{
    backend::tty::output_device::OutputDevice,
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
                    if let Err(_) = libinput.resume() {
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

        let primary_device = udev.device_list().find(|(device_id, _)| {
            primary_node
                .map(|primary_node| *device_id == primary_node.dev_id())
                .unwrap_or(false)
                || *device_id == primary_gpu.dev_id()
        });

        // if let Some((device_id, path)) = primary_device {
        //     let node = DrmNode::from_dev_id(device_id).expect("failed to get primary node");
        //     state
        //         .device_added(node, path)
        //         .expect("failed to initialize primary node");
        // }

        let primary_device_id = primary_device.map(|(device_id, _)| device_id);
        for (device_id, _path) in udev.device_list() {
            if Some(device_id) == primary_device_id {
                continue;
            }

            let Ok(_node) = DrmNode::from_dev_id(device_id) else {
                error!("Skipping device {device_id}: failed to get DRM node");
                continue;
            };

            // TODO: device_added
        }

        let api = GbmGlesBackend::with_context_priority(ContextPriority::High);
        let mut gpu_manager = GpuManager::new(api)?;

        let mut renderer = gpu_manager.single_renderer(&primary_gpu)?;

        renderer.bind_wl_display(display_handle)?;

        info!("Initialized renderer");

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

        Ok(Self {
            session,
            gpu_manager,
            primary_gpu,
            output_devices: HashMap::new(),
        })
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
        let fd = self.session.open(&path, openflags)?;
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

        // if render_node == Some(self.primary_render_node) && self.dmabuf_global.is_none() {
        //     let render_node = self.primary_render_node;
        //     debug!("initializing the primary renderer");

        //     let mut renderer = self
        //         .gpu_manager
        //         .single_renderer(&render_node)
        //         .context("error creating renderer")?;

        //     if let Err(err) = renderer.bind_wl_display(&niri.display_handle) {
        //         // wl_drm is on its way out so this is expected on most modern distros.
        //         trace!("error binding legacy EGL to wl_display: {err}");
        //     } else {
        //         debug!("bound legacy EGL to wl_display");
        //     }

        //     let gles_renderer = renderer.as_gles_renderer();
        //     resources::init(gles_renderer);
        //     shaders::init(gles_renderer);

        //     let config = self.config.borrow();
        //     if let Some(src) = config.animations.window_resize.custom_shader.as_deref() {
        //         shaders::set_custom_resize_program(gles_renderer, Some(src));
        //     }
        //     if let Some(src) = config.animations.window_close.custom_shader.as_deref() {
        //         shaders::set_custom_close_program(gles_renderer, Some(src));
        //     }
        //     if let Some(src) = config.animations.window_open.custom_shader.as_deref() {
        //         shaders::set_custom_open_program(gles_renderer, Some(src));
        //     }
        //     drop(config);

        //     niri.update_shaders();

        //     // Create the dmabuf global.
        //     let primary_formats = renderer.dmabuf_formats();
        //     let default_feedback =
        //         DmabufFeedbackBuilder::new(render_node.dev_id(), primary_formats.clone())
        //             .build()
        //             .context("error building default dmabuf feedback")?;
        //     let dmabuf_global = niri
        //         .dmabuf_state
        //         .create_global_with_default_feedback::<State>(
        //             &niri.display_handle,
        //             &default_feedback,
        //         );
        //     assert!(self.dmabuf_global.replace(dmabuf_global).is_none());

        //     // Update the dmabuf feedbacks for all surfaces.
        //     for (node, device) in self.devices.iter_mut() {
        //         for surface in device.surfaces.values_mut() {
        //             match surface_dmabuf_feedback(
        //                 &surface.compositor,
        //                 primary_formats.clone(),
        //                 self.primary_render_node,
        //                 device.render_node,
        //                 *node,
        //             ) {
        //                 Ok(feedback) => {
        //                     surface.dmabuf_feedback = Some(feedback);
        //                 }
        //                 Err(err) => {
        //                     warn!("error building dmabuf feedback: {err:?}");
        //                 }
        //             }
        //         }
        //     }
        // }

        let allocator_gbm = if render_node.is_some() {
            gbm.clone()
        } else {
            panic!("no render node available for device {node}");
        };

        // } else if let Some(primary_device) = self.devices.get(&self.primary_node) {
        //     primary_device.gbm.clone()
        // } else {
        //     bail!("no allocator available for device");
        // };
        let gbm_flags = GbmBufferFlags::RENDERING | GbmBufferFlags::SCANOUT;
        let allocator = GbmAllocator::new(allocator_gbm, gbm_flags);

        let token = loop_handle.insert_source(drm_notifier, move |event, meta, state| {
            state.backend.as_tty().on_drm_event(event, node, meta);
        })?;

        // let drm_lease_state = DrmLeaseState::new::<State>(&niri.display_handle, &node)
        //     .map_err(|err| warn!("error initializing DRM leasing for {node}: {err:?}"))
        //     .ok();

        let device = OutputDevice {
            token,
            render_node,
            drm,
            gbm,
            allocator,
        };

        self.output_devices.insert(node, device);

        Ok(())
    }
}
