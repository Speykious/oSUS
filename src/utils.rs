use std::fmt::Debug;
use std::marker::PhantomData;
use std::str::FromStr;

use error_stack::{IntoReport, Report, Result, ResultExt};
use thiserror::Error;

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
pub fn parse_field_value_pair(line: &str) -> Result<(String, String), InvalidKeyValuePairError> {
    let (field, value) = line.split_once(':').ok_or_else(|| {
        Report::new(InvalidKeyValuePairError::from(line))
            .attach_printable("Could not split with ':'")
    })?;

    let field = field.trim().to_owned();
    let value = value.trim().to_owned();

    Ok((field, value))
}

pub fn parse_list_of<T, E>(line: &str) -> Result<Vec<T>, InvalidListError<T>>
where
    T: Debug + FromStr<Err = E> + Send + Sync + 'static,
    std::result::Result<T, E>: IntoReport<Ok = T, Err = E>,
{
    let mut tobjs = Vec::new();
    for value in line.split(',') {
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

pub fn to_standardized_path(path: &str) -> String {
    path.replace('\\', "/")
}
