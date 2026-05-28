use std::{cell::RefCell, rc::Rc};

use crate::core::Hazel;

pub mod event_handler;
pub mod runtime;
pub mod api;

pub type HazelHandle = Rc<RefCell<Hazel>>;
