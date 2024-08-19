//! adapted from <https://github.com/JPK314/LazerToStable/blob/main/src/bezier_converter.py>

use std::f64::consts::TAU;

use crate::file::beatmap::{SliderCurveType, SliderPoint};
use crate::is_close;
use crate::point::Point;

#[derive(Clone, Debug)]
pub struct CirclePreset<const N: usize> {
	/// Max angle in radians
	pub max_angle: f64,
	pub points: [Point; N],
}

pub struct CirclePresets {
	pub preset_3: CirclePreset<3>,
	pub preset_4: CirclePreset<4>,
	pub preset_5: CirclePreset<5>,
	pub preset_6: CirclePreset<6>,
	pub preset_7: CirclePreset<7>,
}

#[allow(clippy::unreadable_literal)]
pub const CIRCLE_PRESETS: CirclePresets = CirclePresets {
	preset_3: CirclePreset {
		max_angle: 0.4993379862754501,
		points: [
			Point::new(1.0, 0.0),
			Point::new(1.0, 0.2549893626632736),
			Point::new(0.8778997558480327, 0.47884446188920726),
		],
	},
	preset_4: CirclePreset {
		max_angle: 1.7579419829169447,
		points: [
			Point::new(1.0, 0.0),
			Point::new(1.0, 0.6263026),
			Point::new(0.42931178, 1.0990661),
			Point::new(-0.18605515, 0.9825393),
		],
	},
	preset_5: CirclePreset {
		max_angle: 3.1385246920140215,
		points: [
			Point::new(1.0, 0.0),
			Point::new(1.0, 0.87084764),
			Point::new(0.002304826, 1.5033062),
			Point::new(-0.9973236, 0.8739115),
			Point::new(-0.9999953, 0.0030679568),
		],
	},
	preset_6: CirclePreset {
		max_angle: 5.69720464620727,
		points: [
			Point::new(1.0, 0.0),
			Point::new(1.0, 1.4137783),
			Point::new(-1.4305235, 2.0779421),
			Point::new(-2.3410065, -0.94017583),
			Point::new(0.05132711, -1.7309346),
			Point::new(0.8331702, -0.5530167),
		],
	},
	preset_7: CirclePreset {
		max_angle: TAU,
		points: [
			Point::new(1.0, 0.0),
			Point::new(1.0, 1.2447058),
			Point::new(-0.8526471, 2.118367),
			Point::new(-2.6211002, 7.854936_e-06),
			Point::new(-0.8526448, -2.118357),
			Point::new(1.0, -1.2447058),
			Point::new(1.0, 0.0),
		],
	},
};

#[derive(Clone, Debug, thiserror::Error)]
#[allow(clippy::module_name_repetitions)]
pub enum BezierConversionError {
	#[error("There are no control points to convert")]
	NoControlPoints,
	#[error("Perfect curve has more than 3 points")]
	PerfectCurveWithMoreThan3Points,
}

/// Converts a slider's control points to bezier anchors.
///
/// # Errors
///
/// This function will return an error if there are no control points
/// or if the control points do not represent a valid slider segment.
pub fn convert_to_bezier_anchors(control_points: &[SliderPoint]) -> Result<Vec<Point>, BezierConversionError> {
	if control_points.is_empty() {
		return Err(BezierConversionError::NoControlPoints);
	}

	Ok(match control_points[0].curve_type {
		SliderCurveType::Linear => convert_linear_to_bezier_anchors(control_points),
		SliderCurveType::PerfectCurve => {
			if control_points.len() == 2 {
				convert_linear_to_bezier_anchors(control_points)
			} else if let Ok(control_points) = control_points.try_into() {
				convert_circle_to_bezier_anchors(control_points)
			} else {
				return Err(BezierConversionError::PerfectCurveWithMoreThan3Points);
			}
		}
		SliderCurveType::Catmull => convert_catmull_to_bezier_anchors(control_points),
		_ => control_points.iter().map(SliderPoint::to_point).collect(),
	})
}

#[derive(Clone, Debug, Default)]
pub struct CircleArcProperties {
	pub theta_start: f64,
	pub theta_range: f64,
	pub direction: f64,
	pub radius: f64,
	pub center: Point,
}

#[must_use]
fn get_circle_arc_properties(control_points: &[SliderPoint; 3]) -> Option<CircleArcProperties> {
	let a = control_points[0].to_point();
	let b = control_points[1].to_point();
	let c = control_points[2].to_point();

	if is_close(
		0.0,
		(b.y - a.y).mul_add(c.x - a.x, -(b.x - a.x) * (c.y - a.y)),
		f64::EPSILON,
	) {
		return None;
	}

	let d = 2.0 * c.x.mul_add((a - b).y, a.x.mul_add((b - c).y, b.x * (c - a).y));

	let a_sq = a.dot(a);
	let b_sq = b.dot(b);
	let c_sq = c.dot(c);

	let center = Point {
		x: c_sq.mul_add((a - b).y, a_sq.mul_add((b - c).y, b_sq * (c - a).y)),
		y: c_sq.mul_add((b - a).x, a_sq.mul_add((c - b).x, b_sq * (a - c).x)),
	};
	let center = center / d;

	let da = a - center;
	let dc = c - center;

	let radius = da.len();

	let theta_start = da.y.atan2(da.x);
	let theta_end = {
		let theta_end = dc.y.atan2(dc.x);
		// turn as many times as necessary so that theta_end >= theta_start
		TAU.mul_add(((theta_start - theta_end) / TAU).ceil(), theta_end)
	};

	let mut theta_range = theta_end - theta_start;
	let mut direction = 1.0;

	// Decide in which direction to draw the circle, depending on which side of AC B lies.
	let ortho_a_to_c = c - a;
	let ortho_a_to_c = Point {
		x: ortho_a_to_c.y,
		y: -ortho_a_to_c.x,
	};

	if ortho_a_to_c.dot(b - a) < 0.0 {
		direction = -direction;
		theta_range = TAU - theta_range;
	}

	Some(CircleArcProperties {
		theta_start,
		theta_range,
		direction,
		radius,
		center,
	})
}

fn convert_circle_to_bezier_anchors(points: &[SliderPoint; 3]) -> Vec<Point> {
	let Some(cs) = get_circle_arc_properties(points) else {
		return points.iter().map(SliderPoint::to_point).collect();
	};

	let mut arc;
	let mut arc_len;

	if CIRCLE_PRESETS.preset_3.max_angle >= cs.theta_range {
		let preset = CIRCLE_PRESETS.preset_3;
		arc = preset.points.to_vec();
		arc_len = preset.max_angle;
	} else if CIRCLE_PRESETS.preset_4.max_angle >= cs.theta_range {
		let preset = CIRCLE_PRESETS.preset_4;
		arc = preset.points.to_vec();
		arc_len = preset.max_angle;
	} else if CIRCLE_PRESETS.preset_5.max_angle >= cs.theta_range {
		let preset = CIRCLE_PRESETS.preset_5;
		arc = preset.points.to_vec();
		arc_len = preset.max_angle;
	} else if CIRCLE_PRESETS.preset_6.max_angle >= cs.theta_range {
		let preset = CIRCLE_PRESETS.preset_6;
		arc = preset.points.to_vec();
		arc_len = preset.max_angle;
	} else {
		let preset = CIRCLE_PRESETS.preset_7;
		arc = preset.points.to_vec();
		arc_len = preset.max_angle;
	}

	// converge on arc length of theta range
	let n = arc.len() - 1;
	let mut tf = cs.theta_range / arc_len;

	#[allow(clippy::while_float)]
	while (tf - 1.0).abs() > 0.000_001 {
		for j in 0..n {
			for i in ((j + 1)..=n).rev() {
				arc[i] = arc[i] * tf + arc[i - 1] * (1.0 - tf);
			}
		}

		let last_point = arc.last().unwrap();
		arc_len = last_point.y.atan2(last_point.x);
		if arc_len < 0.0 {
			arc_len += TAU;
		}
		tf = cs.theta_range / arc_len;
	}

	// adjust rotation, radius and position
	let rot_a = Point {
		x: cs.theta_start.cos(),
		y: -cs.theta_start.sin() * cs.direction,
	} * cs.radius;

	let rot_b = Point {
		x: cs.theta_start.sin(),
		y: cs.theta_start.cos() * cs.direction,
	} * cs.radius;

	for point in &mut arc {
		*point = Point {
			x: rot_a.dot(*point),
			y: rot_b.dot(*point),
		} + cs.center;
	}

	*arc.first_mut().unwrap() = points[0].to_point();
	*arc.last_mut().unwrap() = points[2].to_point();

	arc
}

#[must_use]
fn convert_catmull_to_bezier_anchors(points: &[SliderPoint]) -> Vec<Point> {
	let [first_point, points @ ..] = points else {
		return points.iter().map(SliderPoint::to_point).collect();
	};

	let mut cubics = vec![first_point.to_point()];
	for i in 0..(points.len() - 1) {
		let v1 = points[if i > 0 { i - 1 } else { i }].to_point();
		let v2 = points[i].to_point();
		let v3 = if i < points.len() - 1 {
			points[i + 1].to_point()
		} else {
			v2 + v2 - v1
		};
		let v4 = if i < points.len() - 2 {
			points[i + 2].to_point()
		} else {
			v3 + v3 - v2
		};

		cubics.push((-v1 + v2 * 6.0 + v3) / 6.0);
		cubics.push((-v4 + v3 * 6.0 + v2) / 6.0);
		cubics.push(v3);
		cubics.push(v3);
	}
	cubics.remove(cubics.len() - 1);

	cubics
}

#[must_use]
fn convert_linear_to_bezier_anchors(points: &[SliderPoint]) -> Vec<Point> {
	let Some(first_point) = points.first() else {
		return Vec::new();
	};

	let mut bezier = vec![first_point.to_point()];
	for &point in points {
		bezier.push(point.to_point());
		bezier.push(point.to_point());
	}
	bezier.remove(bezier.len() - 1);

	bezier
}
