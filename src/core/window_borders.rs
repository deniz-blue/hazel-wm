use std::cell::RefCell;

use smithay::{
    backend::renderer::element::{Kind, solid::{SolidColorBuffer, SolidColorRenderElement}},
    desktop::Window,
    utils::{Logical, Physical, Point, Rectangle, Scale},
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BorderStyle {
    pub width: i32,
    pub color: [f32; 4],
}

#[derive(Default)]
struct WindowBorder {
    style: Option<BorderStyle>,
    edges: [SolidColorBuffer; 4],
}

pub fn parse_hex_color(text: &str) -> Option<[f32; 4]> {
    let digits = text.strip_prefix('#')?;
    if !digits.is_ascii() || !matches!(digits.len(), 6 | 8) {
        return None;
    }
    let channel = |index: usize| {
        let byte = u8::from_str_radix(digits.get(index * 2..index * 2 + 2)?, 16).ok()?;
        Some(f32::from(byte) / 255.0)
    };
    let alpha = if digits.len() == 8 { channel(3)? } else { 1.0 };
    Some([channel(0)?, channel(1)?, channel(2)?, alpha])
}

fn border_data(window: &Window) -> &RefCell<WindowBorder> {
    window.user_data().get_or_insert(RefCell::<WindowBorder>::default)
}

pub fn set_window_border(window: &Window, style: Option<BorderStyle>) {
    border_data(window).borrow_mut().style = style;
}

pub fn window_border(window: &Window) -> Option<BorderStyle> {
    border_data(window).borrow().style
}

pub const MAXIMUM_BORDER_WIDTH: i32 = 4096;

pub fn edge_rectangles(inner: Rectangle<i32, Logical>, width: i32) -> [Rectangle<i32, Logical>; 4] {
    let (x, y, w, h) = (inner.loc.x, inner.loc.y, inner.size.w, inner.size.h);
    let left = x.saturating_sub(width);
    let top = y.saturating_sub(width);
    let across = w.saturating_add(width.saturating_mul(2));
    [
        Rectangle::new((left, top).into(), (across, width).into()),
        Rectangle::new((left, y.saturating_add(h)).into(), (across, width).into()),
        Rectangle::new((left, y).into(), (width, h).into()),
        Rectangle::new((x.saturating_add(w), y).into(), (width, h).into()),
    ]
}

pub fn border_elements(
    window: &Window,
    inner: Rectangle<i32, Logical>,
    output_origin: Point<i32, Logical>,
    scale: Scale<f64>,
) -> Vec<SolidColorRenderElement> {
    let mut border = border_data(window).borrow_mut();
    let Some(BorderStyle { width, color }) = border.style.filter(|style| style.width > 0) else {
        return Vec::new();
    };
    let rectangles = edge_rectangles(inner, width);
    let mut elements = Vec::with_capacity(4);
    for (buffer, rectangle) in border.edges.iter_mut().zip(rectangles) {
        buffer.update(rectangle.size, color);
        let location: Point<i32, Physical> = (rectangle.loc - output_origin)
            .to_f64()
            .to_physical(scale)
            .to_i32_round();
        elements.push(SolidColorRenderElement::from_buffer(
            buffer,
            location,
            scale,
            1.0,
            Kind::Unspecified,
        ));
    }
    elements
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_colors_with_and_without_alpha() {
        assert_eq!(parse_hex_color("#ff0000"), Some([1.0, 0.0, 0.0, 1.0]));
        assert_eq!(parse_hex_color("#00ff0080"), Some([0.0, 1.0, 0.0, 128.0 / 255.0]));
    }

    #[test]
    fn edge_rectangles_at_the_edge_of_i32_do_not_overflow() {
        let inner = Rectangle::new((i32::MIN, i32::MAX - 5).into(), (i32::MAX, 10).into());
        edge_rectangles(inner, MAXIMUM_BORDER_WIDTH);
        edge_rectangles(inner, i32::MAX);
    }

    #[test]
    fn malformed_hex_colors_are_rejected() {
        for text in ["ff0000", "#fff", "#gg0000", "#ff00000", "#é00000", ""] {
            assert_eq!(parse_hex_color(text), None, "{text}");
        }
    }

    #[test]
    fn edges_surround_the_window_without_overlapping() {
        let inner = Rectangle::<i32, Logical>::new((100, 50).into(), (200, 100).into());
        let [top, bottom, left, right] = edge_rectangles(inner, 2);
        assert_eq!(top, Rectangle::new((98, 48).into(), (204, 2).into()));
        assert_eq!(bottom, Rectangle::new((98, 150).into(), (204, 2).into()));
        assert_eq!(left, Rectangle::new((98, 50).into(), (2, 100).into()));
        assert_eq!(right, Rectangle::new((300, 50).into(), (2, 100).into()));
    }
}
