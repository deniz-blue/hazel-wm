use smithay::{
    backend::input::{AxisRelativeDirection, AxisSource},
    input::pointer::AxisFrame,
};

const PIXELS_PER_WHEEL_NOTCH: f64 = 15.0;
const V120_PER_WHEEL_NOTCH: f64 = 120.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scroll {
    pub source: AxisSource,
    pub horizontal: Option<f64>,
    pub vertical: Option<f64>,
    pub horizontal_v120: Option<f64>,
    pub vertical_v120: Option<f64>,
}

impl Scroll {
    pub fn wheel_notches(horizontal: f64, vertical: f64) -> Self {
        Self {
            source: AxisSource::Wheel,
            horizontal: Some(horizontal * PIXELS_PER_WHEEL_NOTCH),
            vertical: Some(vertical * PIXELS_PER_WHEEL_NOTCH),
            horizontal_v120: Some(horizontal * V120_PER_WHEEL_NOTCH),
            vertical_v120: Some(vertical * V120_PER_WHEEL_NOTCH),
        }
    }

    pub fn pixels(source: AxisSource, horizontal: f64, vertical: f64) -> Self {
        Self {
            source,
            horizontal: Some(horizontal),
            vertical: Some(vertical),
            horizontal_v120: None,
            vertical_v120: None,
        }
    }

    fn amount(pixels: Option<f64>, v120: Option<f64>) -> f64 {
        pixels
            .or_else(|| v120.map(|v| v * PIXELS_PER_WHEEL_NOTCH / V120_PER_WHEEL_NOTCH))
            .unwrap_or_default()
    }

    pub fn axis_frame(&self, time: u32) -> AxisFrame {
        let horizontal = Self::amount(self.horizontal, self.horizontal_v120);
        let vertical = Self::amount(self.vertical, self.vertical_v120);
        let is_wheel = self.source == AxisSource::Wheel;
        let has_v120 = self.horizontal_v120.is_some() || self.vertical_v120.is_some();
        let v120 = (is_wheel && has_v120).then(|| {
            (
                self.horizontal_v120.unwrap_or_default().round() as i32,
                self.vertical_v120.unwrap_or_default().round() as i32,
            )
        });
        let finger = self.source == AxisSource::Finger;
        AxisFrame {
            axis: (horizontal, vertical),
            source: Some(self.source),
            time,
            stop: (finger && horizontal == 0.0, finger && vertical == 0.0),
            relative_direction: (
                AxisRelativeDirection::Identical,
                AxisRelativeDirection::Identical,
            ),
            v120,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wheel_notches_carry_pixels_and_v120() {
        let frame = Scroll::wheel_notches(0.0, 2.0).axis_frame(5);
        assert_eq!(frame.axis, (0.0, 30.0));
        assert_eq!(frame.v120, Some((0, 240)));
        assert_eq!(frame.stop, (false, false));
    }

    #[test]
    fn missing_pixels_are_derived_from_v120() {
        let scroll = Scroll {
            source: AxisSource::Wheel,
            horizontal: None,
            vertical: None,
            horizontal_v120: Some(60.0),
            vertical_v120: None,
        };
        let frame = scroll.axis_frame(0);
        assert_eq!(frame.axis, (7.5, 0.0));
        assert_eq!(frame.v120, Some((60, 0)));
    }

    #[test]
    fn finger_scroll_has_no_v120_and_stops_at_zero() {
        let frame = Scroll::pixels(AxisSource::Finger, 0.0, 12.0).axis_frame(0);
        assert_eq!(frame.v120, None);
        assert_eq!(frame.stop, (true, false));
    }
}
