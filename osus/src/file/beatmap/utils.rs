use std::fmt;

use crate::file::beatmap::{SliderCurveType, SliderPoint};

pub struct SliderPointsView<'a>(pub &'a [SliderPoint]);

impl fmt::Display for SliderPointsView<'_> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		if let [first_curve_point, ..] = self.0 {
			let first_curve_type = first_curve_point.curve_type;

			let mut started = false;
			for &curve_point in self.0 {
				if started {
					write!(f, "|")?;
				}

				let SliderPoint { curve_type, x, y } = curve_point;
				let prefix = match curve_type {
					SliderCurveType::Inherit => "",
					SliderCurveType::Bezier => "B|",
					SliderCurveType::Catmull => "C|",
					SliderCurveType::Linear => "L|",
					SliderCurveType::PerfectCurve => "P|",
				};

				if !started && curve_type != first_curve_type {
					let preprefix = match first_curve_type {
						SliderCurveType::Inherit => "",
						SliderCurveType::Bezier => "B|",
						SliderCurveType::Catmull => "C|",
						SliderCurveType::Linear => "L|",
						SliderCurveType::PerfectCurve => "P|",
					};
					write!(f, "{preprefix}")?;
				}

				write!(f, "{prefix}{x}:{y}")?;
				started = true;
			}

			Ok(())
		} else {
			write!(f, "(empty)")
		}
	}
}
