use smithay::{
    backend::renderer::{ImportAll, ImportMem, RendererSuper},
    desktop::Window,
    output::Output,
    utils::{Logical, Point, Scale},
    wayland::shell::wlr_layer::Layer,
};

use super::shape::Shape;
use crate::core::{compositor::HazelCompositor, render::HazelElement};

pub struct WindowShapeElements<R: ImportAll + ImportMem> {
    pub in_front: Vec<HazelElement<R>>,
    pub behind: Vec<HazelElement<R>>,
}

impl HazelCompositor {
    fn elements_of_shapes<R>(
        &self,
        renderer: &mut R,
        shapes: &[&Shape],
        origin: Point<i32, Logical>,
        scale: Scale<f64>,
    ) -> Vec<HazelElement<R>>
    where
        R: ImportAll + ImportMem,
        <R as RendererSuper>::TextureId: Send + Clone + 'static,
    {
        let mut text = self.drawing.text.borrow_mut();
        shapes
            .iter()
            .filter_map(|shape| {
                let location = (origin + Point::from((shape.x, shape.y))).to_f64().to_physical(scale).to_i32_round();
                let mut prepared = shape.prepared.borrow_mut();
                prepared
                    .at_scale(scale.x, &shape.content, &mut text)
                    .element(renderer, location, scale)
            })
            .collect()
    }

    pub fn output_shape_elements<R>(&self, renderer: &mut R, output: &Output, layer: Layer) -> Vec<HazelElement<R>>
    where
        R: ImportAll + ImportMem,
        <R as RendererSuper>::TextureId: Send + Clone + 'static,
    {
        let scale = Scale::from(output.current_scale().fractional_scale());
        let shapes = self.drawing.on_output(output, layer);
        self.elements_of_shapes(renderer, &shapes, Point::default(), scale)
    }

    pub fn window_shape_elements<R>(
        &self,
        renderer: &mut R,
        window: &Window,
        origin: Point<i32, Logical>,
        scale: Scale<f64>,
    ) -> WindowShapeElements<R>
    where
        R: ImportAll + ImportMem,
        <R as RendererSuper>::TextureId: Send + Clone + 'static,
    {
        let (in_front, behind): (Vec<&Shape>, Vec<&Shape>) = self.drawing.on_window(window).into_iter().partition(|shape| shape.z >= 0);
        WindowShapeElements {
            in_front: self.elements_of_shapes(renderer, &in_front, origin, scale),
            behind: self.elements_of_shapes(renderer, &behind, origin, scale),
        }
    }
}
