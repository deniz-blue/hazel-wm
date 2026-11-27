use std::cell::RefCell;

use smithay::{desktop::Window, output::Output, wayland::shell::wlr_layer::Layer};

use super::{
    shape::{Shape, ShapeChange, ShapeContent, ShapeTarget},
    text::TextEngine,
};

#[derive(Default)]
pub struct Drawing {
    shapes: Vec<Shape>,
    next_id: u64,
    generation: u64,
    pub text: RefCell<TextEngine>,
}

impl Drawing {
    pub fn add(&mut self, target: ShapeTarget, content: ShapeContent, change: &ShapeChange) -> Result<u64, String> {
        self.next_id += 1;
        let mut shape = Shape {
            id: self.next_id,
            generation: self.generation,
            target,
            x: 0,
            y: 0,
            z: 0,
            layer: Layer::Top,
            visible: true,
            content,
            prepared: RefCell::default(),
        };
        shape.apply(change)?;
        self.shapes.push(shape);
        Ok(self.next_id)
    }

    pub fn start_generation(&mut self) -> u64 {
        self.generation += 1;
        self.generation - 1
    }

    pub fn keep_only_current_generation(&mut self) {
        let current = self.generation;
        self.shapes.retain(|shape| shape.generation == current);
    }

    pub fn abandon_generation(&mut self, previous: u64) {
        let abandoned = self.generation;
        self.shapes.retain(|shape| shape.generation != abandoned);
        self.generation = previous;
    }

    pub fn get(&self, id: u64) -> Option<&Shape> {
        self.shapes.iter().find(|shape| shape.id == id)
    }

    pub fn update(&mut self, id: u64, change: &ShapeChange) -> Result<bool, String> {
        match self.shapes.iter_mut().find(|shape| shape.id == id) {
            Some(shape) => shape.apply(change).map(|()| true),
            None => Ok(false),
        }
    }

    pub fn remove(&mut self, id: u64) -> bool {
        let before = self.shapes.len();
        self.shapes.retain(|shape| shape.id != id);
        self.shapes.len() != before
    }

    pub fn remove_on_window(&mut self, window: &Window) {
        self.shapes.retain(|shape| !matches!(&shape.target, ShapeTarget::Window(other) if other == window));
    }

    pub fn remove_on_output(&mut self, output: &Output) {
        self.shapes.retain(|shape| !matches!(&shape.target, ShapeTarget::Output(other) if other == &output.downgrade()));
    }

    fn visible_front_first<'a>(&'a self, belongs: impl Fn(&Shape) -> bool + 'a) -> Vec<&'a Shape> {
        let mut found: Vec<&Shape> = self.shapes.iter().filter(|shape| shape.visible && belongs(shape)).collect();
        found.sort_by_key(|shape| std::cmp::Reverse((shape.z, shape.id)));
        found
    }

    pub fn on_output(&self, output: &Output, layer: Layer) -> Vec<&Shape> {
        let output = output.downgrade();
        self.visible_front_first(move |shape| {
            shape.layer == layer && matches!(&shape.target, ShapeTarget::Output(other) if *other == output)
        })
    }

    pub fn on_window(&self, window: &Window) -> Vec<&Shape> {
        let window = window.clone();
        self.visible_front_first(move |shape| matches!(&shape.target, ShapeTarget::Window(other) if *other == window))
    }
}

#[cfg(test)]
mod tests {
    use smithay::output::{PhysicalProperties, Subpixel};

    use super::*;
    use crate::core::drawing::shape::ShapeChange;

    fn output(name: &str) -> Output {
        Output::new(
            name.into(),
            PhysicalProperties { size: (0, 0).into(), subpixel: Subpixel::Unknown, make: String::new(), model: String::new(), serial_number: String::new() },
        )
    }

    fn add_on(drawing: &mut Drawing, output: &Output, z: i32, layer: Layer) -> u64 {
        let change = ShapeChange { z: Some(z), layer: Some(layer), ..Default::default() };
        drawing.add(ShapeTarget::Output(output.downgrade()), ShapeContent::empty_rectangle(), &change).unwrap()
    }

    #[test]
    fn shapes_come_back_front_first_with_newer_on_top_of_equal_z() {
        let (mut drawing, output) = (Drawing::default(), output("a"));
        let low = add_on(&mut drawing, &output, 0, Layer::Top);
        let high = add_on(&mut drawing, &output, 5, Layer::Top);
        let newer = add_on(&mut drawing, &output, 0, Layer::Top);
        let ids: Vec<u64> = drawing.on_output(&output, Layer::Top).iter().map(|shape| shape.id).collect();
        assert_eq!(ids, [high, newer, low]);
    }

    #[test]
    fn a_layer_only_returns_its_own_shapes() {
        let (mut drawing, output) = (Drawing::default(), output("a"));
        add_on(&mut drawing, &output, 0, Layer::Bottom);
        assert!(drawing.on_output(&output, Layer::Top).is_empty());
        assert_eq!(drawing.on_output(&output, Layer::Bottom).len(), 1);
    }

    #[test]
    fn shapes_belong_to_their_own_output() {
        let (mut drawing, first, second) = (Drawing::default(), output("a"), output("b"));
        add_on(&mut drawing, &first, 0, Layer::Top);
        assert!(drawing.on_output(&second, Layer::Top).is_empty());
    }

    #[test]
    fn removing_an_output_removes_its_shapes_only() {
        let (mut drawing, first, second) = (Drawing::default(), output("a"), output("b"));
        add_on(&mut drawing, &first, 0, Layer::Top);
        let kept = add_on(&mut drawing, &second, 0, Layer::Top);
        drawing.remove_on_output(&first);
        assert!(drawing.get(kept).is_some());
        assert_eq!(drawing.shapes.len(), 1);
    }

    #[test]
    fn a_new_generation_replaces_the_old_shapes_once_kept() {
        let (mut drawing, output) = (Drawing::default(), output("a"));
        let old = add_on(&mut drawing, &output, 0, Layer::Top);
        drawing.start_generation();
        let new = add_on(&mut drawing, &output, 0, Layer::Top);
        assert!(drawing.get(old).is_some());
        drawing.keep_only_current_generation();
        assert!(drawing.get(old).is_none() && drawing.get(new).is_some());
    }

    #[test]
    fn an_abandoned_generation_leaves_the_old_shapes_alone() {
        let (mut drawing, output) = (Drawing::default(), output("a"));
        let old = add_on(&mut drawing, &output, 0, Layer::Top);
        let previous = drawing.start_generation();
        let new = add_on(&mut drawing, &output, 0, Layer::Top);
        drawing.abandon_generation(previous);
        assert!(drawing.get(old).is_some() && drawing.get(new).is_none());
        let later = add_on(&mut drawing, &output, 0, Layer::Top);
        drawing.start_generation();
        drawing.keep_only_current_generation();
        assert!(drawing.get(later).is_none());
    }

    #[test]
    fn hidden_shapes_are_not_returned_and_removed_ones_are_gone() {
        let (mut drawing, output) = (Drawing::default(), output("a"));
        let id = add_on(&mut drawing, &output, 0, Layer::Top);
        drawing.update(id, &ShapeChange { visible: Some(false), ..Default::default() }).unwrap();
        assert!(drawing.on_output(&output, Layer::Top).is_empty());
        assert!(drawing.remove(id));
        assert!(!drawing.remove(id));
        assert_eq!(drawing.update(id, &ShapeChange::default()), Ok(false));
    }
}
