use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
};

use smithay::{
    backend::renderer::element::{
        Kind,
        solid::{SolidColorBuffer, SolidColorRenderElement},
    },
    output::Output,
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    utils::{Logical, Point, Rectangle, Scale},
    wayland::session_lock::{LockSurface, SessionLocker},
};

#[derive(Default)]
enum LockPhase {
    #[default]
    Unlocked,
    Locking(SessionLocker),
    Locked,
}

pub struct SessionLock {
    phase: LockPhase,
    surfaces: Vec<(LockSurface, Output)>,
    backdrops: RefCell<HashMap<String, SolidColorBuffer>>,
    backdrop_color: Cell<[f32; 4]>,
}

impl Default for SessionLock {
    fn default() -> Self {
        Self {
            phase: LockPhase::default(),
            surfaces: Vec::new(),
            backdrops: RefCell::default(),
            backdrop_color: Cell::new([0.0, 0.0, 0.0, 1.0]),
        }
    }
}

impl SessionLock {
    pub fn phase_name(&self) -> &'static str {
        match self.phase {
            LockPhase::Unlocked => "unlocked",
            LockPhase::Locking(_) => "locking",
            LockPhase::Locked => "locked",
        }
    }

    pub fn live_surfaces(&self) -> impl Iterator<Item = &(LockSurface, Output)> {
        self.surfaces.iter().filter(|(surface, _)| surface.alive())
    }

    pub fn set_backdrop_color(&self, color: [f32; 4]) {
        self.backdrop_color.set(color);
    }

    pub fn is_active(&self) -> bool {
        !matches!(self.phase, LockPhase::Unlocked)
    }

    pub fn is_locked(&self) -> bool {
        matches!(self.phase, LockPhase::Locked)
    }

    pub fn is_orphaned(&self) -> bool {
        self.is_locked() && !self.surfaces.iter().any(|(surface, _)| surface.alive())
    }

    pub fn begin(&mut self, locker: SessionLocker) {
        self.surfaces.clear();
        self.phase = LockPhase::Locking(locker);
    }

    pub fn add_surface(&mut self, surface: LockSurface, output: Output) {
        self.surfaces.push((surface, output));
    }

    pub fn confirm(&mut self) -> bool {
        match std::mem::take(&mut self.phase) {
            LockPhase::Locking(locker) => {
                locker.lock();
                self.phase = LockPhase::Locked;
                true
            }
            other => {
                self.phase = other;
                false
            }
        }
    }

    pub fn release(&mut self) {
        self.phase = LockPhase::Unlocked;
        self.surfaces.clear();
        self.backdrops.borrow_mut().clear();
    }

    pub fn is_waiting_for_surfaces(&self) -> bool {
        matches!(self.phase, LockPhase::Locking(_))
    }

    pub fn surface_on(&self, output: &Output) -> Option<&LockSurface> {
        self.surfaces
            .iter()
            .find(|(surface, on)| on == output && surface.alive())
            .map(|(surface, _)| surface)
    }

    pub fn owns_surface(&self, surface: &WlSurface) -> bool {
        self.surfaces.iter().any(|(lock_surface, _)| lock_surface.wl_surface() == surface)
    }

    pub fn focus_surface(&self) -> Option<WlSurface> {
        self.surfaces
            .iter()
            .find(|(surface, _)| surface.alive())
            .map(|(surface, _)| surface.wl_surface().clone())
    }

    pub fn surface_at(
        &self,
        point: Point<f64, Logical>,
        geometry_of: impl Fn(&Output) -> Option<Rectangle<i32, Logical>>,
    ) -> Option<(WlSurface, Point<f64, Logical>)> {
        self.surfaces.iter().find_map(|(surface, output)| {
            let geometry = geometry_of(output)?;
            (surface.alive() && geometry.to_f64().contains(point))
                .then(|| (surface.wl_surface().clone(), geometry.loc.to_f64()))
        })
    }

    pub fn backdrop(
        &self,
        output: &Output,
        geometry: Rectangle<i32, Logical>,
        scale: Scale<f64>,
    ) -> SolidColorRenderElement {
        let mut backdrops = self.backdrops.borrow_mut();
        let buffer = backdrops.entry(output.name()).or_default();
        buffer.update(geometry.size, self.backdrop_color.get());
        SolidColorRenderElement::from_buffer(buffer, (0, 0), scale, 1.0, Kind::Unspecified)
    }
}
