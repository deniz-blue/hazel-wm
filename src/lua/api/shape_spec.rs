use std::path::PathBuf;

use mlua::{FromLua, Lua, Table, Value};
use smithay::wayland::shell::wlr_layer::Layer;

use crate::core::{
    drawing::{
        shape::{FontChange, ShapeChange, ShapeContent},
        text::{FontSource, FontSpec},
    },
    window_borders::parse_hex_color,
};

const SPEC_KEYS: [&str; 16] = [
    "kind", "x", "y", "z", "layer", "visible", "width", "height", "fill", "radius", "border_width", "border_color", "text", "font", "color",
    "max_width",
];
const FONT_KEYS: [&str; 4] = ["family", "file", "size", "weight"];

pub struct ShapeSpec {
    pub kind: Option<String>,
    pub change: ShapeChange,
}

impl ShapeSpec {
    pub fn content(&self) -> mlua::Result<ShapeContent> {
        match self.kind.as_deref() {
            Some("rect") => Ok(ShapeContent::empty_rectangle()),
            Some("text") => Ok(ShapeContent::empty_text()),
            Some(other) => Err(mlua::Error::external(format!("kind must be \"rect\" or \"text\", got {other:?}"))),
            None => Err(mlua::Error::external("a shape needs a kind, \"rect\" or \"text\"")),
        }
    }
}

fn only_known_keys(table: &Table, known: &[&str], what: &str) -> mlua::Result<()> {
    for pair in table.pairs::<Value, Value>() {
        let (key, _) = pair?;
        match &key {
            Value::String(name) if known.contains(&&*name.to_str()?) => {}
            Value::String(name) => {
                return Err(mlua::Error::external(format!("{what} has no field {:?}, it has {}", name.to_str()?, known.join(", "))));
            }
            other => return Err(mlua::Error::external(format!("{what} fields are named by strings, got a {}", other.type_name()))),
        }
    }
    Ok(())
}

fn integer(table: &Table, name: &str) -> mlua::Result<Option<i32>> {
    let Some(number) = table.get::<Option<f64>>(name)? else {
        return Ok(None);
    };
    if !number.is_finite() || number.abs() > f64::from(i32::MAX) {
        return Err(mlua::Error::external(format!("{name} must be a number within the 32-bit range, got {number}")));
    }
    Ok(Some(number.round() as i32))
}

fn colour(table: &Table, name: &str) -> mlua::Result<Option<[f32; 4]>> {
    table
        .get::<Option<String>>(name)?
        .map(|text| {
            parse_hex_color(&text)
                .ok_or_else(|| mlua::Error::external(format!("{name} must look like \"#rrggbb\" or \"#rrggbbaa\", got {text:?}")))
        })
        .transpose()
}

fn layer(table: &Table) -> mlua::Result<Option<Layer>> {
    table
        .get::<Option<String>>("layer")?
        .map(|name| match name.as_str() {
            "background" => Ok(Layer::Background),
            "bottom" => Ok(Layer::Bottom),
            "top" => Ok(Layer::Top),
            "overlay" => Ok(Layer::Overlay),
            other => Err(mlua::Error::external(format!("layer must be \"background\", \"bottom\", \"top\" or \"overlay\", got {other:?}"))),
        })
        .transpose()
}

fn fill(table: &Table) -> mlua::Result<Option<Option<[f32; 4]>>> {
    match table.get::<Value>("fill")? {
        Value::Nil => Ok(None),
        Value::Boolean(false) => Ok(Some(None)),
        _ => Ok(colour(table, "fill")?.map(Some)),
    }
}

fn max_width(table: &Table) -> mlua::Result<Option<Option<i32>>> {
    match table.get::<Value>("max_width")? {
        Value::Nil => Ok(None),
        Value::Boolean(false) => Ok(Some(None)),
        _ => Ok(integer(table, "max_width")?.map(Some)),
    }
}

fn font_change(table: &Table) -> mlua::Result<FontChange> {
    only_known_keys(table, &FONT_KEYS, "a font")?;
    let family = table.get::<Option<String>>("family")?;
    let file = table.get::<Option<String>>("file")?;
    let source = match (family, file) {
        (Some(_), Some(_)) => return Err(mlua::Error::external("a font has a family or a file, not both")),
        (Some(family), None) => Some(FontSource::Family(family)),
        (None, Some(file)) => Some(FontSource::File(PathBuf::from(file))),
        (None, None) => None,
    };
    let weight = integer(table, "weight")?
        .map(|weight| u16::try_from(weight).ok().filter(|weight| (1..=1000).contains(weight)))
        .map(|weight| weight.ok_or_else(|| mlua::Error::external("font weight must be between 1 and 1000")))
        .transpose()?;
    Ok(FontChange { source, size: table.get::<Option<f32>>("size")?, weight })
}

pub fn font_spec(value: Value) -> mlua::Result<FontSpec> {
    let mut font = FontSpec::default();
    let table = match value {
        Value::Nil => return Ok(font),
        Value::Table(table) => table,
        other => return Err(mlua::Error::external(format!("a font is described by a table, got a {}", other.type_name()))),
    };
    let change = font_change(&table)?;
    font.source = change.source.unwrap_or(font.source);
    font.size = change.size.unwrap_or(font.size);
    font.weight = change.weight.unwrap_or(font.weight);
    if !(font.size > 0.0 && font.size <= crate::core::drawing::shape::MAXIMUM_FONT_SIZE) {
        return Err(mlua::Error::external(format!("font size must be above 0 and at most 1024, got {}", font.size)));
    }
    Ok(font)
}

impl FromLua for ShapeSpec {
    fn from_lua(value: Value, _: &Lua) -> mlua::Result<Self> {
        let Value::Table(table) = value else {
            return Err(mlua::Error::external(format!("a shape is described by a table, got a {}", value.type_name())));
        };
        only_known_keys(&table, &SPEC_KEYS, "a shape")?;
        let font = match table.get::<Value>("font")? {
            Value::Nil => FontChange::default(),
            Value::Table(font) => font_change(&font)?,
            other => return Err(mlua::Error::external(format!("font must be a table, got a {}", other.type_name()))),
        };
        Ok(Self {
            kind: table.get("kind")?,
            change: ShapeChange {
                x: integer(&table, "x")?,
                y: integer(&table, "y")?,
                z: integer(&table, "z")?,
                layer: layer(&table)?,
                visible: table.get("visible")?,
                width: integer(&table, "width")?,
                height: integer(&table, "height")?,
                fill: fill(&table)?,
                radius: integer(&table, "radius")?,
                border_width: integer(&table, "border_width")?,
                border_color: colour(&table, "border_color")?,
                text: table.get("text")?,
                font,
                color: colour(&table, "color")?,
                max_width: max_width(&table)?,
            },
        })
    }
}
