use std::{fs::File, io::BufWriter, path::Path};

pub fn write_png(
    path: &Path,
    width: u32,
    height: u32,
    rgba_top_down: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut encoder = png::Encoder::new(BufWriter::new(File::create(path)?), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(rgba_top_down)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOP_DOWN: [u8; 16] = [
        0, 0, 255, 255, 0, 0, 255, 255, // blue row
        255, 0, 0, 255, 255, 0, 0, 255, // red row
    ];

    #[test]
    fn rows_are_written_in_the_order_given() {
        let path = std::env::temp_dir().join("hazel-screenshot-test.png");
        write_png(&path, 2, 2, &TOP_DOWN).unwrap();

        let decoder = png::Decoder::new(std::io::BufReader::new(File::open(&path).unwrap()));
        let mut reader = decoder.read_info().unwrap();
        let mut out = vec![0; reader.output_buffer_size()];
        reader.next_frame(&mut out).unwrap();
        assert_eq!(&out[..4], &[0, 0, 255, 255]);
        assert_eq!(&out[8..12], &[255, 0, 0, 255]);
        std::fs::remove_file(path).ok();
    }
}
