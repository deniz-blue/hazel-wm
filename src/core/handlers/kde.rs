use smithay::{
    delegate_kde_decoration,
    wayland::shell::kde::decoration::{KdeDecorationHandler, KdeDecorationState},
};

use crate::core::Hazel;

impl KdeDecorationHandler for Hazel {
    fn kde_decoration_state(&self) -> &KdeDecorationState {
        &self.compositor.smithay.kde_decoration_state
    }
}

delegate_kde_decoration!(Hazel);
