use smithay::{
    desktop::LayerSurface,
    wayland::shell::wlr_layer::{Anchor, KeyboardInteractivity, Layer},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LayerProperties {
    pub anchor: Anchor,
    pub exclusive_zone: i32,
    pub exclusive_edge: Option<Anchor>,
    pub margin_top: i32,
    pub margin_right: i32,
    pub margin_bottom: i32,
    pub margin_left: i32,
    pub requested_width: i32,
    pub requested_height: i32,
    pub keyboard_interactivity: KeyboardInteractivity,
    pub depth: Layer,
}

impl LayerProperties {
    pub fn of(layer: &LayerSurface) -> Self {
        let state = layer.cached_state();
        Self {
            anchor: state.anchor,
            exclusive_zone: state.exclusive_zone.into(),
            exclusive_edge: state.exclusive_edge,
            margin_top: state.margin.top,
            margin_right: state.margin.right,
            margin_bottom: state.margin.bottom,
            margin_left: state.margin.left,
            requested_width: state.size.w,
            requested_height: state.size.h,
            keyboard_interactivity: state.keyboard_interactivity,
            depth: state.layer,
        }
    }
}

pub fn edge_name(edge: Anchor) -> Option<&'static str> {
    match edge {
        Anchor::TOP => Some("top"),
        Anchor::BOTTOM => Some("bottom"),
        Anchor::LEFT => Some("left"),
        Anchor::RIGHT => Some("right"),
        _ => None,
    }
}
