use smithay::wayland::selection::SelectionHandler;

use crate::core::Hazel;

impl SelectionHandler for Hazel {
    type SelectionUserData = ();
}
