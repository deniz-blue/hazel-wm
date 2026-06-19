use mlua::UserData;
use smithay::{
    desktop::{layer_map_for_output, LayerSurface as DesktopLayerSurface, WindowSurfaceType},
    utils::Logical,
    wayland::shell::wlr_layer::Layer as LayerDepth,
};

use crate::{
    impl_lua_event_handler, impl_lua_event_source,
    lua::{
        api::{
            utils::{LuaPoint, LuaSize},
            wm_outputs::WmOutputHandle,
        },
        event_handler::LuaEventHandler,
    },
    lua_typedef, with_hazel,
};

#[derive(Default)]
pub struct WmLayers {
    pub events: LuaEventHandler,
}

impl_lua_event_source!(WmLayers);

impl UserData for WmLayers {
    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        impl_lua_event_handler!(methods);

        methods.add_method("count", |_, _, _: ()| {
            with_hazel!(hazel => Ok(hazel.compositor.space.outputs().flat_map(|o| {
                let map = layer_map_for_output(o);
                map.layers().cloned().collect::<Vec<_>>()
            }).count()))
        });

        methods.add_method("get", |_, _, index: usize| {
            with_hazel!(hazel => {
                let idx = index.checked_sub(1);
                Ok(idx.and_then(|i| {
                    hazel.compositor.space.outputs().flat_map(|o| {
                        let map = layer_map_for_output(o);
                        map.layers().cloned().collect::<Vec<_>>()
                    }).nth(i).map(LayerHandle)
                }))
            })
        });

        methods.add_method("all", |_, _, _: ()| {
            with_hazel!(hazel => {
                Ok(hazel.compositor.space.outputs().flat_map(|o| {
                    let map = layer_map_for_output(o);
                    map.layers().cloned().map(LayerHandle).collect::<Vec<_>>()
                }).collect::<Vec<_>>())
            })
        });

        methods.add_method("namespace", |_, _, ns: String| {
            with_hazel!(hazel => {
                Ok(hazel.compositor.space.outputs().flat_map(|o| {
                    let map = layer_map_for_output(o);
                    map.layers().filter(|l| l.namespace() == ns).cloned().map(LayerHandle).collect::<Vec<_>>()
                }).collect::<Vec<_>>())
            })
        });

        methods.add_method("depth", |_, _, depth: String| {
            with_hazel!(hazel => {
                let target = match depth.as_str() {
                    "background" => Some(LayerDepth::Background),
                    "bottom" => Some(LayerDepth::Bottom),
                    "top" => Some(LayerDepth::Top),
                    "overlay" => Some(LayerDepth::Overlay),
                    _ => None,
                };
                Ok(hazel.compositor.space.outputs().flat_map(|o| {
                    let map = layer_map_for_output(o);
                    map.layers().filter(move |ls| Some(ls.layer()) == target).cloned().map(LayerHandle).collect::<Vec<_>>()
                }).collect::<Vec<_>>())
            })
        });

        methods.add_method("on_output", |_, _, name: String| {
            with_hazel!(hazel => {
                let out = hazel.compositor.space.outputs().find(|o| o.name() == name).cloned();
                let Some(out) = out else { return Ok(Vec::new()); };
                let map = layer_map_for_output(&out);
                Ok(map.layers().cloned().map(LayerHandle).collect::<Vec<_>>())
            })
        });

        methods.add_method("under", |_, _, point: LuaPoint<f64, _>| {
            with_hazel!(hazel => {
                for output in hazel.compositor.space.outputs() {
                    let geo = hazel.compositor.space.output_geometry(output);
                    let Some(geo) = geo else { continue; };
                    let local = point.0 - geo.loc.to_f64();
                    if local.x < 0.0 || local.y < 0.0
                        || local.x > geo.size.w as f64
                        || local.y > geo.size.h as f64
                    {
                        continue;
                    }
                    let map = layer_map_for_output(output);
                    for depth in &[LayerDepth::Overlay, LayerDepth::Top, LayerDepth::Bottom, LayerDepth::Background] {
                        if let Some(surface) = map.layer_under(*depth, local) {
                            return Ok(Some(LayerHandle(surface.clone())));
                        }
                    }
                }
                Ok(None)
            })
        });
    }
}

/// A handle to a layer surface, exposed to Lua as `Layer`.
pub struct LayerHandle(pub DesktopLayerSurface);

impl LayerHandle {
    pub fn namespace(&self) -> String {
        self.0.namespace().to_owned()
    }

    pub fn layer_name(&self) -> String {
        match self.0.layer() {
            LayerDepth::Background => "background",
            LayerDepth::Bottom => "bottom",
            LayerDepth::Top => "top",
            LayerDepth::Overlay => "overlay",
        }
        .to_owned()
    }

    pub fn position(&self) -> LuaPoint<i32, Logical> {
        with_hazel!(hazel => {
            for output in hazel.compositor.space.outputs() {
                let map = layer_map_for_output(output);
                if let Some(geo) = map.layer_geometry(&self.0) {
                    return LuaPoint(geo.loc);
                }
            }
            LuaPoint(Default::default())
        })
    }

    pub fn size(&self) -> LuaSize<i32, Logical> {
        with_hazel!(hazel => {
            for output in hazel.compositor.space.outputs() {
                let map = layer_map_for_output(output);
                if let Some(geo) = map.layer_geometry(&self.0) {
                    return LuaSize(geo.size);
                }
            }
            LuaSize(Default::default())
        })
    }

    pub fn output(&self) -> Option<WmOutputHandle> {
        let outputs = crate::core::GlobalHazel::with(|hazel| {
            hazel.compositor.space.outputs().cloned().collect::<Vec<_>>()
        });
        for output in &outputs {
            let map = layer_map_for_output(output);
            if map.layer_for_surface(self.0.wl_surface(), WindowSurfaceType::TOPLEVEL).is_some() {
                return Some(WmOutputHandle::new(output));
            }
        }
        None
    }

    pub fn close(&self) {
        self.0.layer_surface().send_close();
    }
}

impl UserData for LayerHandle {
    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("namespace", |_, this, _: ()| Ok(this.namespace()));
        methods.add_method("layer", |_, this, _: ()| Ok(this.layer_name()));
        methods.add_method("position", |_, this, _: ()| Ok(this.position()));
        methods.add_method("size", |_, this, _: ()| Ok(this.size()));
        methods.add_method("output", |_, this, _: ()| Ok(this.output()));
        methods.add_method("close", |_, this, _: ()| {
            this.close();
            Ok(())
        });
    }
}

lua_typedef!(WmLayers => WmLayers {
    fn count() -> number;
    fn get(index: number) -> Layer;
    fn all() -> Array<Layer>;
    fn namespace(name: string) -> Array<Layer>;
    fn depth(depth: string) -> Array<Layer>;
    fn on_output(name: string) -> Array<Layer>;
    fn under(point: Point) -> Layer;
});

lua_typedef!(Layer => LayerHandle {
    fn namespace() -> string;
    fn layer() -> string;
    fn position() -> Point;
    fn size() -> Size;
    fn output() -> WmOutput;
    fn close() -> nil;
});
