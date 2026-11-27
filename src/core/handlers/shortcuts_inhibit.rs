use smithay::{
    wayland::keyboard_shortcuts_inhibit::{
        KeyboardShortcutsInhibitHandler, KeyboardShortcutsInhibitState, KeyboardShortcutsInhibitor,
    },
};

use crate::{core::Hazel, lua::api::wm_input_shortcuts::WmShortcutsInhibitor};

impl KeyboardShortcutsInhibitHandler for Hazel {
    fn keyboard_shortcuts_inhibit_state(&mut self) -> &mut KeyboardShortcutsInhibitState {
        &mut self.compositor.smithay.keyboard_shortcuts_inhibit_state
    }

    fn new_inhibitor(&mut self, inhibitor: KeyboardShortcutsInhibitor) {
        let _ = self
            .wm()
            .input
            .events
            .emit(WmShortcutsInhibitor::name(), WmShortcutsInhibitor(inhibitor));
    }
}

