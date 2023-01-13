use std::num::{ParseFloatError, ParseIntError};

use miette::{Diagnostic, NamedSource, SourceOffset, SourceSpan};
use nom::error::{ContextError, ParseError};
use nom::Offset;
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
#[error("{kind}")]
pub struct BeatmapError {
    /// Source string for the beatmap file that failed to parse.
    #[source_code]
    pub input: NamedSource,

    /// Offset in chars of the error.
    #[label("{}", label.unwrap_or("here"))]
    pub span: SourceSpan,

    /// Label text for this span. Defaults to `"here"`.
    pub label: Option<&'static str>,

    /// Suggestion for fixing the parser error.
    #[help]
    pub help: Option<&'static str>,

    /// Specific error kind for this parser error.
    pub kind: BeatmapErrorKind,
}

impl BeatmapError {
    pub fn from_src_and_parse_error(
        name: impl AsRef<str>,
        source: &str,
        error: BeatmapParseError<&str>,
    ) -> Self {
        BeatmapError {
            input: NamedSource::new(name, source.to_owned()),
            span: SourceSpan::new(
                SourceOffset::from(source.offset(error.input)),
                SourceOffset::from(error.len),
            ),
            label: error.label,
            help: error.help,
            kind: if let Some(context) = error.context {
                BeatmapErrorKind::Context(context)
            } else {
                error.kind.unwrap_or(BeatmapErrorKind::Other)
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Error, Diagnostic)]
pub enum BeatmapErrorKind {
    #[error("Unknown section {0:?}")]
    #[diagnostic(code(osu::unknown_section))]
    UnknownSection(String),

    #[error("Unknown format version ({0})")]
    #[diagnostic(code(osu::unknown_format_version))]
    UnknownFormatVersion(u32),

    #[error("Unknown event {0:?}")]
    #[diagnostic(code(osu::unknown_event))]
    UnknownEvent(String),

    #[error("Unknown color field {0:?}")]
    #[diagnostic(code(osu::unknown_color_field))]
    UnknownColorField(String),

    #[error("Unknown slider curve type {0:?}")]
    #[diagnostic(code(osu::unknown_slider_curve_type))]
    UnknownSliderCurveType(String),

    #[error("Unknown hit object type ({0})")]
    #[diagnostic(code(osu::unknown_hit_object_type))]
    UnknownHitObjectType(u8),

    #[error("Invalid slider curve token")]
    #[diagnostic(code(osu::invalid_slider_curve_token))]
    InvalidSliderCurveToken,

    #[error("Invalid hit-sample set")]
    #[diagnostic(code(osu::invalid_hit_sample_set))]
    InvalidHitSampleSet,

    #[error("Invalid osu!mania hold")]
    #[diagnostic(code(osu::invalid_osu_mania_hold))]
    InvalidOsuManiaHold,

    #[error(transparent)]
    #[diagnostic(code(osu::invalid_overlay_position))]
    InvalidOverlayPosition(#[from] InvalidOverlayPositionError),

    #[error(transparent)]
    #[diagnostic(code(osu::parse_int))]
    ParseInt(#[from] ParseIntError),

    #[error(transparent)]
    #[diagnostic(code(osu::parse_float))]
    ParseFloat(#[from] ParseFloatError),

    #[error(transparent)]
    #[diagnostic(code(osu::parse_list))]
    ParseList(#[from] ParseListError),

    /// Generic parsing error. The given context string denotes the component
    /// that failed to parse.
    #[error("Expected {0}.")]
    #[diagnostic(code(osu::parse_component))]
    Context(&'static str),

    /// Generic unspecified error. If this is returned, the call site should
    /// be annotated with context, if possible.
    #[error("An unspecified error occurred")]
    #[diagnostic(code(osu::other))]
    Other,
}

#[derive(Clone, Debug, Eq, PartialEq, Error, Diagnostic)]
pub enum ParseListError {
    #[error("Expected at least {0} values, got {1}")]
    #[diagnostic(code(osu::parse_list::too_few_values))]
    TooFewValues(usize, usize),
    #[error("Expected at most {0} values, got {1}")]
    #[diagnostic(code(osu::parse_list::too_many_values))]
    TooManyValues(usize, usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeatmapParseError<I> {
    pub input: I,
    pub len: usize,
    pub context: Option<&'static str>,
    pub label: Option<&'static str>,
    pub help: Option<&'static str>,
    pub kind: Option<BeatmapErrorKind>,
    pub touched: bool,
}

impl<I> ParseError<I> for BeatmapParseError<I> {
    fn from_error_kind(input: I, _kind: nom::error::ErrorKind) -> Self {
        Self {
            input,
            len: 0,
            label: None,
            help: None,
            context: None,
            kind: None,
            touched: false,
        }
    }

    fn append(_input: I, _kind: nom::error::ErrorKind, other: Self) -> Self {
        other
    }
}

impl<I> ContextError<I> for BeatmapParseError<I> {
    fn add_context(_input: I, ctx: &'static str, mut other: Self) -> Self {
        other.context = other.context.or(Some(ctx));
        other
    }
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[error("Invalid overlay position: {op_string:?}. Expected NoChange, Below or Above")]
pub struct InvalidOverlayPositionError {
    pub op_string: String,
}

impl From<&str> for InvalidOverlayPositionError {
    fn from(op_str: &str) -> Self {
        Self {
            op_string: op_str.to_owned(),
        }
    }
}
