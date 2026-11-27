use std::collections::HashMap;

use smithay::{
    backend::{
        allocator::Fourcc,
        renderer::{
            ImportAll, ImportMem,
            element::{
                Kind,
                memory::{MemoryRenderBuffer, MemoryRenderBufferRenderElement},
                solid::{SolidColorBuffer, SolidColorRenderElement},
            },
        },
    },
    utils::{Logical, Physical, Point, Rectangle, Scale, Size, Transform},
};
use tracing::warn;

use crate::core::render::HazelElement;

use super::{
    pixels::{MAXIMUM_PIXELS, RoundedRectangle, logical_length_holding, physical_length},
    shape::{RectangleStyle, ShapeContent, TextStyle},
    text::TextEngine,
};

pub enum Prepared {
    Nothing,
    Solid { buffer: SolidColorBuffer },
    Bitmap { buffer: MemoryRenderBuffer, logical: Size<i32, Logical>, physical: Size<i32, smithay::utils::Buffer> },
}

#[derive(Default)]
pub struct PreparedByScale(HashMap<u64, Prepared>);

impl PreparedByScale {
    pub fn clear(&mut self) {
        self.0.clear();
    }

    pub fn at_scale(&mut self, scale: f64, content: &ShapeContent, text: &mut TextEngine) -> &Prepared {
        self.0.entry(scale.to_bits()).or_insert_with(|| prepare(content, scale, text))
    }
}

fn bitmap(pixels: &[u8], logical: (i32, i32), physical: (u32, u32)) -> Prepared {
    let size = (physical.0 as i32, physical.1 as i32);
    Prepared::Bitmap {
        buffer: MemoryRenderBuffer::from_slice(pixels, Fourcc::Argb8888, size, 1, Transform::Normal, None),
        logical: Size::from(logical),
        physical: Size::from(size),
    }
}

fn too_many_pixels(width: u32, height: u32) -> bool {
    u64::from(width) * u64::from(height) > MAXIMUM_PIXELS
}

fn prepare_rectangle(style: &RectangleStyle, scale: f64) -> Prepared {
    let (width, height) = (physical_length(style.width, scale), physical_length(style.height, scale));
    if width == 0 || height == 0 {
        return Prepared::Nothing;
    }
    if style.radius == 0 && style.border_width == 0 {
        return match style.fill {
            Some(color) => Prepared::Solid { buffer: SolidColorBuffer::new((style.width, style.height), color) },
            None => Prepared::Nothing,
        };
    }
    if too_many_pixels(width, height) {
        warn!("not drawing a {}x{} rectangle at scale {scale}: too many pixels", style.width, style.height);
        return Prepared::Nothing;
    }
    let border_width = if style.border_width > 0 { (style.border_width as f32 * scale as f32).max(1.0) } else { 0.0 };
    let pixels = RoundedRectangle {
        width,
        height,
        radius: style.radius as f32 * scale as f32,
        border_width,
        fill: style.fill,
        border: style.border_color,
    }
    .pixels();
    bitmap(&pixels, (style.width, style.height), (width, height))
}

fn prepare_text(style: &TextStyle, scale: f64, engine: &mut TextEngine) -> Prepared {
    let max_width = style.max_width.map(|width| width as f32);
    let shown = match engine.measure(&style.text, &style.font, max_width) {
        Ok(metrics) => metrics.shown,
        Err(error) => {
            warn!("not drawing text: {error}");
            return Prepared::Nothing;
        }
    };
    let extent = match engine.extent(&shown, &style.font, scale as f32) {
        Ok(extent) if extent.0 >= 1.0 => extent,
        Ok(_) => return Prepared::Nothing,
        Err(error) => {
            warn!("not drawing text: {error}");
            return Prepared::Nothing;
        }
    };
    let (logical_width, width) = logical_length_holding(extent.0.ceil() as u32, scale);
    let (logical_height, height) = logical_length_holding(extent.1.ceil() as u32, scale);
    if too_many_pixels(width, height) {
        warn!("not drawing text at scale {scale}: too many pixels");
        return Prepared::Nothing;
    }
    match engine.rasterize(&shown, &style.font, style.color, scale as f32, (width, height)) {
        Ok(text) => bitmap(&text.pixels, (logical_width, logical_height), (text.width, text.height)),
        Err(error) => {
            warn!("not drawing text: {error}");
            Prepared::Nothing
        }
    }
}

fn prepare(content: &ShapeContent, scale: f64, engine: &mut TextEngine) -> Prepared {
    match content {
        ShapeContent::Rectangle(style) => prepare_rectangle(style, scale),
        ShapeContent::Text(style) => prepare_text(style, scale, engine),
    }
}

impl Prepared {
    pub fn element<R>(&self, renderer: &mut R, location: Point<i32, Physical>, scale: Scale<f64>) -> Option<HazelElement<R>>
    where
        R: ImportAll + ImportMem,
        R::TextureId: Send + Clone + 'static,
    {
        match self {
            Self::Nothing => None,
            Self::Solid { buffer } => {
                Some(HazelElement::Solid(SolidColorRenderElement::from_buffer(buffer, location, scale, 1.0, Kind::Unspecified)))
            }
            Self::Bitmap { buffer, logical, physical } => {
                let source = Rectangle::from_size(physical.to_f64().to_logical(1.0, Transform::Normal));
                MemoryRenderBufferRenderElement::from_buffer(
                    renderer,
                    location.to_f64(),
                    buffer,
                    None,
                    Some(source),
                    Some(*logical),
                    Kind::Unspecified,
                )
                .ok()
                .map(HazelElement::Memory)
            }
        }
    }
}
