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
    pub fn from_source_and_parse_error(
        name: impl AsRef<str>,
        source: &'static str,
        error: BeatmapParseError<&str>,
    ) -> Self {
        BeatmapError {
            input: NamedSource::new(name, source),
            span: SourceSpan::new(
                SourceOffset::from(source.offset(error.err_span)),
                SourceOffset::from(error.err_span.len()),
            ),
            label: error.label,
            help: error.help,
            kind: error.kind.unwrap_or(BeatmapErrorKind::Other),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Error, Diagnostic)]
pub enum BeatmapErrorKind {
    #[error("Unknown section {0:?}")]
    #[diagnostic(code(osu::unknown_section))]
    UnknownSection(String),

    /// Generic unspecified error. If this is returned, the call site should
    /// be annotated with context, if possible.
    #[error("An unspecified error occurred")]
    #[diagnostic(code(osu::other))]
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeatmapParseError<I> {
    pub err_span: I,
    pub context: Option<&'static str>,
    pub label: Option<&'static str>,
    pub help: Option<&'static str>,
    pub kind: Option<BeatmapErrorKind>,
}

impl<I> ParseError<I> for BeatmapParseError<I> {
    fn from_error_kind(input: I, _kind: nom::error::ErrorKind) -> Self {
        Self {
            err_span: input,
            label: None,
            help: None,
            context: None,
            kind: None,
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
