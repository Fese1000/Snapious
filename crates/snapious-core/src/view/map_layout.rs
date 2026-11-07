//! Geometry for the plasmid map (circular) and linear map. Pure number crunching, no drawing.

use std::f64::consts::PI;

use crate::error::{Error, Result};

/// Circular map: position 0 is at the top (12 o'clock), positions run clockwise.
/// Angles are radians in `[0, 2π)`. Points have the center `(0, 0)`, `x` points right,
/// `y` down (as on screen).
#[derive(Clone, Debug, PartialEq)]
pub struct CircularMapLayout {
    length: usize,
}

impl CircularMapLayout {
    /// Error `Error::InvalidInput` if `length == 0`.
    pub fn new(length: usize) -> Result<Self> {
        if length == 0 {
            return Err(Error::InvalidInput("length must be > 0".into()));
        }
        Ok(CircularMapLayout { length })
    }

    /// Length of the sequence in bases.
    pub fn length(&self) -> usize {
        self.length
    }

    /// Angle of a position: `2π · position / length`. Positions from `length` on keep wrapping around the circle
    /// (the result is normalized to `[0, 2π)`).
    pub fn angle_of(&self, position: f64) -> f64 {
        (position / self.length as f64 * 2.0 * PI) % (2.0 * PI)
    }

    /// Point on the circle with `radius`: `x = radius · sin(angle)`, `y = -radius · cos(angle)`.
    pub fn point_at(&self, position: f64, radius: f64) -> (f64, f64) {
        let angle = self.angle_of(position);
        (radius * angle.sin(), -radius * angle.cos())
    }

    /// Swept angle from `start` to `end` (base positions, `end >= start`): `2π · (end − start) / length`.
    /// If `end < start` (feature across the origin), `end + length` is used.
    pub fn sweep_of(&self, start: usize, end: usize) -> f64 {
        let effective_end = if end < start { end + self.length } else { end };
        2.0 * PI * (effective_end - start) as f64 / self.length as f64
    }

    /// Inverse of `angle_of`: bring the angle (also negative or above 2π) into `[0, 2π)`, then
    /// `round(angle / 2π · length)`; a result of `length` becomes `0`.
    pub fn position_at_angle(&self, angle: f64) -> usize {
        let normalized = ((angle % (2.0 * PI)) + 2.0 * PI) % (2.0 * PI);
        let pos = (normalized / (2.0 * PI) * self.length as f64).round() as usize;
        if pos == self.length {
            0
        } else {
            pos
        }
    }

    /// Position for a point (mouse click): angle = `atan2(x, -y)`, then as in `position_at_angle`.
    /// The center `(0, 0)` yields position 0.
    pub fn position_at_point(&self, x: f64, y: f64) -> usize {
        if x == 0.0 && y == 0.0 {
            return 0;
        }
        let angle = x.atan2(-y);
        self.position_at_angle(angle)
    }
}

/// Pushes label angles apart. `angles` is sorted ascending. Forward pass only:
/// `out[0] = angles[0]`, then `out[i] = max(angles[i], out[i-1] + min_gap)`. There is no wrap-around
/// at the end of the circle. Same length and order as the input.
pub fn spread_labels(angles: &[f64], min_gap: f64) -> Vec<f64> {
    if angles.is_empty() {
        return vec![];
    }
    let mut out = Vec::with_capacity(angles.len());
    out.push(angles[0]);
    for i in 1..angles.len() {
        out.push(angles[i].max(out[i - 1] + min_gap));
    }
    out
}

/// Linear map: position `0` is at `x = 0`, position `length` at `x = width`.
#[derive(Clone, Debug, PartialEq)]
pub struct LinearMapLayout {
    length: usize,
    width: f64,
}

impl LinearMapLayout {
    /// Error `Error::InvalidInput` if `length == 0` or `width` is not greater than 0.
    pub fn new(length: usize, width: f64) -> Result<Self> {
        if length == 0 || width <= 0.0 || width.is_nan() {
            return Err(Error::InvalidInput(
                "length must be > 0 and width must be > 0".into(),
            ));
        }
        Ok(LinearMapLayout { length, width })
    }

    /// `x = position / length · width` (unclamped).
    pub fn x_of(&self, position: f64) -> f64 {
        position / self.length as f64 * self.width
    }

    /// Inverse: `round(x / width · length)`, clamped to `0..=length`.
    pub fn position_at_x(&self, x: f64) -> usize {
        let pos = (x / self.width * self.length as f64).round() as usize;
        pos.min(self.length)
    }

    /// Width of a span `[start, end)` in x units: `(end − start) / length · width`; 0 if `end <= start`.
    pub fn width_of(&self, start: usize, end: usize) -> f64 {
        if end <= start {
            return 0.0;
        }
        (end - start) as f64 / self.length as f64 * self.width
    }

    /// Positions of the tick marks: multiples of `tick_step(length, max_ticks)` from 0 up to and including `length`.
    pub fn ticks(&self, max_ticks: usize) -> Vec<usize> {
        let step = tick_step(self.length, max_ticks);
        (0..=self.length).step_by(step).collect()
    }
}

/// "Nice" step size for tick marks: the smallest number of the form 1, 2 or 5 times 10^k (k ≥ 0) for which
/// `length / step <= max_ticks` holds. `max_ticks == 0` is treated as 1. At least 1.
pub fn tick_step(length: usize, max_ticks: usize) -> usize {
    for exp in 0.. {
        for &mult in &[1usize, 2usize, 5usize] {
            let step = mult.saturating_mul(10usize.saturating_pow(exp));
            if step == usize::MAX {
                return step;
            }
            let threshold = step.saturating_mul(usize::max(max_ticks, 1));
            if length <= threshold {
                return step;
            }
        }
    }
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn kreis_winkel() {
        let l = CircularMapLayout::new(1000).unwrap();
        assert_eq!(l.length(), 1000);
        assert!(close(l.angle_of(0.0), 0.0));
        assert!(close(l.angle_of(250.0), PI / 2.0));
        assert!(close(l.angle_of(500.0), PI));
        assert!(close(l.angle_of(1000.0), 0.0));
        assert!(close(l.angle_of(1250.0), PI / 2.0));
        assert!(matches!(
            CircularMapLayout::new(0),
            Err(Error::InvalidInput(_))
        ));
    }

    #[test]
    fn kreis_punkte() {
        let l = CircularMapLayout::new(1000).unwrap();
        let (x, y) = l.point_at(0.0, 10.0);
        assert!(close(x, 0.0) && close(y, -10.0));
        let (x, y) = l.point_at(250.0, 10.0);
        assert!(close(x, 10.0) && close(y, 0.0));
        let (x, y) = l.point_at(500.0, 10.0);
        assert!(close(x, 0.0) && close(y, 10.0));
        let (x, y) = l.point_at(750.0, 10.0);
        assert!(close(x, -10.0) && close(y, 0.0));
    }

    #[test]
    fn kreis_ueberstrichen() {
        let l = CircularMapLayout::new(1000).unwrap();
        assert!(close(l.sweep_of(0, 250), PI / 2.0));
        assert!(close(l.sweep_of(100, 100), 0.0));
        // across the origin: 900 to 100 is 200 bases
        assert!(close(l.sweep_of(900, 100), 2.0 * PI * 0.2));
    }

    #[test]
    fn kreis_rueckwaerts() {
        let l = CircularMapLayout::new(1000).unwrap();
        assert_eq!(l.position_at_angle(PI / 2.0), 250);
        assert_eq!(l.position_at_angle(0.0), 0);
        assert_eq!(l.position_at_angle(2.0 * PI), 0);
        assert_eq!(l.position_at_angle(-PI / 2.0), 750);
        assert_eq!(l.position_at_angle(2.0 * PI - 1e-9), 0);
        assert_eq!(l.position_at_point(10.0, 0.0), 250);
        assert_eq!(l.position_at_point(0.0, 10.0), 500);
        assert_eq!(l.position_at_point(-10.0, 0.0), 750);
        assert_eq!(l.position_at_point(0.0, -10.0), 0);
        assert_eq!(l.position_at_point(0.0, 0.0), 0);
    }

    #[test]
    fn beschriftungen_auseinander() {
        let out = spread_labels(&[0.0, 0.01, 0.02, 1.0], 0.1);
        assert_eq!(out.len(), 4);
        assert!(close(out[0], 0.0));
        assert!(close(out[1], 0.1));
        assert!(close(out[2], 0.2));
        assert!(close(out[3], 1.0));
        assert!(spread_labels(&[], 0.1).is_empty());
        assert_eq!(spread_labels(&[0.5], 0.1), vec![0.5]);
        // already far enough apart: unchanged
        assert_eq!(spread_labels(&[0.0, 1.0, 2.0], 0.5), vec![0.0, 1.0, 2.0]);
    }

    #[test]
    fn linear_abbildung() {
        let l = LinearMapLayout::new(1000, 500.0).unwrap();
        assert!(close(l.x_of(0.0), 0.0));
        assert!(close(l.x_of(500.0), 250.0));
        assert!(close(l.x_of(1000.0), 500.0));
        assert_eq!(l.position_at_x(250.0), 500);
        assert_eq!(l.position_at_x(-30.0), 0);
        assert_eq!(l.position_at_x(9999.0), 1000);
        assert!(close(l.width_of(100, 300), 100.0));
        assert!(close(l.width_of(300, 100), 0.0));
        assert!(LinearMapLayout::new(0, 10.0).is_err());
        assert!(LinearMapLayout::new(10, 0.0).is_err());
        assert!(LinearMapLayout::new(10, -1.0).is_err());
    }

    #[test]
    fn schrittweite() {
        assert_eq!(tick_step(100, 10), 10);
        assert_eq!(tick_step(95, 10), 10);
        assert_eq!(tick_step(101, 10), 20);
        assert_eq!(tick_step(2686, 10), 500);
        assert_eq!(tick_step(5, 10), 1);
        assert_eq!(tick_step(0, 10), 1);
        assert_eq!(tick_step(100, 0), 100);
        assert_eq!(tick_step(1_000_000, 8), 200_000);
    }

    #[test]
    fn skalenstriche() {
        let l = LinearMapLayout::new(2686, 800.0).unwrap();
        assert_eq!(l.ticks(10), vec![0, 500, 1000, 1500, 2000, 2500]);
        let k = LinearMapLayout::new(100, 800.0).unwrap();
        assert_eq!(
            k.ticks(10),
            vec![0, 10, 20, 30, 40, 50, 60, 70, 80, 90, 100]
        );
    }
}
