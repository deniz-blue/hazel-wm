use smithay::{
    backend::{
        allocator::{
            Buffer, Format, Fourcc, Modifier,
            dmabuf::Dmabuf,
            format::FormatSet,
        },
    },
    reexports::wayland_server::protocol::{wl_buffer::WlBuffer, wl_shm},
    utils::{Physical, Size},
    wayland::dmabuf::get_dmabuf,
};

use super::shm::check_shm_buffer;

const PREFERRED_DMABUF_CODES: [Fourcc; 2] = [Fourcc::Xrgb8888, Fourcc::Argb8888];

#[derive(Debug, Clone)]
pub struct DmabufCaptureSupport {
    pub device: u64,
    pub formats: FormatSet,
}

impl DmabufCaptureSupport {
    pub fn codes(&self) -> Vec<Fourcc> {
        PREFERRED_DMABUF_CODES
            .into_iter()
            .filter(|code| self.formats.iter().any(|format| format.code == *code))
            .collect()
    }

    pub fn modifiers_for(&self, code: Fourcc) -> Vec<Modifier> {
        self.formats
            .iter()
            .filter(|format| format.code == code)
            .map(|format| format.modifier)
            .collect()
    }

    fn accepts(&self, dmabuf: &Dmabuf) -> bool {
        let format: Format = dmabuf.format();
        self.codes().contains(&format.code)
    }
}

#[derive(Debug, Clone)]
pub enum CaptureTarget {
    Shm(WlBuffer),
    Dmabuf(Dmabuf),
}

pub fn classify_buffer(
    buffer: &WlBuffer,
    size: Size<i32, Physical>,
    shm_formats: &[wl_shm::Format],
    dmabuf_support: Option<&DmabufCaptureSupport>,
) -> Result<CaptureTarget, &'static str> {
    let Ok(dmabuf) = get_dmabuf(buffer) else {
        return check_shm_buffer(buffer, size, shm_formats).map(|()| CaptureTarget::Shm(buffer.clone()));
    };
    let Some(support) = dmabuf_support else {
        return Err("dmabuf buffers are not supported");
    };
    let announced: Size<i32, smithay::utils::Buffer> = (size.w, size.h).into();
    if dmabuf.size() != announced {
        return Err("buffer does not match the announced size");
    }
    if !support.accepts(dmabuf) {
        return Err("buffer format was not announced");
    }
    Ok(CaptureTarget::Dmabuf(dmabuf.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn support() -> DmabufCaptureSupport {
        let formats = [
            (Fourcc::Argb8888, Modifier::Linear),
            (Fourcc::Argb8888, Modifier::Invalid),
            (Fourcc::Xrgb8888, Modifier::Linear),
            (Fourcc::Abgr16161616f, Modifier::Linear),
        ]
        .into_iter()
        .map(|(code, modifier)| Format { code, modifier })
        .collect();
        DmabufCaptureSupport { device: 1, formats }
    }

    #[test]
    fn only_preferred_codes_are_announced_in_preference_order() {
        assert_eq!(support().codes(), [Fourcc::Xrgb8888, Fourcc::Argb8888]);
    }

    #[test]
    fn modifiers_are_listed_per_code() {
        assert_eq!(support().modifiers_for(Fourcc::Argb8888), [Modifier::Linear, Modifier::Invalid]);
        assert_eq!(support().modifiers_for(Fourcc::Xrgb8888), [Modifier::Linear]);
        assert!(support().modifiers_for(Fourcc::Nv12).is_empty());
    }

    #[test]
    fn nothing_is_announced_when_no_format_is_renderable() {
        let none = DmabufCaptureSupport { device: 1, formats: FormatSet::default() };
        assert!(none.codes().is_empty());
    }
}
