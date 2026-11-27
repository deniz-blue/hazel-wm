use smithay::{
    input::pointer::PointerHandle,
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    utils::{Logical, Point, Rectangle},
    wayland::{
        compositor::{RectangleKind, RegionAttributes},
        pointer_constraints::{PointerConstraint, with_pointer_constraint},
    },
    backend::renderer::utils::with_renderer_surface_state,
};

use crate::core::Hazel;

#[derive(Debug, PartialEq)]
pub enum ConstraintEffect {
    Free,
    Locked,
    ConfinedTo(Rectangle<f64, Logical>),
}

fn region_bounds(region: &RegionAttributes) -> Option<Rectangle<i32, Logical>> {
    region
        .rects
        .iter()
        .filter(|(kind, _)| matches!(kind, RectangleKind::Add))
        .map(|(_, rect)| *rect)
        .reduce(|a, b| a.merge(b))
}

fn surface_bounds(surface: &WlSurface) -> Option<Rectangle<i32, Logical>> {
    with_renderer_surface_state(surface, |state| state.surface_size())
        .flatten()
        .map(|size| Rectangle::from_size(size))
}

const SMALLEST_STEP: f64 = 1.0 / 256.0;

// The far edge of a surface is outside it, so the last point inside is one wl_fixed step short.
pub fn clamp_to(rect: Rectangle<f64, Logical>, point: Point<f64, Logical>) -> Point<f64, Logical> {
    let max = rect.loc + Point::from((rect.size.w - SMALLEST_STEP, rect.size.h - SMALLEST_STEP));
    (point.x.clamp(rect.loc.x, max.x), point.y.clamp(rect.loc.y, max.y)).into()
}

enum ActiveConstraint {
    Locked,
    ConfinedWithin(Option<Rectangle<i32, Logical>>),
}

fn active_constraint(surface: &WlSurface, pointer: &PointerHandle<Hazel>) -> Option<ActiveConstraint> {
    with_pointer_constraint(surface, pointer, |constraint| {
        let constraint = constraint.filter(|constraint| constraint.is_active())?;
        Some(match &*constraint {
            PointerConstraint::Locked(_) => ActiveConstraint::Locked,
            PointerConstraint::Confined(_) => {
                ActiveConstraint::ConfinedWithin(constraint.region().and_then(region_bounds))
            }
        })
    })
}

impl Hazel {
    pub fn constraint_effect(
        &self,
        pointer: &PointerHandle<Hazel>,
        held: Option<&(WlSurface, Point<f64, Logical>)>,
    ) -> ConstraintEffect {
        let Some((surface, origin)) = held else {
            return ConstraintEffect::Free;
        };
        match active_constraint(surface, pointer) {
            None => ConstraintEffect::Free,
            Some(ActiveConstraint::Locked) => ConstraintEffect::Locked,
            Some(ActiveConstraint::ConfinedWithin(region)) => match region.or_else(|| surface_bounds(surface)) {
                Some(bounds) => ConstraintEffect::ConfinedTo(Rectangle::new(
                    bounds.loc.to_f64() + *origin,
                    bounds.size.to_f64(),
                )),
                None => ConstraintEffect::Free,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_keeps_points_inside() {
        let rect = Rectangle::<f64, Logical>::new((10.0, 20.0).into(), (100.0, 50.0).into());
        assert_eq!(clamp_to(rect, (50.0, 30.0).into()), (50.0, 30.0).into());
        assert_eq!(clamp_to(rect, (0.0, 90.0).into()), (10.0, 70.0 - SMALLEST_STEP).into());
        assert_eq!(clamp_to(rect, (500.0, -5.0).into()), (110.0 - SMALLEST_STEP, 20.0).into());
    }

    #[test]
    fn region_bounds_ignore_subtracted_rectangles() {
        let rect = |x, y, w, h| Rectangle::<i32, Logical>::new((x, y).into(), (w, h).into());
        let region = RegionAttributes {
            rects: vec![
                (RectangleKind::Add, rect(0, 0, 10, 10)),
                (RectangleKind::Add, rect(20, 0, 10, 10)),
                (RectangleKind::Subtract, rect(0, 0, 100, 100)),
            ],
        };
        assert_eq!(region_bounds(&region), Some(rect(0, 0, 30, 10)));
        assert_eq!(region_bounds(&RegionAttributes::default()), None);
    }
}
