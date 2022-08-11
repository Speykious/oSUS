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
#[error("Invalid list of floats (line: {line:?})")]
pub struct InvalidFloatListError {
    pub line: String,
}

impl From<&str> for InvalidFloatListError {
    fn from(line: &str) -> Self {
        Self {
            line: line.to_owned(),
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

pub fn parse_floats(line: &str) -> Result<Vec<f32>, InvalidFloatListError> {
    let mut ints = Vec::new();
    for value in line.split(',') {
        if value.is_empty() {
            continue;
        }

        ints.push(
            value
                .parse::<f32>()
                .report()
                .change_context_lazy(|| InvalidFloatListError::from(line))?,
        );
    }

    Ok(ints)
}

pub fn to_standardized_path(path: &str) -> String {
    path.replace('\\', "/")
}
