use error_stack::{Report, Result};
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
/// Wraps the result in a report with a `SectionParseError` context of a given section, lazily.
macro_rules! section_ctx {
    ($result:expr, $section:ident) => {
        ctx!(($result).report(), SectionParseError::from(stringify!($section)))
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

/// Parse a `field:value` pair (arbitrary spaces allowed).
pub fn parse_field_value_pair(line: &str) -> Result<(String, String), InvalidKeyValuePairError> {
    let (field, value) = line.split_once(':').ok_or(
        Report::new(InvalidKeyValuePairError::from(line))
            .attach_printable("Could not split with ':'"),
    )?;

    let field = field.trim().to_owned();
    let value = value.trim().to_owned();

    Ok((field, value))
}
