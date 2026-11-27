use std::cell::Cell;

use smithay::desktop::Window;

use crate::core::compositor::HazelCompositor;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum WindowLayer {
    Below,
    #[default]
    Normal,
    Above,
}

impl WindowLayer {
    pub fn name(self) -> &'static str {
        match self {
            Self::Below => "below",
            Self::Normal => "normal",
            Self::Above => "above",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "below" => Some(Self::Below),
            "normal" => Some(Self::Normal),
            "above" => Some(Self::Above),
            _ => None,
        }
    }
}

pub fn window_layer(window: &Window) -> WindowLayer {
    window.user_data().get_or_insert(Cell::<WindowLayer>::default).get()
}

pub fn set_window_layer(window: &Window, layer: WindowLayer) {
    window.user_data().get_or_insert(Cell::<WindowLayer>::default).set(layer);
}

impl HazelCompositor {
    pub fn restack(&mut self) {
        let mut order: Vec<Window> = self.space.elements().cloned().collect();
        if order.windows(2).all(|pair| window_layer(&pair[0]) <= window_layer(&pair[1])) {
            return;
        }
        order.sort_by_key(window_layer);
        for window in &order {
            self.space.raise_element(window, false);
        }
    }
}
