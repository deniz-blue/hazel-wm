use std::cell::RefCell;

use smithay::{desktop::Window, output::WeakOutput, wayland::shell::wlr_layer::Layer};

use super::{
    pixels::Color,
    prepared::PreparedByScale,
    text::{FontSource, FontSpec},
};

pub const MAXIMUM_SIZE: i32 = 16384;
pub const MAXIMUM_BORDER_WIDTH: i32 = 4096;
pub const MAXIMUM_FONT_SIZE: f32 = 1024.0;

#[derive(Clone, Debug)]
pub enum ShapeTarget {
    Output(WeakOutput),
    Window(Window),
}

#[derive(Clone, Debug, PartialEq)]
pub struct RectangleStyle {
    pub width: i32,
    pub height: i32,
    pub fill: Option<Color>,
    pub radius: i32,
    pub border_width: i32,
    pub border_color: Color,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextStyle {
    pub text: String,
    pub font: FontSpec,
    pub color: Color,
    pub max_width: Option<i32>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ShapeContent {
    Rectangle(RectangleStyle),
    Text(TextStyle),
}

impl ShapeContent {
    pub fn empty_rectangle() -> Self {
        Self::Rectangle(RectangleStyle { width: 0, height: 0, fill: None, radius: 0, border_width: 0, border_color: [0.0, 0.0, 0.0, 1.0] })
    }

    pub fn empty_text() -> Self {
        Self::Text(TextStyle { text: String::new(), font: FontSpec::default(), color: [1.0, 1.0, 1.0, 1.0], max_width: None })
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Self::Rectangle(_) => "rect",
            Self::Text(_) => "text",
        }
    }
}

pub struct Shape {
    pub id: u64,
    pub generation: u64,
    pub target: ShapeTarget,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub layer: Layer,
    pub visible: bool,
    pub content: ShapeContent,
    pub prepared: RefCell<PreparedByScale>,
}

#[derive(Default)]
pub struct FontChange {
    pub source: Option<FontSource>,
    pub size: Option<f32>,
    pub weight: Option<u16>,
}

#[derive(Default)]
pub struct ShapeChange {
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub z: Option<i32>,
    pub layer: Option<Layer>,
    pub visible: Option<bool>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub fill: Option<Option<Color>>,
    pub radius: Option<i32>,
    pub border_width: Option<i32>,
    pub border_color: Option<Color>,
    pub text: Option<String>,
    pub font: FontChange,
    pub color: Option<Color>,
    pub max_width: Option<Option<i32>>,
}

fn within(name: &str, value: i32, maximum: i32) -> Result<i32, String> {
    if (0..=maximum).contains(&value) {
        Ok(value)
    } else {
        Err(format!("{name} must be between 0 and {maximum}, got {value}"))
    }
}

fn not_for(kind: &str, fields: &[(&str, bool)]) -> Result<(), String> {
    match fields.iter().find(|(_, given)| *given) {
        Some((name, _)) => Err(format!("a {kind} shape has no {name}")),
        None => Ok(()),
    }
}

impl ShapeContent {
    fn apply(&mut self, change: &ShapeChange) -> Result<(), String> {
        let font_given = change.font.source.is_some() || change.font.size.is_some() || change.font.weight.is_some();
        match self {
            Self::Rectangle(style) => {
                not_for("rect", &[("text", change.text.is_some()), ("font", font_given), ("color", change.color.is_some()), ("max_width", change.max_width.is_some())])?;
                style.width = change.width.map(|v| within("width", v, MAXIMUM_SIZE)).transpose()?.unwrap_or(style.width);
                style.height = change.height.map(|v| within("height", v, MAXIMUM_SIZE)).transpose()?.unwrap_or(style.height);
                style.radius = change.radius.map(|v| within("radius", v, MAXIMUM_SIZE)).transpose()?.unwrap_or(style.radius);
                style.border_width = change.border_width.map(|v| within("border_width", v, MAXIMUM_BORDER_WIDTH)).transpose()?.unwrap_or(style.border_width);
                style.fill = change.fill.unwrap_or(style.fill);
                style.border_color = change.border_color.unwrap_or(style.border_color);
            }
            Self::Text(style) => {
                not_for("text", &[("width", change.width.is_some()), ("height", change.height.is_some()), ("fill", change.fill.is_some()), ("radius", change.radius.is_some()), ("border_width", change.border_width.is_some()), ("border_color", change.border_color.is_some())])?;
                if let Some(size) = change.font.size {
                    if !(size > 0.0 && size <= MAXIMUM_FONT_SIZE) {
                        return Err(format!("font size must be above 0 and at most {MAXIMUM_FONT_SIZE}, got {size}"));
                    }
                    style.font.size = size;
                }
                style.font.source = change.font.source.clone().unwrap_or_else(|| style.font.source.clone());
                style.font.weight = change.font.weight.unwrap_or(style.font.weight);
                style.text = change.text.clone().unwrap_or_else(|| style.text.clone());
                style.color = change.color.unwrap_or(style.color);
                style.max_width = match change.max_width {
                    Some(Some(width)) => Some(within("max_width", width, MAXIMUM_SIZE)?),
                    Some(None) => None,
                    None => style.max_width,
                };
            }
        }
        Ok(())
    }
}

impl Shape {
    pub fn apply(&mut self, change: &ShapeChange) -> Result<(), String> {
        let before = self.content.clone();
        let mut content = before.clone();
        content.apply(change)?;
        if content != before {
            self.prepared.get_mut().clear();
            self.content = content;
        }
        self.x = change.x.unwrap_or(self.x);
        self.y = change.y.unwrap_or(self.y);
        self.z = change.z.unwrap_or(self.z);
        self.layer = change.layer.unwrap_or(self.layer);
        self.visible = change.visible.unwrap_or(self.visible);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rectangle() -> Shape {
        Shape {
            id: 1,
            generation: 0,
            target: ShapeTarget::Output(smithay::output::Output::new(
                "test".into(),
                smithay::output::PhysicalProperties {
                    size: (0, 0).into(),
                    subpixel: smithay::output::Subpixel::Unknown,
                    make: String::new(),
                    model: String::new(),
                    serial_number: String::new(),
                },
            ).downgrade()),
            x: 0,
            y: 0,
            z: 0,
            layer: Layer::Top,
            visible: true,
            content: ShapeContent::empty_rectangle(),
            prepared: RefCell::default(),
        }
    }

    #[test]
    fn a_field_left_out_keeps_its_value() {
        let mut shape = rectangle();
        shape.apply(&ShapeChange { width: Some(10), height: Some(4), x: Some(7), ..Default::default() }).unwrap();
        shape.apply(&ShapeChange { height: Some(5), ..Default::default() }).unwrap();
        let ShapeContent::Rectangle(style) = &shape.content else { unreachable!() };
        assert_eq!((style.width, style.height, shape.x), (10, 5, 7));
    }

    #[test]
    fn text_fields_on_a_rectangle_are_refused() {
        let mut shape = rectangle();
        let error = shape.apply(&ShapeChange { text: Some("hi".into()), ..Default::default() }).unwrap_err();
        assert!(error.contains("no text"), "{error}");
    }

    #[test]
    fn sizes_outside_the_limits_are_refused_and_change_nothing() {
        let mut shape = rectangle();
        assert!(shape.apply(&ShapeChange { width: Some(-1), x: Some(3), ..Default::default() }).is_err());
        assert!(shape.apply(&ShapeChange { width: Some(MAXIMUM_SIZE + 1), ..Default::default() }).is_err());
        assert_eq!(shape.x, 0);
    }

    #[test]
    fn clearing_the_fill_is_a_change() {
        let mut shape = rectangle();
        shape.apply(&ShapeChange { fill: Some(Some([1.0, 0.0, 0.0, 1.0])), ..Default::default() }).unwrap();
        shape.apply(&ShapeChange { fill: Some(None), ..Default::default() }).unwrap();
        let ShapeContent::Rectangle(style) = &shape.content else { unreachable!() };
        assert_eq!(style.fill, None);
    }

    #[test]
    fn the_font_changes_field_by_field() {
        let mut shape = rectangle();
        shape.content = ShapeContent::empty_text();
        shape.apply(&ShapeChange { font: FontChange { size: Some(20.0), ..Default::default() }, ..Default::default() }).unwrap();
        shape.apply(&ShapeChange { font: FontChange { weight: Some(700), ..Default::default() }, ..Default::default() }).unwrap();
        let ShapeContent::Text(style) = &shape.content else { unreachable!() };
        assert_eq!((style.font.size, style.font.weight), (20.0, 700));
        assert!(shape.apply(&ShapeChange { font: FontChange { size: Some(f32::NAN), ..Default::default() }, ..Default::default() }).is_err());
    }
}
