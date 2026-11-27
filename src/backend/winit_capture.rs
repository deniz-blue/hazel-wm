use std::error::Error as StdError;

use smithay::{
    backend::{
        allocator::{Fourcc, dmabuf::Dmabuf},
        renderer::{
            Bind, Blit, ExportMem, Offscreen, TextureFilter,
            damage::OutputDamageTracker,
            gles::{GlesRenderer, GlesTarget, GlesTexture},
        },
    },
    output::Output,
    utils::{Buffer, Physical, Rectangle, Size},
};
use tracing::{error, info, warn};

use super::winit::CLEAR_COLOR;
use crate::core::{
    Hazel,
    capture::{
        CaptureRequest,
        shm::copy_region_to_shm,
        target::CaptureTarget,
    },
    screenshot::write_png,
};

fn blit_to_dmabuf(
    renderer: &mut GlesRenderer,
    rendered: &GlesTarget<'_>,
    dmabuf: &Dmabuf,
    region: Rectangle<i32, Physical>,
) -> Result<(), Box<dyn StdError>> {
    let mut dmabuf = dmabuf.clone();
    let mut destination = renderer.bind(&mut dmabuf)?;
    let to = Rectangle::from_size(region.size);
    renderer.blit(rendered, &mut destination, region, to, TextureFilter::Nearest)?;
    Ok(())
}

fn read_pixels(
    renderer: &mut GlesRenderer,
    rendered: &GlesTarget<'_>,
    size: Size<i32, Physical>,
) -> Result<Vec<u8>, Box<dyn StdError>> {
    let region = Rectangle::<i32, Buffer>::from_size((size.w, size.h).into());
    let mapping = renderer.copy_framebuffer(rendered, region, Fourcc::Abgr8888)?;
    Ok(renderer.map_texture(&mapping)?.to_vec())
}

struct Served {
    failed_requests: Vec<CaptureRequest>,
    finished_requests: Vec<CaptureRequest>,
    pixels: Option<Vec<u8>>,
}

fn serve(
    state: &Hazel,
    renderer: &mut GlesRenderer,
    output: &Output,
    size: Size<i32, Physical>,
    requests: Vec<CaptureRequest>,
    wants_pixels: bool,
) -> Result<Served, Box<dyn StdError>> {
    let buffer_size = Size::<i32, Buffer>::from((size.w, size.h));
    let mut texture: GlesTexture = renderer.create_buffer(Fourcc::Abgr8888, buffer_size)?;
    let mut rendered = renderer.bind(&mut texture)?;
    let elements = state.compositor.output_elements(renderer, output, false);
    OutputDamageTracker::new(size, output.current_scale().fractional_scale(), output.current_transform())
        .render_output(renderer, &mut rendered, 0, &elements, CLEAR_COLOR)?;

    let needs_pixels = wants_pixels || requests.iter().any(|request| matches!(request.target, CaptureTarget::Shm(_)));
    let pixels = if needs_pixels { Some(read_pixels(renderer, &rendered, size)?) } else { None };

    let mut served = Served { failed_requests: Vec::new(), finished_requests: Vec::new(), pixels };
    for request in requests {
        let copied = match (&request.target, &served.pixels) {
            (CaptureTarget::Shm(buffer), Some(pixels)) => {
                copy_region_to_shm(buffer, request.region, size, pixels).map_err(Into::into)
            }
            (CaptureTarget::Dmabuf(dmabuf), _) => {
                blit_to_dmabuf(renderer, &rendered, dmabuf, request.region)
            }
            (CaptureTarget::Shm(_), None) => Err("the output was not read back".into()),
        };
        match copied {
            Ok(()) => served.finished_requests.push(request),
            Err(e) => {
                warn!("capture failed: {e}");
                served.failed_requests.push(request);
            }
        }
    }
    if !served.finished_requests.is_empty() {
        renderer.with_context(|gl| unsafe { gl.Finish() })?;
    }
    Ok(served)
}

pub fn serve_capture_requests(
    state: &mut Hazel,
    renderer: &mut GlesRenderer,
    output: &Output,
    size: Size<i32, Physical>,
    is_main: bool,
) {
    let requests = state.take_capture_requests(output);
    let screenshots = state.compositor.take_screenshots(output, is_main);
    if requests.is_empty() && screenshots.is_empty() {
        return;
    }
    let served = match serve(state, renderer, output, size, requests, !screenshots.is_empty()) {
        Ok(served) => served,
        Err(e) => {
            error!("Capturing the output failed: {e}");
            state.fail_pending_captures();
            return;
        }
    };
    for request in served.failed_requests {
        state.fail_capture(request);
    }
    for request in served.finished_requests {
        state.finish_capture(request, output, size);
    }
    let Some(pixels) = served.pixels else {
        return;
    };
    for path in screenshots {
        match write_png(&path, size.w as u32, size.h as u32, &pixels) {
            Ok(()) => info!("Screenshot saved to {}", path.display()),
            Err(e) => error!("Screenshot to {} failed: {e}", path.display()),
        }
    }
}
