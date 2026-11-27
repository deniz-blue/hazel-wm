use smithay::{
    reexports::wayland_server::protocol::{wl_buffer::WlBuffer, wl_shm},
    utils::{Physical, Rectangle, Size},
    wayland::shm::{BufferAccessError, with_buffer_contents_mut},
};

pub const BYTES_PER_PIXEL: usize = 4;

pub fn check_shm_buffer(
    buffer: &WlBuffer,
    size: Size<i32, Physical>,
    formats: &[wl_shm::Format],
) -> Result<(), &'static str> {
    let result = with_buffer_contents_mut(buffer, |_, len, data| {
        let stride_ok = data.stride == size.w * BYTES_PER_PIXEL as i32;
        let fits = len >= (data.offset + data.stride * size.h) as usize;
        data.width == size.w && data.height == size.h && stride_ok && fits && formats.contains(&data.format)
    });
    match result {
        Ok(true) => Ok(()),
        Ok(false) => Err("buffer does not match the announced size, stride and format"),
        Err(BufferAccessError::NotManaged) => Err("only shm and dmabuf buffers are supported"),
        Err(_) => Err("the buffer cannot be read"),
    }
}

pub fn copy_region_to_shm(
    buffer: &WlBuffer,
    region: Rectangle<i32, Physical>,
    size: Size<i32, Physical>,
    pixels: &[u8],
) -> Result<(), BufferAccessError> {
    with_buffer_contents_mut(buffer, |target, len, data| {
        // SAFETY: with_buffer_contents_mut hands out a pointer to `len` bytes of the pool.
        let pool = unsafe { std::slice::from_raw_parts_mut(target, len) };
        let layout = BufferLayout { offset: data.offset as usize, stride: data.stride as usize };
        write_region_as_argb(pool, layout, pixels, size, region);
    })
}

struct BufferLayout {
    offset: usize,
    stride: usize,
}

fn write_region_as_argb(
    pool: &mut [u8],
    layout: BufferLayout,
    rgba: &[u8],
    size: Size<i32, Physical>,
    region: Rectangle<i32, Physical>,
) {
    let source_stride = size.w as usize * BYTES_PER_PIXEL;
    let row_bytes = region.size.w as usize * BYTES_PER_PIXEL;
    for row in 0..region.size.h as usize {
        let from = (region.loc.y as usize + row) * source_stride + region.loc.x as usize * BYTES_PER_PIXEL;
        let to = layout.offset + row * layout.stride;
        let source = rgba[from..from + row_bytes].chunks_exact(BYTES_PER_PIXEL);
        let destination = pool[to..to + row_bytes].chunks_exact_mut(BYTES_PER_PIXEL);
        for (out, pixel) in destination.zip(source) {
            out.copy_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: i32, y: i32, w: i32, h: i32) -> Rectangle<i32, Physical> {
        Rectangle::new((x, y).into(), (w, h).into())
    }

    #[test]
    fn pixels_are_written_as_argb_rows_of_the_region() {
        let rgba: Vec<u8> = (0..16u8).flat_map(|i| [i, 0, 100 + i, 255]).collect();
        let mut pool = vec![0u8; 16 + 2 * 12];
        let layout = BufferLayout { offset: 16, stride: 12 };
        write_region_as_argb(&mut pool, layout, &rgba, (4, 4).into(), rect(1, 2, 2, 2));
        assert_eq!(pool[..16], [0; 16]);
        assert_eq!(pool[16..24], [109, 0, 9, 255, 110, 0, 10, 255]);
        assert_eq!(pool[28..36], [113, 0, 13, 255, 114, 0, 14, 255]);
    }
}
