use smithay::wayland::selection::data_device::{ClientDndGrabHandler, ServerDndGrabHandler};

use crate::core::Hazel;

impl ServerDndGrabHandler for Hazel {}

impl ClientDndGrabHandler for Hazel {}
