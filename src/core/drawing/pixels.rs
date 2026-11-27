pub type Color = [f32; 4];

pub const BYTES_PER_PIXEL: usize = 4;
pub const MAXIMUM_PIXELS: u64 = 32 * 1024 * 1024;

pub fn physical_length(logical: i32, scale: f64) -> u32 {
    (f64::from(logical.max(0)) * scale).round() as u32
}

pub fn logical_length_holding(pixels: u32, scale: f64) -> (i32, u32) {
    let mut logical = (f64::from(pixels) / scale).ceil().min(f64::from(i32::MAX)) as i32;
    let mut physical = physical_length(logical, scale);
    if physical < pixels && logical < i32::MAX {
        logical += 1;
        physical = physical_length(logical, scale);
    }
    (logical, physical)
}

pub fn premultiplied_argb(color: Color, coverage: f32) -> [u8; 4] {
    let alpha = (color[3] * coverage).clamp(0.0, 1.0);
    let channel = |value: f32| (value.clamp(0.0, 1.0) * alpha * 255.0).round() as u8;
    [channel(color[2]), channel(color[1]), channel(color[0]), (alpha * 255.0).round() as u8]
}

pub fn blend_over(canvas: &mut [u8], offset: usize, source: [u8; 4]) {
    let inverse = 255 - u32::from(source[3]);
    for (destination, source) in canvas[offset..offset + 4].iter_mut().zip(source) {
        *destination = (u32::from(source) + (u32::from(*destination) * inverse + 127) / 255).min(255) as u8;
    }
}

fn distance_to_rounded_box(x: f32, y: f32, half: (f32, f32), radius: f32) -> f32 {
    let qx = x.abs() - half.0 + radius;
    let qy = y.abs() - half.1 + radius;
    qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - radius
}

fn coverage(distance: f32) -> f32 {
    (0.5 - distance).clamp(0.0, 1.0)
}

pub struct RoundedRectangle {
    pub width: u32,
    pub height: u32,
    pub radius: f32,
    pub border_width: f32,
    pub fill: Option<Color>,
    pub border: Color,
}

impl RoundedRectangle {
    pub fn pixels(&self) -> Vec<u8> {
        let (width, height) = (self.width as usize, self.height as usize);
        let half = (self.width as f32 / 2.0, self.height as f32 / 2.0);
        let radius = self.radius.clamp(0.0, half.0.min(half.1));
        let inner_half = (half.0 - self.border_width, half.1 - self.border_width);
        let inner_radius = (radius - self.border_width).max(0.0);
        let mut pixels = vec![0; width * height * BYTES_PER_PIXEL];
        for row in 0..height {
            for column in 0..width {
                let x = column as f32 + 0.5 - half.0;
                let y = row as f32 + 0.5 - half.1;
                let outer = coverage(distance_to_rounded_box(x, y, half, radius));
                let inner = if inner_half.0 > 0.0 && inner_half.1 > 0.0 {
                    coverage(distance_to_rounded_box(x, y, inner_half, inner_radius))
                } else {
                    0.0
                };
                let offset = (row * width + column) * BYTES_PER_PIXEL;
                if let Some(fill) = self.fill {
                    blend_over(&mut pixels, offset, premultiplied_argb(fill, inner));
                }
                blend_over(&mut pixels, offset, premultiplied_argb(self.border, (outer - inner).max(0.0)));
            }
        }
        pixels
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RED: Color = [1.0, 0.0, 0.0, 1.0];
    const BLUE: Color = [0.0, 0.0, 1.0, 1.0];

    fn pixel(pixels: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
        let offset = (y * width + x) as usize * BYTES_PER_PIXEL;
        pixels[offset..offset + 4].try_into().unwrap()
    }

    fn rectangle(radius: f32, border_width: f32, fill: Option<Color>) -> Vec<u8> {
        RoundedRectangle { width: 20, height: 10, radius, border_width, fill, border: BLUE }.pixels()
    }

    #[test]
    fn a_square_filled_rectangle_is_opaque_everywhere() {
        let pixels = rectangle(0.0, 0.0, Some(RED));
        assert!(pixels.chunks(4).all(|p| p == [0, 0, 255, 255]));
    }

    #[test]
    fn rounding_clears_the_corner_and_keeps_the_middle() {
        let pixels = rectangle(5.0, 0.0, Some(RED));
        assert_eq!(pixel(&pixels, 20, 0, 0)[3], 0);
        assert_eq!(pixel(&pixels, 20, 10, 5), [0, 0, 255, 255]);
        assert_eq!(pixel(&pixels, 20, 10, 0), [0, 0, 255, 255]);
    }

    #[test]
    fn a_border_surrounds_the_fill() {
        let pixels = rectangle(0.0, 2.0, Some(RED));
        assert_eq!(pixel(&pixels, 20, 0, 5), [255, 0, 0, 255]);
        assert_eq!(pixel(&pixels, 20, 1, 5), [255, 0, 0, 255]);
        assert_eq!(pixel(&pixels, 20, 2, 5), [0, 0, 255, 255]);
        assert_eq!(pixel(&pixels, 20, 10, 9), [255, 0, 0, 255]);
    }

    #[test]
    fn a_border_without_fill_leaves_the_inside_clear() {
        let pixels = rectangle(0.0, 1.0, None);
        assert_eq!(pixel(&pixels, 20, 0, 0), [255, 0, 0, 255]);
        assert_eq!(pixel(&pixels, 20, 10, 5), [0, 0, 0, 0]);
    }

    #[test]
    fn a_radius_past_half_the_height_makes_a_pill() {
        let pixels = rectangle(100.0, 0.0, Some(RED));
        assert_eq!(pixel(&pixels, 20, 0, 0)[3], 0);
        assert_eq!(pixel(&pixels, 20, 0, 9)[3], 0);
        assert_eq!(pixel(&pixels, 20, 10, 5)[3], 255);
    }

    #[test]
    fn a_border_wider_than_the_shape_fills_it_with_border_colour() {
        let pixels = rectangle(0.0, 50.0, Some(RED));
        assert!(pixels.chunks(4).all(|p| p == [255, 0, 0, 255]));
    }

    #[test]
    fn translucent_colours_are_stored_premultiplied() {
        assert_eq!(premultiplied_argb([1.0, 1.0, 1.0, 0.5], 1.0), [128, 128, 128, 128]);
    }

    #[test]
    fn logical_lengths_round_trip_through_the_renderer_rounding() {
        for scale in [1.0, 1.25, 1.5, 1.75, 2.0, 3.0] {
            for pixels in 0..200 {
                let (logical, physical) = logical_length_holding(pixels, scale);
                assert!(physical >= pixels, "{pixels} at {scale}");
                assert_eq!(physical, physical_length(logical, scale));
            }
        }
    }
}
