use std::fmt::{self, Debug};
use std::marker::PhantomData;
use std::ops::Range;
use std::str::FromStr;

use error_stack::{IntoReport, Report, Result, ResultExt};
use thiserror::Error;

use crate::file::beatmap::{SliderCurveType, SliderPoint};

#[macro_export]
/// Wraps the result in a given context, lazily.
macro_rules! ctx {
    ($result:expr, $ctx:expr) => {
        ($result).change_context_lazy(|| $ctx)
    };
}

#[macro_export]
/// Wraps the result in a report with a given context, lazily.
macro_rules! rctx {
    ($result:expr, $ctx:expr) => {
        ctx!(($result).report(), $ctx)
    };
}

#[macro_export]
/// Wraps the result in a `SectionParseError` context of a given section, lazily.
macro_rules! section_ctx {
    ($result:expr, $section:ident) => {
        ctx!(($result), SectionParseError::from(stringify!($section)))
    };
}

#[macro_export]
/// Wraps the result in a report with a `SectionParseError` context of a given section, lazily.
macro_rules! section_rctx {
    ($result:expr, $section:ident) => {
        section_ctx!(($result).report(), $section)
    };
}

#[macro_export]
/// Section context with printable attached for specific field value parsing error.
macro_rules! section_fvp_ctx {
    ($result:expr, $section:ident, $field:ident) => {
        section_ctx!(($result), $section).attach_printable_lazy(|| {
            format!("Could not parse value for {} field", stringify!($field))
        })
    };
}

#[macro_export]
/// Section context with printable attached for specific field value parsing error.
macro_rules! section_fvp_rctx {
    ($result:expr, $section:ident, $field:ident) => {
        section_fvp_ctx!(($result).report(), $section, $field)
    };
}

#[derive(Clone, Debug, Error)]
#[error("Invalid key-value pair (line: {line:?})")]
pub struct InvalidKeyValuePairError {
    pub line: String,
}

impl From<&str> for InvalidKeyValuePairError {
    fn from(line: &str) -> Self {
        Self {
            line: line.to_owned(),
        }
    }
}

#[derive(Clone, Debug, Error)]
#[error("Invalid list of {type_name} (line: {line:?})")]
pub struct InvalidListError<T> {
    type_name: &'static str,
    pub line: String,
    _phantom_data: PhantomData<T>,
}

impl<T> From<&str> for InvalidListError<T> {
    fn from(line: &str) -> Self {
        Self {
            type_name: std::any::type_name::<T>(),
            line: line.to_owned(),
            _phantom_data: PhantomData::<T>,
        }
    }
}

/// Parse a `field:value` pair (arbitrary spaces allowed).
pub(crate) fn parse_field_value_pair(
    line: &str,
) -> Result<(String, String), InvalidKeyValuePairError> {
    let (field, value) = line.split_once(':').ok_or_else(|| {
        Report::new(InvalidKeyValuePairError::from(line))
            .attach_printable("Could not split with ':'")
    })?;

    let field = field.trim().to_owned();
    let value = value.trim().to_owned();

    Ok((field, value))
}

pub(crate) fn parse_list_of_with_sep<T, E>(
    line: &str,
    sep: char,
) -> Result<Vec<T>, InvalidListError<T>>
where
    T: Debug + FromStr<Err = E> + Send + Sync + 'static,
    std::result::Result<T, E>: IntoReport<Ok = T, Err = E>,
{
    let mut tobjs = Vec::new();
    for value in line.split(sep) {
        if value.is_empty() {
            continue;
        }

        tobjs.push(
            value
                .parse::<T>()
                .report()
                .change_context_lazy(|| InvalidListError::from(line))?,
        );
    }

    Ok(tobjs)
}

pub(crate) fn parse_list_of<T, E>(line: &str) -> Result<Vec<T>, InvalidListError<T>>
where
    T: Debug + FromStr<Err = E> + Send + Sync + 'static,
    std::result::Result<T, E>: IntoReport<Ok = T, Err = E>,
{
    parse_list_of_with_sep(line, ',')
}

#[must_use]
pub(crate) fn to_standardized_path(path: &str) -> String {
    path.replace('\\', "/")
}

#[must_use]
pub(crate) fn is_close(a: f64, b: f64, tolerance: f64) -> bool {
    (a - b).abs() <= tolerance
}

#[must_use]
pub fn close_range(a: f64, tolerance: f64) -> Range<f64> {
    (a - tolerance)..(a + tolerance)
}

pub struct SliderPointsView<'a>(pub &'a [SliderPoint]);

impl<'a> fmt::Display for SliderPointsView<'a> {
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
