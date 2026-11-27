use smithay::{
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    wayland::xdg_activation::{
        XdgActivationHandler, XdgActivationState, XdgActivationToken, XdgActivationTokenData,
    },
};

use crate::{core::Hazel, lua::api::wm_windows::WmWindow};

impl XdgActivationHandler for Hazel {
    fn activation_state(&mut self) -> &mut XdgActivationState {
        &mut self.compositor.smithay.xdg_activation_state
    }

    fn request_activation(
        &mut self,
        token: XdgActivationToken,
        token_data: XdgActivationTokenData,
        surface: WlSurface,
    ) {
        if let Some(window) = self.compositor.window_of_surface(&surface) {
            let token_age_ms = token_data.timestamp.elapsed().as_millis() as u64;
            let _ = self
                .wm()
                .windows
                .events
                .emit("activation_request".to_owned(), (WmWindow(window), token_age_ms));
        }
        self.compositor
            .smithay
            .xdg_activation_state
            .remove_token(&token);
    }
}

