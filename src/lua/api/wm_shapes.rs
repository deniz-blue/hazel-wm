use mlua::{Lua, MultiValue, Table, UserData, Value};
use smithay::{
    desktop::Window,
    output::Output,
    utils::{Logical, Size},
};

use crate::{
    core::{
        GlobalHazel,
        drawing::shape::{ShapeContent, ShapeTarget},
        drawing::pixels::logical_length_holding,
    },
    lua::api::{
        shape_spec::{ShapeSpec, font_spec},
        utils::LuaSize,
    },
    lua_eq, lua_typedef,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WmShape(pub u64);

pub fn add_shape(target: ShapeTarget, spec: ShapeSpec) -> mlua::Result<WmShape> {
    if matches!(target, ShapeTarget::Window(_)) && spec.change.layer.is_some() {
        return Err(mlua::Error::external("a shape on a window is drawn with the window; order it with z, not layer"));
    }
    let content = spec.content()?;
    GlobalHazel::try_with(|hazel| {
        hazel.compositor.drawing.add(target, content, &spec.change).map(WmShape).map_err(mlua::Error::external)
    })
}

pub fn add_window_shape(window: &Window, spec: ShapeSpec) -> mlua::Result<WmShape> {
    add_shape(ShapeTarget::Window(window.clone()), spec)
}

pub fn add_output_shape(output: &Output, spec: ShapeSpec) -> mlua::Result<WmShape> {
    add_shape(ShapeTarget::Output(output.downgrade()), spec)
}

impl WmShape {
    fn size(&self) -> mlua::Result<Option<LuaSize<i32, Logical>>> {
        GlobalHazel::try_with(|hazel| {
            let Some(shape) = hazel.compositor.drawing.get(self.0) else {
                return Ok(None);
            };
            let (width, height) = match &shape.content {
                ShapeContent::Rectangle(style) => (style.width, style.height),
                ShapeContent::Text(style) => {
                    let max_width = style.max_width.map(|width| width as f32);
                    let metrics = hazel
                        .compositor
                        .drawing
                        .text
                        .borrow_mut()
                        .measure(&style.text, &style.font, max_width)
                        .map_err(mlua::Error::external)?;
                    (logical_length_holding(metrics.width.ceil() as u32, 1.0).0, logical_length_holding(metrics.height.ceil() as u32, 1.0).0)
                }
            };
            Ok(Some(LuaSize(Size::from((width, height)))))
        })
    }
}

impl UserData for WmShape {
    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("update", |_, this, spec: ShapeSpec| {
            if spec.kind.is_some() {
                return Err(mlua::Error::external("a shape cannot change its kind"));
            }
            GlobalHazel::try_with(|hazel| {
                hazel.compositor.drawing.update(this.0, &spec.change).map_err(mlua::Error::external)
            })?;
            Ok(())
        });
        methods.add_method("remove", |_, this, _: ()| GlobalHazel::try_with(|hazel| Ok(hazel.compositor.drawing.remove(this.0))));
        methods.add_method("alive", |_, this, _: ()| {
            GlobalHazel::try_with(|hazel| Ok(hazel.compositor.drawing.get(this.0).is_some()))
        });
        methods.add_method("kind", |_, this, _: ()| {
            GlobalHazel::try_with(|hazel| Ok(hazel.compositor.drawing.get(this.0).map(|shape| shape.content.kind())))
        });
        methods.add_method("size", |_, this, _: ()| this.size());
        lua_eq!(methods);
    }
}

lua_typedef!(Shape => WmShape {
    fn update(spec: ShapeSpec) -> nil;
    fn remove() -> boolean;
    fn alive() -> boolean;
    fn kind() -> Option<string>;
    fn size() -> Option<Size>;
});

#[derive(Default)]
pub struct WmText;

fn metrics_table(lua: &Lua, text: &str, font: Value, max_width: Option<f32>) -> mlua::Result<Table> {
    let font = font_spec(font)?;
    let metrics = GlobalHazel::try_with(|hazel| {
        hazel.compositor.drawing.text.borrow_mut().measure(text, &font, max_width).map_err(mlua::Error::external)
    })?;
    let table = lua.create_table()?;
    table.set("width", metrics.width)?;
    table.set("height", metrics.height)?;
    table.set("baseline", metrics.baseline)?;
    table.set("text", metrics.shown)?;
    Ok(table)
}

impl UserData for WmText {
    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("measure", |lua, _, arguments: MultiValue| {
            let mut arguments = arguments.into_iter();
            let text = match arguments.next() {
                Some(Value::String(text)) => text.to_string_lossy(),
                _ => return Err(mlua::Error::external("measure needs the text as its first argument")),
            };
            let font = arguments.next().unwrap_or(Value::Nil);
            let max_width = match arguments.next() {
                Some(Value::Number(width)) => Some(width as f32),
                Some(Value::Integer(width)) => Some(width as f32),
                Some(Value::Nil) | None => None,
                Some(other) => return Err(mlua::Error::external(format!("max_width must be a number, got a {}", other.type_name()))),
            };
            metrics_table(lua, &text, font, max_width)
        });
    }
}

lua_typedef!(WmText => WmText {
    fn measure(text: string, font: Option<FontSpec>, max_width: Option<number>) -> TextMetrics;
});
